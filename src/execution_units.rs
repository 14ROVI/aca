use bytes::{Buf, BufMut, BytesMut};

use crate::{
    branch_prediction::BranchPredictor,
    instructions::{Op, Register, Word},
    reorder_buffer::{Destination, ReorderBuffer, RobState, RobValue},
};

#[derive(Debug, Clone, Copy)]
pub enum ExeOperand {
    Reg(Register),
    Value(i32),
    Vector(u128),
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

    pub fn to_vector(&self) -> u128 {
        match self {
            Self::Vector(val) => *val,
            _ => panic!("ExeOperand {:?} is not a vector!", self),
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
    FPU,
    VPU,
    System,
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
                match self.flavour {
                    EUType::ALU => self.alu(rob, inst),
                    EUType::Branch => self.branch(rob, inst, branch_predictor),
                    EUType::Memory => self.load_store(rob, inst),
                    EUType::FPU => self.fpu(rob, inst),
                    EUType::VPU => self.vpu(rob, inst),
                    EUType::System => self.system(rob, inst),
                };
            }
        }
    }

    pub fn system(&mut self, rob: &mut ReorderBuffer, inst: ExeInst) {
        let op = inst.word.op();

        let (dest, value) = match op {
            Op::Exit => (Destination::None, RobValue::Value(inst.left.to_value())),
            Op::ReserveMemory => {
                let left = inst.left.to_value();
                let right = inst.right.to_value();
                (
                    Destination::Reg(inst.ret.to_reg()),
                    RobValue::Value(left + right),
                )
            }
            Op::Save => (
                Destination::Memory(inst.right.to_value() as usize), // start position
                RobValue::Value(inst.left.to_value()),               // number of bytes
            ),
            _ => panic!("System command {:?} not implemented!", op),
        };

        if let Some(rob_el) = rob.get_mut(inst.rob_index).as_mut() {
            rob_el.state = RobState::Finished;
            rob_el.destination = dest;
            rob_el.value = value;
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
        let mut dest = Destination::Reg(Register::ProgramCounter);
        let op = inst.word.op();

        if op == Op::JumpAndLink {
            dest = Destination::Reg(inst.ret.to_reg());
            let left = inst.left.to_value();
            let right = inst.right.to_value();
            value = left + right;
        } else if op == Op::JumpRegister {
            if !inst.branch_taken {
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
            }
        }

        // update the reorder buffer to say this instruction is now finished
        // let mut rob_el = ;
        if let Some(rob_el) = rob.get_mut(inst.rob_index).as_mut() {
            rob_el.state = RobState::Finished;
            rob_el.destination = dest;
            rob_el.value = RobValue::Value(value);
        }
    }

    pub fn alu(&mut self, rob: &mut ReorderBuffer, inst: ExeInst) {
        let op = inst.word.op();
        let dest = inst.ret.to_reg();
        let left = inst.left.to_value();
        let right = inst.right.to_value();

        let out = match op {
            Op::Add | Op::AddImmediate => RobValue::Value(left + right),
            Op::Subtract | Op::SubtractImmediate => RobValue::Value(left - right),
            Op::Compare => RobValue::Value((left - right).signum()),
            Op::Multiply => RobValue::Value((left * right) as i32),
            Op::MultiplyNoOverflow => RobValue::Overflow(
                ((left as i64 * right as i64) >> 32) as i32,
                (left * right) as i32,
            ),
            Op::Divide => RobValue::Overflow(left / right, left % right),
            Op::LeftShift => RobValue::Value(left << right),
            Op::RightShift => RobValue::Value(left >> right),
            Op::BitAnd | Op::BitAndImmediate => RobValue::Value(left & right),
            Op::BitOr | Op::BitOrImmediate => RobValue::Value(left | right),
            Op::Neg => RobValue::Value(-left),
            _ => panic!("ALU does not implement this instruction: {:?}", op),
        };

        // update the reorder buffer to say this instruction is now finished
        if let Some(rob_el) = rob.get_mut(inst.rob_index).as_mut() {
            rob_el.state = RobState::Finished;
            rob_el.destination = Destination::Reg(dest);
            rob_el.value = out;
        }
    }

    pub fn fpu(&mut self, rob: &mut ReorderBuffer, inst: ExeInst) {
        let op = inst.word.op();
        let dest = inst.ret.to_reg();
        let left = f32::from_be_bytes(inst.left.to_value().to_be_bytes());
        let right = f32::from_be_bytes(inst.right.to_value().to_be_bytes());

        let out = match op {
            Op::FAdd | Op::FAddImmediate => left + right,
            Op::FSubtract | Op::FSubtractImmediate => left - right,
            Op::FCompare => (left - right).signum(),
            Op::FMultiply => (left * right) as f32,
            Op::FDivide => (left / right) as f32,
            _ => panic!("FPU does not implement this instruction: {:?}", op),
        };

        // update the reorder buffer to say this instruction is now finished
        if let Some(rob_el) = rob.get_mut(inst.rob_index).as_mut() {
            rob_el.state = RobState::Finished;
            rob_el.destination = Destination::Reg(dest);
            rob_el.value = RobValue::Value(i32::from_be_bytes(out.to_be_bytes()));
        }
    }

    pub fn vpu(&mut self, rob: &mut ReorderBuffer, inst: ExeInst) {
        let op = inst.word.op();
        let dest = inst.ret.to_reg();

        let mut b = BytesMut::new();
        b.put_u128(inst.left.to_vector());
        let left = [b.get_u32(), b.get_u32(), b.get_u32(), b.get_u32()];

        b.put_u128(inst.right.to_vector());
        let right = [b.get_u32(), b.get_u32(), b.get_u32(), b.get_u32()];

        for i in 0..4 {
            let il = left[i].to_be_bytes();
            let ir = right[i].to_be_bytes();

            let io = match op {
                Op::VAdd => (i32::from_be_bytes(il) + i32::from_be_bytes(ir)).to_be_bytes(),
                Op::VSubtract => (i32::from_be_bytes(il) - i32::from_be_bytes(ir)).to_be_bytes(),
                Op::VMultiply => (i32::from_be_bytes(il) * i32::from_be_bytes(ir)).to_be_bytes(),
                Op::VDivide => (i32::from_be_bytes(il) / i32::from_be_bytes(ir)).to_be_bytes(),
                Op::VLeftShift => (i32::from_be_bytes(il) << i32::from_be_bytes(ir)).to_be_bytes(),
                Op::VRightShift => (i32::from_be_bytes(il) >> i32::from_be_bytes(ir)).to_be_bytes(),
                Op::VFAdd => (f32::from_be_bytes(il) + f32::from_be_bytes(ir)).to_be_bytes(),
                Op::VFSubtract => (f32::from_be_bytes(il) - f32::from_be_bytes(ir)).to_be_bytes(),
                Op::VFMultiply => (f32::from_be_bytes(il) * f32::from_be_bytes(ir)).to_be_bytes(),
                Op::VFDivide => (f32::from_be_bytes(il) / f32::from_be_bytes(ir)).to_be_bytes(),
                _ => panic!("VPU does not implement this instruction: {:?}", op),
            };

            b.put(&io[..]);
        }

        let out = b.get_u128();

        // update the reorder buffer to say this instruction is now finished
        if let Some(rob_el) = rob.get_mut(inst.rob_index).as_mut() {
            rob_el.state = RobState::Finished;
            rob_el.destination = Destination::Reg(dest);
            rob_el.value = RobValue::Vector(out);
        }
    }

    // calculates address of the thing we need to store OR the value (load immediate)
    pub fn load_store(&mut self, rob: &mut ReorderBuffer, inst: ExeInst) {
        let op = inst.word.op();

        let (dest, value) = match op {
            Op::LoadImmediate | Op::FLoadImmediate => {
                let dest = inst.ret.to_reg();
                let left = inst.left.to_value();
                let right = inst.right.to_value();
                (Destination::Reg(dest), left + right)
            }
            Op::LoadMemory | Op::VLoadMemory | Op::LoadHalfWord | Op::LoadChar => {
                let dest = inst.ret.to_reg();
                let left = inst.left.to_value();
                let right = inst.right.to_value();
                (Destination::Reg(dest), left + right)
            }
            Op::StoreMemory | Op::VStoreMemory | Op::StoreChar => {
                let value = inst.ret.to_value();
                let left = inst.left.to_value();
                let right = inst.right.to_value();
                (Destination::Memory((left + right) as usize), value)
            }
            Op::MoveFromHigh | Op::MoveFromLow => {
                let dest = inst.ret.to_reg();
                let value = inst.left.to_value();
                (Destination::Reg(dest), value)
            }
            _ => panic!("LSU does not implement this instruction: {:?}", op),
        };

        // update the reorder buffer to say this instruction is now finished
        if let Some(rob_el) = rob.get_mut(inst.rob_index).as_mut() {
            rob_el.state = RobState::Finished;
            rob_el.destination = dest;
            rob_el.value = RobValue::Value(value);
        }
    }
}
