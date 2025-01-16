extern crate num;
#[macro_use]
extern crate num_derive;

mod cpu;
mod instructions;

use cpu::CPU;
use instructions::Op;

fn main() {
    let instructions = vec![
        Op::load_immediate(1, 100),
        Op::load_immediate(2, 200),
        Op::add(3, 2, 1),
        // Op::subtract(3, 3, 1),
        // Op::subtract(3, 3, 1),
        // Op::subtract(3, 3, 1),
        Op::compare(4, 1, 2),
    ];

    let mut simulator = CPU::new();
    simulator.run_program(instructions);
}
