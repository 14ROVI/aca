use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    hash::Hash,
    rc::Rc,
};

use crate::instructions::{Op, Word};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
enum Register {
    ProgramCounter,
    General(u32),
    CurrentOp,
}

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

struct Clock {
    cycle: usize,
}
impl Clock {
    fn new() -> Self {
        Clock { cycle: 0 }
    }

    fn cycle(&self) -> usize {
        self.cycle
    }

    fn adv(&mut self) {
        self.cycle += 1;
    }
}

pub struct CPU {
    registers: Rc<RefCell<Registers>>,
    layers: Group,
    cur_instruction: Rc<Cell<Word>>,
}
impl CPU {
    pub fn new() -> Self {
        let registers = Rc::new(RefCell::new(Registers::new()));
        let cur_instruction = Rc::new(Cell::new(Op::add(0, 0, 0)));
        let ro = Rc::new(Cell::new(0));
        let op = Rc::new(Cell::new(Op::Add));
        let left = Rc::new(Cell::new(0));
        let right = Rc::new(Cell::new(0));
        let out = Rc::new(Cell::new(0));

        let layers = Group {
            children: vec![
                Box::new(Decoder {
                    word: cur_instruction.clone(),
                    registers: registers.clone(),
                    op: op.clone(),
                    ro: ro.clone(),
                    left: left.clone(),
                    right: right.clone(),
                }),
                Box::new(ALU {
                    op: op.clone(),
                    left: left.clone(),
                    right: right.clone(),
                    out: out.clone(),
                }),
                Box::new(Writer {
                    registers: registers.clone(),
                    out: out.clone(),
                    ro: ro.clone(),
                }),
            ],
        };

        CPU {
            registers,
            layers,
            cur_instruction,
        }
    }

    pub fn run_program(&mut self, instructions: Vec<Word>) {
        while self.registers.borrow().pc() < instructions.len() {
            // fetch
            let instruction = instructions[self.registers.borrow().pc()];
            self.cur_instruction.set(instruction);

            self.layers.execute();

            self.registers.borrow_mut().inc_pc();
        }

        println!("{:?}", self.registers.borrow().registers);
    }
}

pub struct Group {
    pub children: Vec<Box<dyn Execute>>,
}
impl Execute for Group {
    fn execute(&mut self) {
        for child in self.children.iter_mut() {
            child.execute();
        }
    }
}

pub struct Decoder {
    pub word: Rc<Cell<Word>>,
    pub registers: Rc<RefCell<Registers>>,
    pub op: Rc<Cell<Op>>,
    pub ro: Rc<Cell<u32>>,
    pub left: Rc<Cell<i32>>,
    pub right: Rc<Cell<i32>>,
}
impl Execute for Decoder {
    fn execute(&mut self) {
        let word = self.word.get();

        match word {
            Word::R(op, ro, rl, rr, _, _) => {
                self.op.set(op);
                self.ro.set(ro);
                self.left.set(self.registers.borrow().get_general(rl));
                self.right.set(self.registers.borrow().get_general(rr));
            }
            Word::I(op, ro, rl, i) => {
                self.op.set(op);
                self.ro.set(ro);
                self.left.set(self.registers.borrow().get_general(rl));
                self.right.set(i);
            }
            _ => panic!("not implemented!"),
        };
    }
}

pub struct Writer {
    pub registers: Rc<RefCell<Registers>>,
    pub out: Rc<Cell<i32>>,
    pub ro: Rc<Cell<u32>>,
}
impl Execute for Writer {
    fn execute(&mut self) {
        let ro = self.ro.get();
        let out = self.out.get();

        self.registers.borrow_mut().set(Register::General(ro), out);
    }
}

pub struct ALU {
    pub op: Rc<Cell<Op>>,
    pub left: Rc<Cell<i32>>,
    pub right: Rc<Cell<i32>>,
    pub out: Rc<Cell<i32>>,
}
impl Execute for ALU {
    fn execute(&mut self) {
        let op = self.op.get();
        let left = self.left.get();
        let right = self.right.get();

        println!("executing: {:?}", op);

        let out = match op {
            Op::LoadImmediate => left + right,
            Op::Add | Op::AddImmediate => left + right,
            Op::Subtract | Op::SubtractImmediate => left - right,
            Op::Compare => (left - right).signum(),
        };

        self.out.set(out);
    }
}

pub trait Execute {
    fn execute(&mut self);
}
