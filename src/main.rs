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
        Word::branch_equal(1, 1, 3),
        Word::load_immediate(5, 1),
        Word::load_immediate(6, 1),
        Word::load_immediate(7, 1),
    ];

    let mut simulator = CPU::new();
    simulator.run_program(instructions);
}
