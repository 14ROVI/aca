use std::{collections::HashMap, convert::identity, hash::Hash};

use crate::instructions::{Op, Register, Word};

#[derive(Debug, Clone)]
struct Registers {
    registers: HashMap<Register, i32>,
}
impl Registers {
    fn new() -> Self {
        let mut registers = HashMap::new();
        registers.insert(Register::ProgramCounter, 0);
        registers.insert(Register::General(0), 0);

        Registers { registers }
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
        self.registers[&reg]
    }

    pub fn get_general(&self, reg: u32) -> i32 {
        self.registers[&Register::General(reg)]
    }

    pub fn set(&mut self, reg: Register, val: i32) {
        if reg != Register::General(0) {
            self.registers.insert(reg, val);
        }
    }
}

pub struct CPU {
    registers: Registers,
    executors: Vec<Box<dyn Execute>>,
}
impl CPU {
    pub fn new() -> Self {
        let mut decode_eu = Box::new(EU::new(instruction_decoder));
        let mut execute_eu = Box::new(EU::new(acu));
        let mut write_eu = Box::new(EU::new(write));

        let executors: Vec<Box<dyn Execute>> = vec![decode_eu, execute_eu, write_eu];

        CPU {
            registers: Registers::new(),
            executors,
        }
    }

    pub fn run_program(&mut self, instructions: Vec<Word>) {
        while self.registers.pc() < instructions.len() {
            for executor in self.executors.iter_mut() {
                next_layer_input =
                    executor.execute(&instructions, &mut self.registers, next_layer_input);
            }
        }

        println!("{:?}", self.registers.registers);
    }
}

pub struct EUInput<'a, I> {
    pub registers: &'a mut Registers,
    pub instructions: &'a Vec<Word>,
    pub input: I,
}
pub trait Execute {
    fn execute(&mut self, instructions: &Vec<Word>, registers: &mut Registers);
}

struct EU<A: Clone, I, O: Clone> {
    input: A,
    input_mapper: fn(A) -> I,
    function: fn(EUInput<I>) -> O,
    output: Option<O>,
}
impl<A: Clone, I, O: Clone> EU<A, I, O> {
    pub fn new(f: fn(EUInput<I>) -> O, i: A, im: fn(A) -> I) -> Self {
        EU {
            input: i,
            input_mapper: im,
            function: f,
            output: None,
        }
    }

    pub fn get_output(&self) -> &O {
        self.output.as_ref().unwrap()
    }
}
impl<A: Clone, I, O: Clone> Execute for EU<A, I, O> {
    fn execute(&mut self, instructions: &Vec<Word>, registers: &mut Registers) {
        let input: EUInput<I> = EUInput {
            registers,
            instructions,
            input: (self.input_mapper)(self.input.clone()),
        };

        self.output = Some((self.function)(input));
    }
}

fn instruction_decoder(input: EUInput<()>) -> (Op, Register, i32, i32) {
    let (instructions, registers) = (input.instructions, input.registers);

    match instructions[registers.pc()] {
        Word::R(op, ro, rl, rr) => (op, ro, registers.get(rl), registers.get(rr)),
        Word::I(op, ro, rl, i) => (op, ro, registers.get(rl), i),
    }
}

fn acu(input: EUInput<(Op, i32, i32)>) -> i32 {
    let (op, left, right) = input.input;

    match op {
        Op::LoadImmediate => left + right,
        Op::Add | Op::AddImmediate => left + right,
        Op::Subtract | Op::SubtractImmediate => left - right,
        Op::Compare => (left - right).signum(),
        Op::JumpAbsolute | Op::JumpRelative => left + right,
    }
}

fn write(input: EUInput<(Op, Register, i32)>) {
    let registers = input.registers;
    let (op, ro, out) = input.input;

    match op {
        Op::JumpRelative => registers.set(ro, out + registers.pc() as i32),
        Op::JumpAbsolute => registers.set(ro, out),
        _ => {
            registers.set(ro, out);
            registers.inc_pc();
        }
    };
}
