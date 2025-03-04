extern crate num;
#[macro_use]
extern crate num_derive;

use std::env;

mod assembler;
mod cpu;
mod instructions;

use assembler::assemble_file;
use cpu::CPU;

#[derive(Debug)]
struct GetFilenameError;

fn main() {
    let acasm_filename = get_filename().expect("Please add a file.acasm argument to run!");

    let (memory, instructions) = assemble_file(&acasm_filename);

    let mut simulator = CPU::new();
    simulator.set_memory(memory);
    println!("{:?}", &instructions);
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
