use crate::{
    branch_prediction::BranchPredictor,
    instructions::{Op, Register, Word},
    reorder_buffer::{Destination, ReorderBuffer, RobState},
};

#[derive(Debug, Clone, Copy)]
pub enum ExeOperand {
    Reg(Register),
    Value(i32),
}
impl ExeOperand {
    pub fn to_reg(&self) -> Register {
        match self {
            Self::Reg(reg) => *reg,
            _ => panic!("ExeOperand is not a register!"),
        }
    }

    pub fn to_value(&self) -> i32 {
        match self {
            Self::Value(val) => *val,
            _ => panic!("ExeOperand is not a value!"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ExeInst {
    pub word: Word,
    pub pc: usize,
    pub rob_index: usize,
    pub branch_taken: bool,
    pub ret: ExeOperand,
    pub left: ExeOperand,
    pub right: ExeOperand,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EUType {
    ALU,
    Branch,
    Memory,
}

#[derive(Debug, Clone)]
pub struct ExecutionUnit {
    pub flavour: EUType,
    cycles_left: usize,
    pub inst: Option<ExeInst>,
}
impl ExecutionUnit {
    pub fn new(flavour: EUType) -> Self {
        ExecutionUnit {
            flavour,
            cycles_left: 0,
            inst: None,
        }
    }

    pub fn is_busy(&self) -> bool {
        self.cycles_left != 0
    }

    pub fn flush(&mut self) {
        *self = Self::new(self.flavour);
    }

    /// does the thing
    pub fn start(&mut self, inst: ExeInst, rob: &mut ReorderBuffer) {
        self.inst = Some(inst);
        self.cycles_left = inst.word.op().cycles_needed();
        rob.get_mut(inst.rob_index).as_mut().unwrap().state = RobState::Executing;
    }

    /// reduces by one each time, on final cycle send it to the rob.
    pub fn cycle(&mut self, branch_predictor: &mut BranchPredictor, rob: &mut ReorderBuffer) {
        // cycle
        if self.cycles_left >= 1 {
            self.cycles_left -= 1;
        }

        // if last cycle actually execute
        if self.cycles_left == 0 {
            if let Some(inst) = self.inst.take() {
                // execute
                let op = inst.word.op();
                // println!("execute {:?}", op);

                match self.flavour {
                    EUType::ALU => self.alu(rob, inst),
                    EUType::Branch => self.branch(rob, inst, branch_predictor),
                    EUType::Memory => self.load_store(rob, inst),
                };
            }
        }
    }

    /// if out == -1 then the branch is valid. otherwise out == new pc and we have to flush!
    pub fn branch(
        &mut self,
        rob: &mut ReorderBuffer,
        inst: ExeInst,
        branch_predictor: &mut BranchPredictor,
    ) {
        let mut value = -1;
        let op = inst.word.op();

        if op == Op::JumpRegister {
            if inst.branch_taken {
                value = -1;
            } else {
                let left = inst.left.to_value();
                let right = inst.right.to_value();
                value = left + right;
            }
        } else {
            let left = inst.ret.to_value();
            let right = inst.left.to_value();
            let offset = inst.right.to_value();

            let should_branch = match op {
                Op::BranchEqual => left == right,
                Op::BranchNotEqual => left != right,
                Op::BranchGreater => left > right,
                Op::BranchGreaterEqual => left >= right,
                Op::BranchLess => left < right,
                Op::BranchLessEqual => left <= right,
                _ => panic!("Branch does not implement this instruction: {:?}", op),
            };

            if op.is_predictable_branch() {
                branch_predictor.update(inst.pc, should_branch);
            }

            if should_branch && !inst.branch_taken {
                value = (inst.pc as i32) + offset;
            } else if !should_branch && inst.branch_taken {
                value = (inst.pc as i32) + 1;
            } else {
                value = -1; // no change
            }
        }

        // update the reorder buffer to say this instruction is now finished
        // let mut rob_el = ;
        if let Some(rob_el) = rob.get_mut(inst.rob_index).as_mut() {
            rob_el.state = RobState::Finished;
            rob_el.destination = Destination::Reg(Register::ProgramCounter);
            rob_el.value = value;
        }
    }

    pub fn alu(&mut self, rob: &mut ReorderBuffer, inst: ExeInst) {
        let op = inst.word.op();
        let dest = inst.ret.to_reg();
        let left = inst.left.to_value();
        let right = inst.right.to_value();

        let out = match op {
            Op::Add | Op::AddImmediate => left + right,
            Op::Subtract | Op::SubtractImmediate => left - right,
            Op::Compare => (left - right).signum(),
            Op::Multiply => (left * right) as i32,
            Op::Divide => (left / right) as i32,
            Op::LeftShift => left << right,
            Op::RightShift => left >> right,
            Op::BitAnd | Op::BitAndImmediate => left & right,
            Op::BitOr | Op::BitOrImmediate => left | right,
            _ => panic!("ALU does not implement this instruction: {:?}", op),
        };

        // update the reorder buffer to say this instruction is now finished
        if let Some(rob_el) = rob.get_mut(inst.rob_index).as_mut() {
            rob_el.state = RobState::Finished;
            rob_el.destination = Destination::Reg(dest);
            rob_el.value = out;
        }
    }

    // calculates address of the thing we need to store OR the value (load immediate)
    pub fn load_store(&mut self, rob: &mut ReorderBuffer, inst: ExeInst) {
        let op = inst.word.op();

        let (dest, value) = match op {
            Op::LoadImmediate => {
                let dest = inst.ret.to_reg();
                let left = inst.left.to_value();
                let right = inst.right.to_value();
                (Destination::Reg(dest), left + right)
            }
            Op::LoadMemory => {
                let dest = inst.ret.to_reg();
                let left = inst.left.to_value();
                let right = inst.right.to_value();
                (Destination::Reg(dest), left + right)
            }
            Op::StoreMemory => {
                let value = inst.ret.to_value();
                let left = inst.left.to_value();
                let right = inst.right.to_value();
                (Destination::Memory((left + right) as usize), value)
            }
            _ => panic!("LSU does not implement this instruction: {:?}", op),
        };

        // update the reorder buffer to say this instruction is now finished
        if let Some(rob_el) = rob.get_mut(inst.rob_index).as_mut() {
            rob_el.state = RobState::Finished;
            rob_el.destination = dest;
            rob_el.value = value;
        }
    }
}
