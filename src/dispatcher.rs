use crate::{
    fetcher::Fetcher,
    instructions::{Op, Register, Word},
    register_alias_table::{RegisterAliasTable, Tag},
    registers::Registers,
    reorder_buffer::{Destination, ReorderBuffer, RobInst, RobState},
    reservation_station::{ResInst, ResOperand, ReservationStation},
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
    ) {
        for _ in 0..self.dispatch_amount {
            let mut rs: Option<&mut ReservationStation> = None;

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
                    Tag::Register(reg) => ResOperand::Value(registers.get(reg)),
                    Tag::Rob(index) => ResOperand::Rob(index),
                };

                let mut ret_op = ResOperand::Value(0);
                let mut left_op = ResOperand::Value(0);
                let mut right_op = ResOperand::Value(0);

                match word.op() {
                    Op::LoadMemory => {
                        if let Word::I(_, ro, rl, i) = word {
                            ret_op = ResOperand::Reg(ro);
                            left_op = make_res_operand(rl);
                            right_op = ResOperand::Value(i);
                        }
                    }
                    Op::StoreMemory => {
                        if let Word::I(_, ro, rl, i) = word {
                            ret_op = make_res_operand(ro);
                            left_op = make_res_operand(rl);
                            right_op = ResOperand::Value(i);
                        }
                    }

                    Op::LoadImmediate
                    | Op::SubtractImmediate
                    | Op::AddImmediate
                    | Op::BitAndImmediate
                    | Op::BitOrImmediate
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
                    | Op::BitOr => {
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
                }

                let mut rob_inst = RobInst {
                    inst: word.op().rob_type(),
                    index: 0,
                    destination: Destination::None,
                    value: 0,
                    state: RobState::Issued,
                };

                let rob_index = rob.add_instruction(rob_inst); // add to reorder buffer

                if word.op().updates_rat() {
                    if let ResOperand::Reg(reg) = ret_op {
                        rat.set(reg, rob_index); // future instructions will now use this rob index's output for this register's value.
                        rob.get_mut(rob_index).as_mut().unwrap().destination =
                            Destination::Reg(reg);
                    }
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
            } else {
                return;
            }
        }
    }
}
