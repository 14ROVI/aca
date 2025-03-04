use std::collections::HashMap;
use std::mem;

use bytes::BufMut;

use crate::instructions::Instruction;
use crate::instructions::{Op, Register, Word};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Output<T> {
    Done(T),
    Processing,
    Free,
}
impl<T> Output<T> {
    pub fn take(&mut self) -> Option<T> {
        match self {
            Done(_) => {
                let val = mem::replace(self, Free);
                match val {
                    Done(v) => Some(v),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}
use bytes::{Buf, BytesMut};
use Output::Done;
use Output::Free;
use Output::Processing;

#[derive(Debug, Clone)]
struct Registers {
    registers: HashMap<Register, i32>,
    availability: HashMap<Register, bool>,
}
impl Registers {
    fn new() -> Self {
        let mut registers = HashMap::new();
        let mut availability = HashMap::new();

        registers.insert(Register::ProgramCounter, 0);
        availability.insert(Register::ProgramCounter, true);

        for i in 0..32 {
            registers.insert(Register::General(i), 0);
            availability.insert(Register::General(i), true);
        }

        Registers {
            registers,
            availability,
        }
    }

    pub fn pc(&self) -> usize {
        self.registers[&Register::ProgramCounter] as usize
    }

    pub fn inc_pc(&mut self) {
        self.registers.insert(
            Register::ProgramCounter,
            self.registers[&Register::ProgramCounter] + 1,
        );
    }

    pub fn get(&self, reg: Register) -> i32 {
        if !self.is_available(reg) {
            panic!("Attempted to read register {:?} before it was ready!", reg);
        }

        self.registers[&reg]
    }

    pub fn get_general(&self, reg: u32) -> i32 {
        let register = Register::General(reg);
        if !self.is_available(register) {
            panic!("Attempted to read register {:?} before it was ready!", reg);
        }

        self.registers[&register]
    }

    pub fn set(&mut self, reg: Register, val: i32) {
        if reg != Register::General(0) {
            self.registers.insert(reg, val);
        }
    }

    pub fn is_available(&self, reg: Register) -> bool {
        self.availability[&reg]
    }

    pub fn set_available(&mut self, reg: Register) {
        self.availability.insert(reg, true);
    }

    pub fn set_unavailable(&mut self, reg: Register) {
        self.availability.insert(reg, false);
    }
}

pub struct CPU {
    registers: Registers,
    branch_predictor: BranchPredictor,
    fetcher: Fetcher,
    dispatcher: Dispatcher,
    execution_units: Vec<ExecutionUnit>,
    should_flush: bool,
    memory: BytesMut,
}
impl CPU {
    pub fn new() -> Self {
        CPU {
            registers: Registers::new(),
            branch_predictor: BranchPredictor::new(),
            fetcher: Fetcher::new(Vec::new()),
            dispatcher: Dispatcher::new(),
            execution_units: vec![
                ExecutionUnit::new(EUType::ALU),
                ExecutionUnit::new(EUType::Branch),
                ExecutionUnit::new(EUType::Memory),
            ],
            should_flush: false,
            memory: BytesMut::new(),
        }
    }

    pub fn set_memory(&mut self, memory: BytesMut) {
        self.memory = memory;
    }

    pub fn run_program(&mut self, instructions: Vec<Word>) {
        self.fetcher = Fetcher::new(instructions);
        self.run();
    }

    fn run(&mut self) {
        let mut i = 0;
        while !self.is_finished() || i == 0 || self.should_flush {
            println!("CYCLE {}", i);
            self.should_flush = false;
            self.cycle();
            println!("");
            i += 1;
        }

        let mut regs = self
            .registers
            .registers
            .iter()
            .filter(|(r, v)| **v != 0)
            .collect::<Vec<(&Register, &i32)>>();
        regs.sort();
        for (reg, value) in regs {
            println!("{:?}: {}", reg, value);
        }

        println!("{:?}", self.memory.to_vec());
    }

    fn cycle(&mut self) {
        // we have to run the cycle thing in reverse
        // because each stage pulls from one one infront
        // so we don't get instructions flying through in one cycle
        //
        // things are usually free if their output is None or processing

        // execute
        for eu in self.execution_units.iter_mut() {
            eu.cycle(
                &mut self.registers,
                &mut self.should_flush,
                &mut self.branch_predictor,
                &mut self.memory,
            );
        }

        if self.should_flush {
            for eu in self.execution_units.iter_mut() {
                eu.instruction = None;
                eu.cycles_left = 0;
            }
            self.dispatcher.staged_instruction = None;
            self.fetcher.fetched = Free;
            return;
        }

        // dispatch ("decodes" too)
        if self.dispatcher.is_free() {
            if let Some(instruction) = self.fetcher.take_fetched() {
                self.dispatcher.dispatch(instruction);
            }
        }
        self.dispatcher.cycle(&mut self.execution_units);

        // fetch
        if self.fetcher.is_free() {
            self.fetcher
                .fetch(&mut self.registers, &mut self.branch_predictor);
        }
    }

    fn is_finished(&self) -> bool {
        let mut finished = true;

        for eu in self.execution_units.iter() {
            finished &= eu.instruction == None;
        }
        finished &= self.dispatcher.staged_instruction == None;
        finished &= self.fetcher.fetched == Free;

        return finished;
    }
}

#[derive(Debug, Clone)]
struct Fetcher {
    instructions: Vec<Word>,
    fetched: Output<Instruction>,
}
impl Fetcher {
    pub fn new(instructions: Vec<Word>) -> Self {
        Fetcher {
            instructions,
            fetched: Free,
        }
    }

    pub fn is_free(&self) -> bool {
        self.fetched == Free
    }

    pub fn fetch(&mut self, registers: &mut Registers, branch_predictor: &mut BranchPredictor) {
        let pc = registers.pc();

        if pc >= self.instructions.len() {
            return;
        }

        let word = self.instructions[pc];
        let mut branch_taken = false;

        if word.op().is_predictable_branch() && branch_predictor.predict(pc) {
            // branch prediction
            if let Word::I(_, _, _, immediate) = word {
                let branch = (pc as i32) + immediate;
                println!("predicted branch to {}", branch);
                registers.set(Register::ProgramCounter, branch);
                branch_taken = true;
            }
        } else if let Word::JI(Op::Jump, val) = word {
            println!("jumped to {}", val);
            registers.set(Register::ProgramCounter, val);
            branch_taken = true;
        } else {
            // normal incrememnt
            registers.inc_pc();
        }

        let instruction = Instruction::new(word, pc, branch_taken);
        self.fetched = Done(instruction);

        // println!("fetch {:?}", word.op());
    }

    pub fn take_fetched(&mut self) -> Option<Instruction> {
        self.fetched.take()
    }
}

#[derive(Debug, Clone)]
struct Dispatcher {
    staged_instruction: Option<Instruction>,
}
impl Dispatcher {
    pub fn new() -> Self {
        Dispatcher {
            staged_instruction: None,
        }
    }

    pub fn is_free(&mut self) -> bool {
        self.staged_instruction.is_none()
    }

    pub fn dispatch(&mut self, instruction: Instruction) {
        self.staged_instruction = Some(instruction);
    }

    pub fn cycle(&mut self, execution_units: &mut Vec<ExecutionUnit>) {
        if self.staged_instruction.is_some() {
            for eu in execution_units {
                if eu.try_dispatch(&mut self.staged_instruction) {
                    break;
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum EUType {
    ALU,
    Branch,
    Memory,
}

#[derive(Debug, Clone)]
struct ExecutionUnit {
    flavour: EUType,
    instruction: Option<Instruction>,
    output: Output<i32>,
    cycles_left: usize,
}
impl ExecutionUnit {
    fn new(flavour: EUType) -> Self {
        ExecutionUnit {
            flavour,
            instruction: None,
            output: Free,
            cycles_left: 0,
        }
    }

    pub fn is_free(&mut self) -> bool {
        self.output == Free
    }

    pub fn try_dispatch(&mut self, instruction: &mut Option<Instruction>) -> bool {
        if let Some(inst) = instruction {
            let can_take = match self.flavour {
                EUType::ALU => inst.word.op().is_alu(),
                EUType::Branch => inst.word.op().is_branch(),
                EUType::Memory => inst.word.op().is_memory(),
            };

            if self.is_free() && can_take {
                self.instruction = instruction.take();
                self.output = Processing;
                self.cycles_left = 1;
                return true;
            }
        }

        false
    }

    fn cycle(
        &mut self,
        registers: &mut Registers,
        should_flush: &mut bool,
        branch_predictor: &mut BranchPredictor,
        memory: &mut BytesMut,
    ) {
        if self.cycles_left > 1 {
            self.cycles_left -= 1;
        } else if self.cycles_left == 1 {
            // execute
            if let Some(instruction) = self.instruction {
                println!("execute {:?}", instruction.word.op());

                match self.flavour {
                    EUType::ALU => self.alu(instruction, registers),
                    EUType::Branch => {
                        self.branch(instruction, registers, should_flush, branch_predictor)
                    }
                    EUType::Memory => self.load_store(instruction, registers, memory),
                };
            }

            self.cycles_left -= 1;
        } else {
            self.instruction = None;
            self.output = Free;
        }
    }

    fn branch(
        &mut self,
        instruction: Instruction,
        registers: &mut Registers,
        should_flush: &mut bool,
        branch_predictor: &mut BranchPredictor,
    ) {
        let word = instruction.word;

        if let Word::I(op, rl, rr, immediate) = word {
            let left = registers.get(rl);
            let right = registers.get(rr);

            if match op {
                Op::BranchEqual => left == right,
                Op::BranchNotEqual => left != right,
                Op::BranchGreater => left > right,
                Op::BranchGreaterEqual => left >= right,
                Op::BranchLess => left < right,
                Op::BranchLessEqual => left <= right,
                _ => panic!("op code isnt branch bruh!"),
            } {
                if !instruction.branch_taken {
                    let branch = (instruction.pc as i32) + immediate;
                    registers.set(Register::ProgramCounter, branch);
                    registers.set_available(Register::ProgramCounter);
                    *should_flush = true;
                }

                if op.is_predictable_branch() {
                    branch_predictor.update(instruction.pc, true);
                }
            } else {
                if instruction.branch_taken {
                    registers.set(Register::ProgramCounter, (instruction.pc as i32) + 1);
                    registers.set_available(Register::ProgramCounter);
                    *should_flush = true;
                }

                if op.is_predictable_branch() {
                    branch_predictor.update(instruction.pc, false);
                }
            }
        } else if let Word::JI(Op::Jump, immediate) = word {
            // we already jumped in fetch no need to do anything :D
            // registers.set(Register::ProgramCounter, immediate);
            // registers.set_available(Register::ProgramCounter);
        } else if let Word::JR(Op::JumpRegister, reg) = word {
            let val = registers.get(reg);
            registers.set(Register::ProgramCounter, val);
            registers.set_available(Register::ProgramCounter);
            *should_flush = true;
        } else {
            panic!("branch op not found!");
        }

        self.output = Free;
    }

    fn alu(&mut self, instruction: Instruction, registers: &mut Registers) {
        let word = instruction.word;

        let (op, ro, left, right) = match word {
            Word::R(op, ro, rl, rr) => (op, ro, registers.get(rl), registers.get(rr)),
            Word::I(op, ro, rl, right) => (op, ro, registers.get(rl), right),
            _ => panic!("not implimented for alu!"),
        };

        let out = match op {
            Op::Add | Op::AddImmediate => left + right,
            Op::Subtract | Op::SubtractImmediate => left - right,
            Op::Compare => (left - right).signum(),
            Op::Multiply => (left * right) as i32,
            Op::LeftShift => left << right,
            Op::RightShift => left >> right,
            _ => panic!("cant alu this!"),
        };

        registers.set(ro, out);
        registers.set_available(ro);

        self.output = Free;
    }

    fn load_store(
        &mut self,
        instruction: Instruction,
        registers: &mut Registers,
        memory: &mut BytesMut,
    ) {
        let word = instruction.word;

        if let Word::I(Op::LoadImmediate, ro, ri, immediate) = word {
            let out = registers.get(ri) + immediate;
            registers.set(ro, out);
            registers.set_available(ro);
        } else if let Word::I(Op::LoadMemory, ro, addr_reg, offset) = word {
            let addr = registers.get(addr_reg) + offset;
            let out = (&memory[addr as usize..(addr + 4) as usize]).get_i32();
            registers.set(ro, out);
            registers.set_available(ro);
        } else if let Word::I(Op::StoreMemory, ri, addr_reg, offset) = word {
            let addr = registers.get(addr_reg) + offset;
            (&mut memory[addr as usize..(addr + 4) as usize]).put_i32(registers.get(ri));
            println!("{} storing {}", addr, registers.get(ri));
        } else {
            panic!("WHAT");
        }

        self.output = Free;
    }
}

enum BranchState {
    StrongNotTaken,
    WeakNotTaken,
    WeakTaken,
    StrongTaken,
}
impl BranchState {
    pub fn update_taken(&mut self) {
        *self = match self {
            Self::StrongNotTaken => Self::WeakNotTaken,
            Self::WeakNotTaken => Self::WeakTaken,
            Self::WeakTaken => Self::StrongTaken,
            Self::StrongTaken => Self::StrongTaken,
        };
    }

    pub fn update_not_taken(&mut self) {
        *self = match self {
            Self::StrongNotTaken => Self::StrongNotTaken,
            Self::WeakNotTaken => Self::StrongNotTaken,
            Self::WeakTaken => Self::WeakNotTaken,
            Self::StrongTaken => Self::WeakTaken,
        };
    }

    pub fn predict(&self) -> bool {
        match self {
            Self::StrongNotTaken | Self::WeakNotTaken => false,
            Self::StrongTaken | Self::WeakTaken => true,
        }
    }
}

struct BranchPredictor {
    state_machines: HashMap<usize, BranchState>, // pc -> state machine
}
impl BranchPredictor {
    pub fn new() -> Self {
        BranchPredictor {
            state_machines: HashMap::new(),
        }
    }

    pub fn predict(&mut self, pc: usize) -> bool {
        self.state_machines.get(&pc).map_or(
            // first prediction assumes we take because of loops!
            true,
            |s| s.predict(),
        )
    }

    pub fn update(&mut self, pc: usize, taken: bool) {
        let state = self.state_machines.get_mut(&pc);

        match (state, taken) {
            (Some(state), true) => state.update_taken(),
            (Some(state), false) => state.update_not_taken(),
            (None, true) => {
                self.state_machines.insert(pc, BranchState::WeakTaken);
            }
            (None, false) => {
                self.state_machines.insert(pc, BranchState::WeakNotTaken);
            }
        };
    }
}
