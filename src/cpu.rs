use bytes::BytesMut;

use crate::branch_prediction::BranchPredictor;
use crate::commiter::Commiter;
use crate::dispatcher::Dispatcher;
use crate::execution_units::{EUType, ExecutionUnit};
use crate::fetcher::Fetcher;
use crate::instructions::{Register, Word};
use crate::register_alias_table::RegisterAliasTable;
use crate::registers::Registers;
use crate::reorder_buffer::ReorderBuffer;
use crate::reservation_station::ReservationStation;

pub struct CPU {
    instructions: Vec<Word>,
    registers: Registers,
    should_flush: bool,
    branch_predictor: BranchPredictor,
    memory: BytesMut,
    rat: RegisterAliasTable,
    rob: ReorderBuffer,
    fetcher: Fetcher,
    dispatcher: Dispatcher,
    reservation_stations: Vec<ReservationStation>,
    execution_units: Vec<ExecutionUnit>,
    commiter: Commiter,
}
impl CPU {
    pub fn new() -> Self {
        CPU {
            instructions: Vec::new(),
            registers: Registers::new(),
            rat: RegisterAliasTable::new(),
            rob: ReorderBuffer::new(32, 8),
            should_flush: false,
            memory: BytesMut::new(),
            branch_predictor: BranchPredictor::new(),
            fetcher: Fetcher::new(8, 8),
            dispatcher: Dispatcher::new(8),
            reservation_stations: vec![
                ReservationStation::new(4, EUType::ALU),
                ReservationStation::new(2, EUType::Branch),
                ReservationStation::new(2, EUType::Memory),
            ],
            execution_units: vec![
                ExecutionUnit::new(EUType::ALU),
                ExecutionUnit::new(EUType::Branch),
                ExecutionUnit::new(EUType::Memory),
            ],
            commiter: Commiter::new(),
        }
    }

    pub fn set_memory(&mut self, memory: BytesMut) {
        self.memory = memory;
    }

    pub fn run_program(&mut self, instructions: Vec<Word>) {
        self.instructions = instructions;
        self.run();
    }

    fn run(&mut self) {
        let mut i = 0;
        while !self.is_finished() || i == 0 || self.should_flush {
            println!("CYCLE {}", i);
            self.should_flush = false;
            self.cycle();
            println!("");
            i += 1;
            if i > 20 {
                break;
            }
        }

        let mut regs = self
            .registers
            .registers
            .iter()
            .filter(|(_, v)| **v != 0)
            .collect::<Vec<(&Register, &i32)>>();
        regs.sort();
        for (reg, value) in regs {
            println!("{:?}: {}", reg, value);
        }

        println!("{:?}", self.memory.to_vec());
    }

    fn cycle(&mut self) {
        // we have to run the cycle thing in reverse
        // because each stage pulls from one one infront
        // so we don't get instructions flying through in one cycle
        // we have 5 stages: fetch, decode/issue, dispatch, execute, commit

        // commit
        self.commiter.commit_finished(
            &mut self.registers,
            &mut self.rat,
            &mut self.rob,
            &mut self.reservation_stations,
            &mut self.memory,
            &mut self.should_flush,
        );

        // hand should flush!
        if self.should_flush {
            self.execution_units.iter_mut().for_each(|eu| eu.flush());
            self.reservation_stations
                .iter_mut()
                .for_each(|rs| rs.flush());
            self.dispatcher.flush();
            self.fetcher.flush();
            self.rob.flush();
            self.rat.flush();
            return;
        }

        // execute
        for eu in self.execution_units.iter_mut() {
            eu.cycle(&mut self.branch_predictor, &mut self.rob);
        }

        // issue
        for eu in self.execution_units.iter_mut() {
            if !eu.is_busy() {
                for rs in self.reservation_stations.iter_mut() {
                    if rs.reserves_for() == eu.flavour {
                        if let Some(rs_inst) = rs.take_oldest_valid() {
                            eu.start(rs_inst.to_exe_inst(), &mut self.rob);
                            break;
                        }
                    }
                }
            }
        }

        // dispatch/decodde
        self.dispatcher.dispatch(
            &mut self.fetcher,
            &mut self.registers,
            &mut self.rat,
            &mut self.rob,
            &mut self.reservation_stations,
        );

        // fetch
        self.fetcher.fetch(
            &self.instructions,
            &mut self.registers,
            &mut self.branch_predictor,
            &mut self.rat,
        );

        self.fetcher
            .buffer
            .iter()
            .for_each(|val| println!("{:?} | branch taken: {:?}", val.word, val.branch_taken));
        // println!("{:?}", self.dispatcher);
        // println!("{:?}", self.reservation_stations);
        // println!("{:?}", self.execution_units);
        // println!("{:?}", self.commiter);

        self.reservation_stations
            .iter()
            .filter(|rs| !rs.is_empty())
            .for_each(|rs| {
                println!("RS {:?}", rs.reserves_for());
                rs.buffer.iter().for_each(|el| {
                    println!(
                        "    {:?} {:?} {:?} {:?} {:?}",
                        el.rob_index,
                        el.word.op(),
                        el.return_op,
                        el.left_op,
                        el.right_op
                    );
                });
            });
        self.execution_units
            .iter()
            .filter(|ex| ex.is_busy())
            .for_each(|ex| {
                println!("Exe {:?}", ex.flavour);
                ex.inst.inspect(|el| {
                    println!(
                        "    {:?} {:?} {:?} {:?} {:?}",
                        el.rob_index,
                        el.word.op(),
                        el.ret,
                        el.left,
                        el.right
                    );
                });
            });

        // println!("{:?}", self.rob.buffer);
        // println!("{:?}", self.rat.table);
    }

    fn is_finished(&mut self) -> bool {
        let mut finished = true;

        finished &= self.rob.is_empty();
        finished &= self.fetcher.get_oldest().is_none();

        return finished;
    }
}
