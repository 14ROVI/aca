use std::{fs, i32};

use bytes::{Buf, BufMut, BytesMut};

use crate::{
    instructions::{Op, Register},
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
            if inst.op == Op::Exit {
                println!("Program exited with value {}", inst.value.to_value());
                registers.set(Register::ProgramCounter, i32::MAX);
                *should_flush = true;
                break;
            }

            match inst.destination {
                Destination::Memory(addr) => match inst.value {
                    RobValue::Value(value) => {
                        if inst.op == Op::StoreChar {
                            (&mut memory[addr..(addr + 1)]).put_u8(value as u8)
                        } else if inst.op == Op::Save {
                            let len = value as usize;
                            let mut contents = vec![
                                0x50, 0x36, 0x0A, 0x32, 0x35, 0x36, 0x20, 0x32, 0x35, 0x36, 0x0A,
                                0x32, 0x35, 0x35, 0x0A,
                            ];
                            contents.append(&mut memory[addr..(addr + len)].to_vec());
                            fs::write("assets/output.ppm", contents).expect("cant write");
                        } else {
                            (&mut memory[addr..(addr + 4)]).put_i32(value)
                        }
                    }
                    RobValue::Vector(value) => (&mut memory[addr..(addr + 16)]).put_u128(value),
                    _ => panic!("cant set memory on overflow value"),
                },
                Destination::Reg(reg) => {
                    // load actual value from memory
                    if !reg.is_vector() {
                        if let RobValue::Overflow(val1, val2) = inst.value {
                            // we are either multiply or divide. so just set the regs and continue
                            registers.set(Register::High, val1);
                            registers.set(Register::Low, val2);

                            reservation_stations
                                .iter_mut()
                                .for_each(|rs| rs.update_operands(inst.index, inst.value.clone()));

                            if let Tag::Rob(index) = rat.get(Register::High) {
                                if index == inst.index {
                                    rat.remove(Register::High);
                                }
                            }
                            if let Tag::Rob(index) = rat.get(Register::Low) {
                                if index == inst.index {
                                    rat.remove(Register::Low);
                                }
                            }
                            continue;
                        };

                        let mut value = match inst.inst {
                            RobType::LoadMemory => {
                                let addr = inst.value.to_value() as usize;
                                if inst.op == Op::LoadHalfWord {
                                    (&memory[addr..(addr + 2)]).get_u16() as i32
                                } else if inst.op == Op::LoadChar {
                                    (&memory[addr..addr + 1]).get_u8() as i32
                                } else {
                                    (&memory[addr..(addr + 4)]).get_i32()
                                }
                            }
                            _ => inst.value.to_value(),
                        };

                        if inst.op == Op::ReserveMemory {
                            let addr = memory.len();
                            memory.put_bytes(0, inst.value.to_value() as usize);
                            value = addr as i32;
                        }

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
