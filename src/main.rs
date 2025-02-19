extern crate num;
#[macro_use]
extern crate num_derive;

use std::env;

mod assembler;
mod cpu;
mod instructions;

use assembler::assemble_file;
use cpu::CPU;
use instructions::Word;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("ERROR: you need a file.acasm argument to run!");
        return;
    }
    let acasm_filename = &args[1];
    if !acasm_filename.ends_with(".acasm") {
        println!("ERROR: you need a file.acasm argument to run!");
        return;
    }

    let (memory, instructions) = assemble_file(acasm_filename);

    let mut simulator = CPU::new();
    simulator.set_memory(memory);
    simulator.run_program(instructions);
}
