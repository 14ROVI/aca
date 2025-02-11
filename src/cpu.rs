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
    fetcher: Fetcher,
    dispatcher: Dispatcher,
    execution_units: Vec<ExecutionUnit>,
    should_flush: bool,
}
impl CPU {
    pub fn new() -> Self {
        CPU {
            registers: Registers::new(),
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
        for i in 0..10 {
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
            eu.cycle(&mut self.registers, &mut self.should_flush);
        }

        if self.should_flush {
            for eu in self.execution_units.iter_mut() {
                eu.instruction = None;
                eu.cycles_left = 0;
            }
            self.dispatcher.staged_instruction = None;
            self.fetcher.fetched = Free;
            self.should_flush = false;
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
            self.fetcher.fetch(&mut self.registers);
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

    pub fn fetch(&mut self, registers: &mut Registers) {
        let pc = registers.pc();

        if pc >= self.instructions.len() {
            return;
        }

        let word = self.instructions[pc];
        let instruction = Instruction::new(word, pc);
        self.fetched = Done(instruction);

        // essentially this is dont take branches prediction ;)
        registers.inc_pc();
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
    const BRANCH_OPS: [Op; 2] = [Op::JumpAbsolute, Op::JumpRelative];
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

    fn cycle(&mut self, registers: &mut Registers, should_flush: &mut bool) {
        if self.cycles_left > 1 {
            self.cycles_left -= 1;
        } else if self.cycles_left == 1 {
            self.execute(registers, should_flush);
            self.cycles_left -= 1;
        }
    }

    fn execute(&mut self, registers: &mut Registers, should_flush: &mut bool) {
        if let Some(instruction) = self.instruction {
            println!("{:?}", instruction.word.op());
            match self.flavour {
                EUType::ALU => self.alu(instruction, registers),
                EUType::Branch => self.branch(instruction, registers, should_flush),
                EUType::Memory => (),
            };
        }
    }

    fn branch(
        &mut self,
        instruction: Instruction,
        registers: &mut Registers,
        should_flush: &mut bool,
    ) {
        let word = instruction.word;

        let (op, ro, left, right) = match word {
            Word::I(op, ro, rl, right) => (op, ro, registers.get(rl), right),
            _ => panic!("uh oh"),
        };

        let out = match op {
            Op::JumpAbsolute => left + right,
            Op::JumpRelative => (instruction.pc as i32) + left + right,
            _ => panic!("op code isnt branch bruh!"),
        };

        registers.set(ro, out);
        registers.set_available(ro);

        self.output = Free;
        *should_flush = true;
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
