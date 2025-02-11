use std::collections::HashMap;
use std::mem;
use std::ops::BitOr;

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
            ],
            should_flush: false,
        }
    }

    pub fn run_program(&mut self, instructions: Vec<Word>) {
        self.fetcher = Fetcher::new(instructions);
        self.run();
    }

    fn run(&mut self) {
        for i in 0..12 {
            println!("CYCLE {}", i);
            self.cycle();
            println!("");
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
            );
        }

        if self.should_flush {
            for eu in self.execution_units.iter_mut() {
                eu.instruction = None;
                eu.cycles_left = 0;
            }
            self.dispatcher.staged_instruction = None;
            self.fetcher.fetched = Free;
            self.should_flush = false;
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
    const ALU_OPS: [Op; 6] = [
        Op::LoadImmediate,
        Op::Add,
        Op::AddImmediate,
        Op::Compare,
        Op::Subtract,
        Op::SubtractImmediate,
    ];
    const BRANCH_OPS: [Op; 6] = [
        Op::BranchEqual,
        Op::BranchNotEqual,
        Op::BranchGreater,
        Op::BranchGreaterEqual,
        Op::BranchLess,
        Op::BranchLessEqual,
    ];
    const MEMORY_OPS: [Op; 0] = [];

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
                EUType::ALU => Self::ALU_OPS.contains(&inst.word.op()),
                EUType::Branch => Self::BRANCH_OPS.contains(&inst.word.op()),
                EUType::Memory => Self::MEMORY_OPS.contains(&inst.word.op()),
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
    ) {
        if self.cycles_left > 1 {
            self.cycles_left -= 1;
        } else if self.cycles_left == 1 {
            self.execute(registers, should_flush, branch_predictor);
            self.cycles_left -= 1;
        }
    }

    fn execute(
        &mut self,
        registers: &mut Registers,
        should_flush: &mut bool,
        branch_predictor: &mut BranchPredictor,
    ) {
        if let Some(instruction) = self.instruction {
            println!("execute {:?}", instruction.word.op());

            match self.flavour {
                EUType::ALU => self.alu(instruction, registers),
                EUType::Branch => {
                    self.branch(instruction, registers, should_flush, branch_predictor)
                }
                EUType::Memory => (),
            };
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

        let (op, right, left, immediate) = match word {
            Word::I(op, rl, rr, immediate) => (op, registers.get(rl), registers.get(rr), immediate),
            _ => panic!("uh oh"),
        };

        if match op {
            Op::BranchEqual => right == left,
            Op::BranchNotEqual => right != left,
            Op::BranchGreater => right > left,
            Op::BranchGreaterEqual => right >= left,
            Op::BranchLess => right < left,
            Op::BranchLessEqual => right <= left,
            _ => panic!("op code isnt branch bruh!"),
        } {
            if !instruction.branch_taken {
                let branch = (instruction.pc as i32) + immediate;
                registers.set(Register::ProgramCounter, branch);
                registers.set_available(Register::ProgramCounter);
                *should_flush = true;
            }
            branch_predictor.update(instruction.pc, true);
        } else {
            if instruction.branch_taken {
                registers.set(Register::ProgramCounter, (instruction.pc as i32) + 1);
                registers.set_available(Register::ProgramCounter);
                *should_flush = true;
            }
            branch_predictor.update(instruction.pc, false);
        }

        self.output = Free;
    }

    fn alu(&mut self, instruction: Instruction, registers: &mut Registers) {
        let word = instruction.word;

        let (op, ro, left, right) = match word {
            Word::R(op, ro, rl, rr) => (op, ro, registers.get(rl), registers.get(rr)),
            Word::I(op, ro, rl, right) => (op, ro, registers.get(rl), right),
        };

        let out = match op {
            Op::LoadImmediate => left + right,
            Op::Add | Op::AddImmediate => left + right,
            Op::Subtract | Op::SubtractImmediate => left - right,
            Op::Compare => (left - right).signum(),
            _ => panic!("cant alu this!"),
        };

        registers.set(ro, out);
        registers.set_available(ro);

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
    state_machines: Vec<BranchState>,
    state_map: HashMap<usize, usize>, // pc -> state machine
}
impl BranchPredictor {
    pub fn new() -> Self {
        BranchPredictor {
            state_machines: Vec::new(),
            state_map: HashMap::new(),
        }
    }

    pub fn predict(&mut self, pc: usize) -> bool {
        self.state_map
            .get(&pc)
            .and_then(|i| self.state_machines.get(*i))
            .map_or_else(
                || {
                    // first prediction assumes we take because loops!
                    &BranchState::WeakTaken
                },
                |s| s,
            )
            .predict()
    }

    pub fn update(&mut self, pc: usize, taken: bool) {
        let state = self
            .state_map
            .get(&pc)
            .and_then(|i| self.state_machines.get_mut(*i));

        match (state, taken) {
            (Some(state), true) => state.update_taken(),
            (Some(state), false) => state.update_not_taken(),
            (None, true) => {
                self.state_machines.push(BranchState::WeakTaken);
                self.state_map.insert(pc, self.state_machines.len());
            }
            (None, false) => {
                self.state_machines.push(BranchState::WeakNotTaken);
                self.state_map.insert(pc, self.state_machines.len());
            }
        };
    }
}
