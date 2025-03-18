use bytes::BytesMut;

use crate::branch_prediction::{BranchPredictionMode, BranchPredictor, CoreBranchPredictor};
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
use crate::Args;

pub struct CpuConfig {
    pub rob_size: usize,
    pub rob_max_retire: usize,
    pub fetch_amount: usize,
    pub fetch_buffer_capacity: usize,
    pub dispatch_amount: usize,
    pub rs_alu_size: usize,
    pub rs_fpu_size: usize,
    pub rs_vpu_size: usize,
    pub rs_lsu_size: usize,
    pub rs_branch_size: usize,
    pub eu_alu_num: usize,
    pub eu_fpu_num: usize,
    pub eu_vpu_num: usize,
    pub eu_lsu_num: usize,
    pub eu_branch_num: usize,
    pub branch_predictor_mode: BranchPredictionMode,
    pub print_memory: bool,
}
impl From<Args> for CpuConfig {
    fn from(value: Args) -> Self {
        Self {
            rob_size: value.rob_size,
            rob_max_retire: value.rob_max_retire,
            fetch_amount: value.fetch_amount,
            fetch_buffer_capacity: value.fetch_buffer_capacity,
            dispatch_amount: value.dispatch_amount,
            rs_alu_size: value.rs_alu_size,
            rs_fpu_size: value.rs_fpu_size,
            rs_vpu_size: value.rs_vpu_size,
            rs_lsu_size: value.rs_lsu_size,
            rs_branch_size: value.rs_branch_size,
            eu_alu_num: value.eu_alu_num,
            eu_fpu_num: value.eu_fpu_num,
            eu_vpu_num: value.eu_vpu_num,
            eu_lsu_num: value.eu_lsu_num,
            eu_branch_num: value.eu_branch_num,
            branch_predictor_mode: value
                .branch_predictor_mode
                .unwrap_or(BranchPredictionMode::TwoBitSaturating),
            print_memory: value.print_memory,
        }
    }
}

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
    config: CpuConfig,
}
impl CPU {
    pub fn new(config: CpuConfig) -> Self {
        let mut execution_units = vec![ExecutionUnit::new(EUType::System)];
        execution_units.append(&mut vec![
            ExecutionUnit::new(EUType::ALU);
            config.eu_alu_num
        ]);
        execution_units.append(&mut vec![
            ExecutionUnit::new(EUType::FPU);
            config.eu_fpu_num
        ]);
        execution_units.append(&mut vec![
            ExecutionUnit::new(EUType::VPU);
            config.eu_vpu_num
        ]);
        execution_units.append(&mut vec![
            ExecutionUnit::new(EUType::Branch);
            config.eu_branch_num
        ]);
        execution_units.append(&mut vec![
            ExecutionUnit::new(EUType::Memory);
            config.eu_lsu_num
        ]);

        CPU {
            instructions: Vec::new(),
            registers: Registers::new(),
            rat: RegisterAliasTable::new(),
            rob: ReorderBuffer::new(config.rob_size, config.rob_max_retire),
            should_flush: false,
            memory: BytesMut::new(),
            branch_predictor: CoreBranchPredictor::new(config.branch_predictor_mode.clone()),
            fetcher: Fetcher::new(config.fetch_amount, config.fetch_buffer_capacity),
            dispatcher: Dispatcher::new(config.dispatch_amount),
            reservation_stations: vec![
                ReservationStation::new(config.rs_alu_size, EUType::ALU),
                ReservationStation::new(config.rs_fpu_size, EUType::FPU),
                ReservationStation::new(config.rs_vpu_size, EUType::VPU),
                ReservationStation::new(config.rs_branch_size, EUType::Branch),
                ReservationStation::new(config.rs_lsu_size, EUType::Memory),
                ReservationStation::new(1, EUType::System),
            ],
            execution_units,
            commiter: Commiter::new(),
            stats_tracker: StatsTracker::new(),
            config,
        }
    }

    pub fn set_memory(&mut self, memory: BytesMut) {
        self.memory = memory;
    }

    pub fn run_program(&mut self, instructions: Vec<Word>) -> StatsTracker {
        self.instructions = instructions;
        self.run();
        return self.stats_tracker;
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
            // println!("{:?}", self.rob.buffer);
        }

        self.print_end_dbg();
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
            &mut self.branch_predictor,
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
            self.branch_predictor.flush();
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

        println!("{:?}", self.rob.buffer);
        println!("{:?}", self.rat.table);

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
        if self.config.print_memory {
            println!("{:?}", self.memory.to_vec());
        }

        println!();
        println!();
    }

    #[allow(dead_code)]
    fn print_end_dbg(&mut self) {
        if self.config.print_memory {
            println!("{:?}", self.memory.to_vec());
        }

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
