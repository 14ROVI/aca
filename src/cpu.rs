use bytes::BytesMut;

use crate::branch_prediction::{BranchPredictionMode, CoreBranchPredictor};
use crate::commiter::Commiter;
use crate::dispatcher::Dispatcher;
use crate::execution_units::{EUType, ExecutionUnit};
use crate::fetcher::Fetcher;
use crate::instructions::{Register, Word};
use crate::register_alias_table::RegisterAliasTable;
use crate::registers::Registers;
use crate::reorder_buffer::ReorderBuffer;
use crate::reservation_station::ReservationStation;
use crate::stats::StatsTracker;

pub struct CPU {
    instructions: Vec<Word>,
    registers: Registers,
    should_flush: bool,
    branch_predictor: CoreBranchPredictor,
    memory: BytesMut,
    rat: RegisterAliasTable,
    rob: ReorderBuffer,
    fetcher: Fetcher,
    dispatcher: Dispatcher,
    reservation_stations: Vec<ReservationStation>,
    execution_units: Vec<ExecutionUnit>,
    commiter: Commiter,
    stats_tracker: StatsTracker,
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
            branch_predictor: CoreBranchPredictor::new(BranchPredictionMode::TwoBitSaturating),
            fetcher: Fetcher::new(8, 8),
            dispatcher: Dispatcher::new(8),
            reservation_stations: vec![
                ReservationStation::new(16, EUType::ALU),
                ReservationStation::new(4, EUType::FPU),
                ReservationStation::new(1, EUType::VPU),
                ReservationStation::new(1, EUType::System),
                ReservationStation::new(4, EUType::Branch),
                ReservationStation::new(4, EUType::Memory),
            ],
            execution_units: vec![
                ExecutionUnit::new(EUType::ALU),
                ExecutionUnit::new(EUType::ALU),
                ExecutionUnit::new(EUType::ALU),
                ExecutionUnit::new(EUType::ALU),
                ExecutionUnit::new(EUType::FPU),
                ExecutionUnit::new(EUType::VPU),
                ExecutionUnit::new(EUType::System),
                ExecutionUnit::new(EUType::Branch),
                ExecutionUnit::new(EUType::Branch),
                ExecutionUnit::new(EUType::Memory),
                ExecutionUnit::new(EUType::Memory),
                ExecutionUnit::new(EUType::Memory),
                ExecutionUnit::new(EUType::Memory),
            ],
            commiter: Commiter::new(),
            stats_tracker: StatsTracker::new(),
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
        while !self.is_finished() || self.stats_tracker.cycles == 0 || self.should_flush {
            self.should_flush = false;
            self.cycle();
            self.stats_tracker.cycles += 1;
            // self.print_dbg();
            // if self.stats_tracker.cycles >= 10 {
            // return;
            // }
        }

        self.print_end_dbg();
        println!("{}", self.stats_tracker);
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
            &mut self.stats_tracker,
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
            eu.cycle(
                &mut self.branch_predictor,
                &mut self.rob,
                &mut self.reservation_stations,
                &mut self.memory,
            );
        }

        // issue
        for eu in self.execution_units.iter_mut() {
            if !eu.is_busy() {
                for rs in self.reservation_stations.iter_mut() {
                    if rs.reserves_for() == eu.flavour {
                        if let Some(rs_inst) = rs.take_oldest_valid(&mut self.rob) {
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
            &mut self.stats_tracker,
        );

        // fetch
        self.fetcher.fetch(
            &self.instructions,
            &mut self.registers,
            &mut self.branch_predictor,
            &mut self.rat,
            &mut self.stats_tracker,
        );
    }

    fn is_finished(&mut self) -> bool {
        let mut finished = true;

        finished &= self.rob.is_empty();
        finished &= self.fetcher.get_oldest().is_none();

        return finished;
    }

    #[allow(dead_code)]
    fn print_dbg(&mut self) {
        self.fetcher
            .buffer
            .iter()
            .for_each(|val| println!("{:?} | branch taken: {:?}", val.word, val.branch_taken));

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

        // let mut regs = self
        //     .registers
        //     .general_registers
        //     .iter()
        //     .filter(|(_, v)| **v != 0)
        //     .collect::<Vec<(&Register, &i32)>>();
        // regs.sort();
        // for (reg, value) in regs {
        //     print!("{:?}: {}  ", reg, value);
        // }
        println!();
    }

    #[allow(dead_code)]
    fn print_end_dbg(&mut self) {
        // println!("{:02X?}", self.memory.to_vec());

        let mut regs = self
            .registers
            .general_registers
            .iter()
            .filter(|(_, v)| **v != 0)
            .collect::<Vec<(&Register, &i32)>>();
        regs.sort();
        for (reg, value) in regs {
            println!(
                "{:?}: i32({}) f32({})",
                reg,
                value,
                f32::from_be_bytes(value.to_be_bytes())
            );
        }

        let mut regs = self
            .registers
            .vector_registers
            .iter()
            // .filter(|(_, v)| **v != 0)
            .collect::<Vec<(&Register, &u128)>>();
        regs.sort();
        for (reg, value) in regs {
            let floats: Vec<f32> = value
                .to_be_bytes()
                .chunks_exact(4)
                .map(|i| [i[0], i[1], i[2], i[3]])
                .map(|i| f32::from_be_bytes(i))
                .collect();

            println!("{:?}: u128({}) f32({:?})", reg, value, floats,);
        }
    }
}
