use bytes::{Buf, BufMut, BytesMut};

use crate::{
    branch_prediction::CoreBranchPredictor,
    instructions::{Op, Register, Word},
    reorder_buffer::{Destination, ReorderBuffer, RobState, RobType, RobValue},
    reservation_station::ReservationStation,
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
    pub fn cycle(
        &mut self,
        branch_predictor: &mut CoreBranchPredictor,
        rob: &mut ReorderBuffer,
        reservation_stations: &mut Vec<ReservationStation>,
        memory: &mut BytesMut,
    ) {
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
                    EUType::Memory => self.load_store(rob, inst, memory),
                    EUType::FPU => self.fpu(rob, inst),
                    EUType::VPU => self.vpu(rob, inst),
                    EUType::System => self.system(rob, inst),
                };

                // forward result to reservation stations :D

                let rob_index = inst.rob_index;
                if let Some(inst) = rob.get_mut(inst.rob_index).as_mut() {
                    if inst.state == RobState::Finished && inst.destination.is_reg() {
                        match inst.value {
                            RobValue::Overflow(_, _) => {
                                reservation_stations.iter_mut().for_each(|rs| {
                                    rs.update_operands(rob_index, inst.value.clone())
                                });
                            }
                            RobValue::Value(value) => {
                                if !(inst.inst == RobType::Branch && value == -1)
                                    && inst.op != Op::ReserveMemory
                                // && inst.op != Op::JumpRegister
                                {
                                    // println!("{:?}", inst);
                                    // propogate to the reservation stations too
                                    reservation_stations.iter_mut().for_each(|rs| {
                                        rs.update_operands(rob_index, RobValue::Value(value))
                                    });
                                }
                            }
                            RobValue::Vector(_) => {
                                let value = inst.value.to_vector();

                                // propogate to the reservation stations too
                                reservation_stations.iter_mut().for_each(|rs| {
                                    rs.update_operands(rob_index, RobValue::Vector(value))
                                });
                            }
                        }
                    }
                }
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
                Destination::Memory((inst.left.to_value() + inst.right.to_value()) as usize), // start position
                RobValue::Value(inst.ret.to_value()), // number of bytes
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
        _branch_predictor: &mut CoreBranchPredictor,
    ) {
        let mut value = -1;
        let mut dest = Destination::Reg(Register::ProgramCounter);
        let mut should_branch = true;
        let op = inst.word.op();

        if op == Op::JumpAndLink {
            dest = Destination::Reg(inst.ret.to_reg());
            let left = inst.left.to_value();
            let right = inst.right.to_value();
            value = left + right;
        } else if op == Op::JumpRegister {
            let left = inst.left.to_value();
            let right = inst.right.to_value();
            value = left + right;
        } else {
            let left = inst.ret.to_value();
            let right = inst.left.to_value();
            let offset = inst.right.to_value();

            should_branch = match op {
                Op::BranchEqual => left == right,
                Op::BranchNotEqual => left != right,
                Op::BranchGreater => left > right,
                Op::BranchGreaterEqual => left >= right,
                Op::BranchLess => left < right,
                Op::BranchLessEqual => left <= right,
                _ => panic!("Branch does not implement this instruction: {:?}", op),
            };

            if should_branch && !inst.branch_taken {
                value = (inst.pc as i32) + offset;
            } else if !should_branch && inst.branch_taken {
                value = (inst.pc as i32) + 1;
            }
        }

        // update the reorder buffer to say this instruction is now finished
        if let Some(rob_el) = rob.get_mut(inst.rob_index).as_mut() {
            rob_el.state = RobState::Finished;
            rob_el.destination = dest;
            rob_el.value = RobValue::Value(value);
            rob_el.taken = should_branch;
        }
    }

    pub fn alu(&mut self, rob: &mut ReorderBuffer, inst: ExeInst) {
        let op = inst.word.op();
        let dest = inst.ret.to_reg();
        let left = inst.left.to_value();
        let right = inst.right.to_value();

        if op == Op::Divide && right == 0 {
            if let Some(rob_el) = rob.get_mut(inst.rob_index).as_mut() {
                rob_el.state = RobState::Errored("tried to divide by 0");
                rob_el.destination = Destination::Reg(dest);
                rob_el.value = RobValue::Value(0);
            }
            return;
        }

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
        let value;

        if op == Op::VSum {
            let mut b = BytesMut::new();
            b.put_u128(inst.right.to_vector());
            let left = inst.left.to_value();
            value = RobValue::Value(left + b.get_i32() + b.get_i32() + b.get_i32() + b.get_i32());
        } else {
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
                    Op::VSubtract => {
                        (i32::from_be_bytes(il) - i32::from_be_bytes(ir)).to_be_bytes()
                    }
                    Op::VMultiply => {
                        (i32::from_be_bytes(il) * i32::from_be_bytes(ir)).to_be_bytes()
                    }
                    Op::VDivide => (i32::from_be_bytes(il) / i32::from_be_bytes(ir)).to_be_bytes(),
                    Op::VLeftShift => {
                        (i32::from_be_bytes(il) << i32::from_be_bytes(ir)).to_be_bytes()
                    }
                    Op::VRightShift => {
                        (i32::from_be_bytes(il) >> i32::from_be_bytes(ir)).to_be_bytes()
                    }
                    Op::VFAdd => (f32::from_be_bytes(il) + f32::from_be_bytes(ir)).to_be_bytes(),
                    Op::VFSubtract => {
                        (f32::from_be_bytes(il) - f32::from_be_bytes(ir)).to_be_bytes()
                    }
                    Op::VFMultiply => {
                        (f32::from_be_bytes(il) * f32::from_be_bytes(ir)).to_be_bytes()
                    }
                    Op::VFDivide => (f32::from_be_bytes(il) / f32::from_be_bytes(ir)).to_be_bytes(),
                    _ => panic!("VPU does not implement this instruction: {:?}", op),
                };

                b.put(&io[..]);
            }

            value = RobValue::Vector(b.get_u128());
        }

        // update the reorder buffer to say this instruction is now finished
        if let Some(rob_el) = rob.get_mut(inst.rob_index).as_mut() {
            rob_el.state = RobState::Finished;
            rob_el.destination = Destination::Reg(dest);
            rob_el.value = value;
        }
    }

    // calculates address of the thing we need to store OR the value (load immediate)
    pub fn load_store(&mut self, rob: &mut ReorderBuffer, inst: ExeInst, memory: &mut BytesMut) {
        let op = inst.word.op();

        let (dest, value) = match op {
            Op::LoadImmediate | Op::FLoadImmediate => {
                let dest = inst.ret.to_reg();
                let left = inst.left.to_value();
                let right = inst.right.to_value();
                (Destination::Reg(dest), RobValue::Value(left + right))
            }
            Op::LoadMemory | Op::LoadHalfWord | Op::LoadChar | Op::VLoadMemory => {
                let dest = inst.ret.to_reg();
                let addr = (inst.left.to_value() + inst.right.to_value()) as usize;

                let value = if memory.len() < addr {
                    RobValue::Value(0)
                } else if op == Op::LoadMemory && addr + 4 <= memory.len() {
                    RobValue::Value((&memory[addr..(addr + 4)]).get_i32())
                } else if op == Op::LoadHalfWord && addr + 2 <= memory.len() {
                    RobValue::Value((&memory[addr..(addr + 2)]).get_u16() as i32)
                } else if op == Op::LoadChar && addr + 1 <= memory.len() {
                    RobValue::Value((&memory[addr..addr + 1]).get_u8() as i32)
                } else if op == Op::VLoadMemory && addr + 16 <= memory.len() {
                    RobValue::Vector((&memory[addr..(addr + 16)]).get_u128())
                } else {
                    RobValue::Value(0)
                };

                (Destination::Reg(dest), value)
            }
            Op::VStoreMemory => {
                let value = inst.ret.to_vector();
                let left = inst.left.to_value();
                let right = inst.right.to_value();
                (
                    Destination::Memory((left + right) as usize),
                    RobValue::Vector(value),
                )
            }
            Op::StoreMemory | Op::StoreChar => {
                let value = inst.ret.to_value();
                let left = inst.left.to_value();
                let right = inst.right.to_value();
                (
                    Destination::Memory((left + right) as usize),
                    RobValue::Value(value),
                )
            }
            Op::MoveFromHigh | Op::MoveFromLow => {
                let dest = inst.ret.to_reg();
                let value = inst.left.to_value();
                (Destination::Reg(dest), RobValue::Value(value))
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
