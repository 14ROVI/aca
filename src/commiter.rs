use bytes::{Buf, BufMut, BytesMut};

use crate::{
    register_alias_table::{RegisterAliasTable, Tag},
    registers::Registers,
    reorder_buffer::{Destination, ReorderBuffer, RobType},
    reservation_station::ReservationStation,
};

#[derive(Debug)]
pub struct Commiter;
impl Commiter {
    pub fn new() -> Self {
        Self
    }

    pub fn commit_finished(
        &self,
        registers: &mut Registers,
        rat: &mut RegisterAliasTable,
        rob: &mut ReorderBuffer,
        reservation_stations: &mut Vec<ReservationStation>,
        memory: &mut BytesMut,
        should_flush: &mut bool,
    ) {
        for inst in rob.retire() {
            // println!("commit: {:?}", inst.index);
            match inst.destination {
                Destination::Memory(addr) => {
                    (&mut memory[addr..(addr + 4)]).put_i32(inst.value);
                }
                Destination::Reg(reg) => {
                    // load actual value from memory
                    let value = match inst.inst {
                        RobType::LoadMemory => {
                            let addr = inst.value as usize;
                            (&memory[addr..(addr + 4)]).get_i32()
                        }
                        _ => inst.value,
                    };

                    // if not correct branch predict - we do nothing if predicted correctly
                    if !(inst.inst == RobType::Branch && inst.value == -1) {
                        // update value in registers
                        registers.set(reg, value);

                        // propogate to the reservation stations too
                        reservation_stations
                            .iter_mut()
                            .for_each(|rs| rs.update_operands(inst.index, value));

                        // remove rob index as alias for register in rat IF rat points to us still for register!
                        if let Tag::Rob(index) = rat.get(reg) {
                            if index == inst.index {
                                rat.remove(reg);
                            }
                        }
                    } else {
                        *should_flush = true
                    }
                }
                Destination::None => (),
            }
        }
    }
}
