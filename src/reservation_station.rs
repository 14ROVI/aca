use crate::{
    execution_units::{EUType, ExeInst, ExeOperand},
    instructions::{Op, Register, Word},
    reorder_buffer::RobValue,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResOperand {
    Reg(Register),
    Rob(usize),
    Value(i32),
    Vector(u128),
}
impl ResOperand {
    pub fn is_rob(&self) -> bool {
        match self {
            Self::Rob(_) => true,
            _ => false,
        }
    }

    pub fn to_exe_operand(&self) -> ExeOperand {
        match self {
            Self::Reg(reg) => ExeOperand::Reg(*reg),
            Self::Value(val) => ExeOperand::Value(*val),
            Self::Vector(val) => ExeOperand::Vector(*val),
            Self::Rob(_) => panic!("ResOperand has not resolved yet!"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ResInst {
    pub word: Word,
    pub pc: usize,
    pub rob_index: usize,
    pub branch_taken: bool,
    pub return_op: ResOperand,
    pub left_op: ResOperand,
    pub right_op: ResOperand,
}
impl ResInst {
    pub fn to_exe_inst(&self) -> ExeInst {
        ExeInst {
            word: self.word,
            pc: self.pc,
            rob_index: self.rob_index,
            branch_taken: self.branch_taken,
            ret: self.return_op.to_exe_operand(),
            left: self.left_op.to_exe_operand(),
            right: self.right_op.to_exe_operand(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReservationStation {
    pub buffer: Vec<ResInst>,
    capacity: usize,
    reserves_for: EUType,
}
impl ReservationStation {
    pub fn new(capacity: usize, reserves_for: EUType) -> Self {
        ReservationStation {
            buffer: Vec::new(),
            capacity,
            reserves_for,
        }
    }

    pub fn reserves_for(&self) -> EUType {
        self.reserves_for
    }

    pub fn is_full(&self) -> bool {
        self.buffer.len() == self.capacity
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.len() == 0
    }

    pub fn flush(&mut self) {
        *self = Self::new(self.capacity, self.reserves_for);
    }

    pub fn add_instruction(&mut self, instruction: ResInst) {
        if self.buffer.len() < self.capacity {
            self.buffer.push(instruction);
        } else {
            panic!("Tried to add instruction to rs but doesn't have capacity!");
        }
    }

    pub fn update_operands(&mut self, rob_index: usize, value: RobValue) {
        let res_op = ResOperand::Rob(rob_index);

        for inst in self.buffer.iter_mut() {
            if value.is_overflow() {
                if inst.word.op() == Op::MoveFromHigh && inst.left_op == res_op {
                    inst.left_op = ResOperand::Value(value.to_overflow().0);
                } else if inst.word.op() == Op::MoveFromLow && inst.left_op == res_op {
                    inst.left_op = ResOperand::Value(value.to_overflow().1);
                }
            } else {
                let res_val = match value {
                    RobValue::Value(val) => ResOperand::Value(val),
                    RobValue::Vector(val) => ResOperand::Vector(val),
                    _ => panic!("not supposed to happen bruh"),
                };
                if inst.return_op == res_op {
                    inst.return_op = res_val;
                }
                if inst.left_op == res_op {
                    inst.left_op = res_val;
                }
                if inst.right_op == res_op {
                    inst.right_op = res_val;
                }
            }
        }
    }

    pub fn take_oldest_valid(&mut self) -> Option<ResInst> {
        for i in 0..self.buffer.len() {
            let inst = &self.buffer[i];

            if !inst.return_op.is_rob() && !inst.left_op.is_rob() && !inst.right_op.is_rob() {
                let inst = self.buffer.remove(i);
                return Some(inst);
            }
        }

        return None;
    }
}
