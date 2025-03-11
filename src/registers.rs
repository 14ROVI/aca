use std::collections::HashMap;

use crate::instructions::Register;

#[derive(Debug, Clone)]
pub struct Registers {
    pub general_registers: HashMap<Register, i32>,
    pub vector_registers: HashMap<Register, u128>,
}
impl Registers {
    pub fn new() -> Self {
        let mut general_registers = HashMap::new();
        let mut vector_registers = HashMap::new();
        general_registers.insert(Register::ProgramCounter, 0);
        general_registers.insert(Register::High, 0);
        general_registers.insert(Register::Low, 0);
        for i in 0..64 {
            general_registers.insert(Register::General(i), 0);
        }
        for i in 0..2 {
            vector_registers.insert(Register::Vector(i), 0);
        }

        Registers {
            general_registers,
            vector_registers,
        }
    }

    pub fn pc(&self) -> usize {
        self.general_registers[&Register::ProgramCounter] as usize
    }

    pub fn inc_pc(&mut self) {
        self.general_registers.insert(
            Register::ProgramCounter,
            self.general_registers[&Register::ProgramCounter] + 1,
        );
    }

    pub fn get(&self, reg: Register) -> i32 {
        self.general_registers[&reg]
    }

    pub fn get_vector(&self, reg: Register) -> u128 {
        self.vector_registers[&reg]
    }

    pub fn set(&mut self, reg: Register, val: i32) {
        if reg != Register::General(0) {
            self.general_registers.insert(reg, val);
        }
    }

    pub fn set_vector(&mut self, reg: Register, val: u128) {
        self.vector_registers.insert(reg, val);
    }
}
