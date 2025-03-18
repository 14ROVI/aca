mod assembler;
mod branch_prediction;
mod commiter;
mod cpu;
mod dispatcher;
mod execution_units;
mod fetcher;
mod instructions;
// mod memory;
mod register_alias_table;
mod registers;
mod reorder_buffer;
mod reservation_station;
mod stats;

use assembler::assemble_file;
use branch_prediction::BranchPredictionMode;
use cpu::CPU;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    pub acasm_filename: String,

    #[arg(short, long, default_value_t = false)]
    pub print_memory: bool,

    #[arg(short, long, default_value_t = 32)]
    pub rob_size: usize,

    #[arg(short, long, default_value_t = 8)]
    pub rob_max_retire: usize,

    #[arg(short, long, default_value_t = 8)]
    pub fetch_amount: usize,

    #[arg(short, long, default_value_t = 8)]
    pub fetch_buffer_capacity: usize,

    #[arg(short, long, default_value_t = 8)]
    pub dispatch_amount: usize,

    #[arg(short, long, default_value_t = 6)]
    pub rs_alu_size: usize,
    #[arg(short, long, default_value_t = 4)]
    pub rs_fpu_size: usize,
    #[arg(short, long, default_value_t = 2)]
    pub rs_vpu_size: usize,
    #[arg(short, long, default_value_t = 2)]
    pub rs_lsu_size: usize,
    #[arg(short, long, default_value_t = 2)]
    pub rs_branch_size: usize,

    #[arg(short, long, default_value_t = 3)]
    pub eu_alu_num: usize,
    #[arg(short, long, default_value_t = 2)]
    pub eu_fpu_num: usize,
    #[arg(short, long, default_value_t = 1)]
    pub eu_vpu_num: usize,
    #[arg(short, long, default_value_t = 1)]
    pub eu_lsu_num: usize,
    #[arg(short, long, default_value_t = 1)]
    pub eu_branch_num: usize,

    #[arg(short, long)]
    pub branch_predictor_mode: Option<BranchPredictionMode>,
}

fn main() {
    let args = Args::parse();
    let (memory, instructions) = assemble_file(&args.acasm_filename);

    let mut simulator = CPU::new(args.into());
    simulator.set_memory(memory);
    let stats = simulator.run_program(instructions);

    println!("{}", stats);
}
