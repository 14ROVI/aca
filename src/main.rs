mod assembler;
mod branch_prediction;
mod commiter;
mod cpu;
mod dispatcher;
mod execution_units;
mod fetcher;
mod instructions;
mod register_alias_table;
mod registers;
mod reorder_buffer;
mod reservation_station;

use assembler::assemble_file;
use cpu::CPU;

use std::env;

#[derive(Debug)]
struct GetFilenameError;

fn main() {
    let acasm_filename = get_filename().expect("Please add a file.acasm argument to run!");

    let (memory, instructions) = assemble_file(&acasm_filename);

    let mut simulator = CPU::new();
    simulator.set_memory(memory);
    simulator.run_program(instructions);
}

fn get_filename() -> Result<String, GetFilenameError> {
    let args: Vec<String> = env::args().collect();
    if args.len() >= 2 && args[1].ends_with(".acasm") {
        return Ok(args[1].clone());
    } else {
        return Err(GetFilenameError);
    }
}
