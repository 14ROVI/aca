extern crate num;
#[macro_use]
extern crate num_derive;

mod cpu;
mod instructions;

use cpu::CPU;
use instructions::Word;

fn main() {
    let instructions = vec![
        Word::load_immediate(1, 100),
        Word::load_immediate(2, 200),
        Word::add(3, 2, 1),
        Word::compare(4, 1, 2),
        // Word::jump_absolute(0, 0),
    ];

    let mut simulator = CPU::new();
    simulator.run_program(instructions);
}
