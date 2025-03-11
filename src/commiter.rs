use bytes::{Buf, BufMut, BytesMut};

use crate::{
    register_alias_table::{RegisterAliasTable, Tag},
    registers::Registers,
    reorder_buffer::{Destination, ReorderBuffer, RobType, RobValue},
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
                Destination::Memory(addr) => match inst.value {
                    RobValue::Value(value) => (&mut memory[addr..(addr + 4)]).put_i32(value),
                    RobValue::Vector(value) => (&mut memory[addr..(addr + 16)]).put_u128(value),
                },
                Destination::Reg(reg) => {
                    // load actual value from memory
                    if !reg.is_vector() {
                        let value = match inst.inst {
                            RobType::LoadMemory => {
                                let addr = inst.value.to_value() as usize;
                                (&memory[addr..(addr + 4)]).get_i32()
                            }
                            _ => inst.value.to_value(),
                        };

                        // if not correct branch predict - we do nothing if predicted correctly
                        if !(inst.inst == RobType::Branch && value == -1) {
                            // update value in registers
                            registers.set(reg, value);

                            // propogate to the reservation stations too
                            reservation_stations.iter_mut().for_each(|rs| {
                                rs.update_operands(inst.index, RobValue::Value(value))
                            });

                            // remove rob index as alias for register in rat IF rat points to us still for register!
                            if let Tag::Rob(index) = rat.get(reg) {
                                if index == inst.index {
                                    rat.remove(reg);
                                }
                            }
                        }

                        if inst.inst == RobType::Branch && value != -1 {
                            println!("FLUSHED");
                            *should_flush = true;
                            break;
                        }
                    } else if reg.is_vector() {
                        let value = match inst.inst {
                            RobType::LoadMemory => {
                                let addr = inst.value.to_value() as usize;
                                (&memory[addr..(addr + 16)]).get_u128()
                            }
                            _ => inst.value.to_vector(),
                        };

                        // update value in registers
                        registers.set_vector(reg, value);

                        // propogate to the reservation stations too
                        reservation_stations
                            .iter_mut()
                            .for_each(|rs| rs.update_operands(inst.index, RobValue::Vector(value)));

                        // remove rob index as alias for register in rat IF rat points to us still for register!
                        if let Tag::Rob(index) = rat.get(reg) {
                            if index == inst.index {
                                rat.remove(reg);
                            }
                        }
                    }
                }
                Destination::None => (),
            }
        }
    }
}
