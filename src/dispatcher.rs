use crate::{
    fetcher::Fetcher,
    instructions::{Op, Register, Word},
    register_alias_table::{RegisterAliasTable, Tag},
    registers::Registers,
    reorder_buffer::{Destination, ReorderBuffer, RobInst, RobState, RobValue},
    reservation_station::{ResInst, ResOperand, ReservationStation},
    stats::StatsTracker,
};

#[derive(Debug)]
pub struct Dispatcher {
    dispatch_amount: usize,
}
impl Dispatcher {
    pub fn new(dispatch_amount: usize) -> Self {
        Self { dispatch_amount }
    }

    pub fn flush(&mut self) {
        ()
    }

    pub fn dispatch(
        &mut self,
        fetcher: &mut Fetcher,
        registers: &mut Registers,
        rat: &mut RegisterAliasTable,
        rob: &mut ReorderBuffer,
        reservation_stations: &mut Vec<ReservationStation>,
        stats_tracker: &mut StatsTracker,
    ) {
        for _ in 0..self.dispatch_amount {
            let mut rs: Option<&mut ReservationStation> = None;

            if rob.is_full() {
                return;
            }

            if let Some(fetched_word) = fetcher.get_oldest() {
                let word = fetched_word.word;

                rs = reservation_stations
                    .iter_mut()
                    .find(|rs| rs.reserves_for() == word.op().needs_eu_type() && !rs.is_full());
            }

            if rs.is_none() {
                return; // stop dispatching, we cant disapatch this to any reservation station.
            }
            let rs = rs.unwrap();

            // remove oldest from the buffer
            if let Some(fetched_word) = fetcher.take_oldest() {
                let word = fetched_word.word;

                let make_res_operand = |reg: Register| match rat.get(reg) {
                    Tag::Register(reg) => match reg {
                        Register::General(_)
                        | Register::ProgramCounter
                        | Register::High
                        | Register::Low => ResOperand::Value(registers.get(reg)),
                        Register::Vector(_) => ResOperand::Vector(registers.get_vector(reg)),
                    },
                    Tag::Rob(index) => ResOperand::Rob(index),
                };

                let mut ret_op = ResOperand::Value(0);
                let mut left_op = ResOperand::Value(0);
                let mut right_op = ResOperand::Value(0);

                match word.op() {
                    Op::LoadMemory | Op::VLoadMemory | Op::LoadHalfWord | Op::LoadChar => {
                        if let Word::I(_, ro, rl, i) = word {
                            ret_op = ResOperand::Reg(ro);
                            left_op = make_res_operand(rl);
                            right_op = ResOperand::Value(i);
                        }
                    }
                    Op::Save => {
                        if let Word::I(_, ro, rl, i) = word {
                            ret_op = make_res_operand(ro);
                            left_op = make_res_operand(rl);
                            right_op = ResOperand::Value(i);
                        }
                    }

                    Op::StoreMemory | Op::VStoreMemory | Op::StoreChar => {
                        if let Word::I(_, ro, rl, i) = word {
                            ret_op = make_res_operand(ro);
                            left_op = make_res_operand(rl);
                            right_op = ResOperand::Value(i);
                        }
                    }
                    Op::ReserveMemory => {
                        if let Word::I(_, ro, rl, i) = word {
                            ret_op = ResOperand::Reg(ro);
                            left_op = make_res_operand(rl);
                            right_op = ResOperand::Value(i);
                        }
                    }

                    Op::LoadImmediate
                    | Op::FLoadImmediate
                    | Op::SubtractImmediate
                    | Op::FSubtractImmediate
                    | Op::FAddImmediate
                    | Op::AddImmediate
                    | Op::BitAndImmediate
                    | Op::BitOrImmediate
                    | Op::Neg
                    | Op::LeftShift
                    | Op::RightShift => {
                        if let Word::I(_, ro, rl, i) = word {
                            ret_op = ResOperand::Reg(ro);
                            left_op = make_res_operand(rl);
                            right_op = ResOperand::Value(i);
                        }
                    }
                    Op::Add
                    | Op::Subtract
                    | Op::Multiply
                    | Op::MultiplyNoOverflow
                    | Op::Divide
                    | Op::Compare
                    | Op::BitAnd
                    | Op::BitOr
                    | Op::FAdd
                    | Op::FSubtract
                    | Op::FMultiply
                    | Op::FDivide
                    | Op::FCompare
                    | Op::VAdd
                    | Op::VSubtract
                    | Op::VMultiply
                    | Op::VDivide
                    | Op::VLeftShift
                    | Op::VRightShift
                    | Op::VFAdd
                    | Op::VFSubtract
                    | Op::VFMultiply
                    | Op::VFDivide
                    | Op::VSum => {
                        if let Word::R(_, ro, rl, rr) = word {
                            ret_op = ResOperand::Reg(ro);
                            left_op = make_res_operand(rl);
                            right_op = make_res_operand(rr);
                        }
                    }

                    Op::BranchEqual
                    | Op::BranchNotEqual
                    | Op::BranchGreater
                    | Op::BranchGreaterEqual
                    | Op::BranchLess
                    | Op::BranchLessEqual => {
                        if let Word::I(_, ro, rl, i) = word {
                            ret_op = make_res_operand(ro);
                            left_op = make_res_operand(rl);
                            right_op = ResOperand::Value(i);
                        }
                    }
                    Op::Jump => {
                        if let Word::JI(_, i) = word {
                            ret_op = ResOperand::Reg(Register::ProgramCounter);
                            left_op = ResOperand::Value(i);
                            right_op = ResOperand::Value(0);
                        }
                    }
                    Op::JumpRegister => {
                        if let Word::JR(_, reg) = word {
                            ret_op = ResOperand::Reg(Register::ProgramCounter);
                            left_op = make_res_operand(reg);
                            right_op = ResOperand::Value(0);
                        }
                    }
                    Op::JumpAndLink => {
                        if let Word::I(_, reg, _, _) = word {
                            ret_op = ResOperand::Reg(reg);
                            left_op = ResOperand::Value((fetched_word.pc + 1) as i32);
                            right_op = ResOperand::Value(0);
                        }
                    }

                    Op::MoveFromHigh | Op::MoveFromLow => {
                        if let Word::I(_, ro, rl, _) = word {
                            ret_op = ResOperand::Reg(ro);
                            left_op = make_res_operand(rl);
                            right_op = ResOperand::Value(0);
                        }
                    }
                    Op::Exit => {
                        if let Word::I(_, _, ri, _) = word {
                            ret_op = ResOperand::Reg(Register::ProgramCounter);
                            left_op = make_res_operand(ri);
                            right_op = ResOperand::Value(0);
                        }
                    }
                }

                let rob_inst = RobInst {
                    inst: word.op().rob_type(),
                    op: word.op(),
                    index: 0,
                    destination: Destination::None,
                    value: RobValue::Value(0),
                    state: RobState::Issued,
                    _speculative: false,
                    taken: fetched_word.branch_taken,
                    pc: fetched_word.pc,
                };

                let rob_index = rob.add_instruction(rob_inst); // add to reorder buffer

                if word.op().updates_rat() {
                    if let ResOperand::Reg(reg) = ret_op {
                        rat.set(reg, rob_index); // future instructions will now use this rob index's output for this register's value.
                        rob.get_mut(rob_index).as_mut().unwrap().destination =
                            Destination::Reg(reg);
                    }
                } else if word.op() == Op::Divide || word.op() == Op::MultiplyNoOverflow {
                    rat.set(Register::High, rob_index);
                    rat.set(Register::Low, rob_index);
                }

                let res_inst = ResInst {
                    word,
                    pc: fetched_word.pc,
                    rob_index,
                    branch_taken: fetched_word.branch_taken,
                    return_op: ret_op,
                    left_op,
                    right_op,
                };

                rs.add_instruction(res_inst); // add to reservation station

                stats_tracker.instructions_started += 1;
            } else {
                return;
            }
        }
    }
}
