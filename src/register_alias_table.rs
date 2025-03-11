use std::collections::HashMap;

use crate::instructions::Register;

pub enum Tag {
    Register(Register),
    Rob(usize),
}

pub struct RegisterAliasTable {
    pub table: HashMap<Register, usize>,
}
impl RegisterAliasTable {
    pub fn new() -> Self {
        Self {
            table: HashMap::new(),
        }
    }

    pub fn flush(&mut self) {
        *self = Self::new();
    }

    pub fn set(&mut self, reg: Register, rob_index: usize) {
        self.table.insert(reg, rob_index);
    }

    pub fn get(&self, reg: Register) -> Tag {
        self.table
            .get(&reg)
            .map_or_else(|| Tag::Register(reg), |rob_index| Tag::Rob(*rob_index))
    }

    pub fn remove(&mut self, reg: Register) {
        self.table.remove(&reg);
    }
}
