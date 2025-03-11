use std::collections::HashMap;

use crate::instructions::Register;

#[derive(Debug, Clone)]
pub struct Registers {
    pub registers: HashMap<Register, i32>,
}
impl Registers {
    pub fn new() -> Self {
        let mut registers = HashMap::new();
        registers.insert(Register::ProgramCounter, 0);
        for i in 0..32 {
            registers.insert(Register::General(i), 0);
        }

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
        let register = Register::General(reg);
        self.registers[&register]
    }

    pub fn set(&mut self, reg: Register, val: i32) {
        if reg != Register::General(0) {
            self.registers.insert(reg, val);
        }
    }
}
