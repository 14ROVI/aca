extern crate num;
#[macro_use]
extern crate num_derive;

mod cpu;
mod instructions;

use cpu::CPU;
use instructions::Word;

fn main() {
    let mut memory: Vec<i32> = Vec::with_capacity(1024); // 4KiB
    memory.append(&mut vec![1, 1, 1, 1, 1]); // load a
    memory.append(&mut vec![1, 2, 3, 4, 5]); // load b
    memory.append(&mut vec![0, 0, 0, 0, 0]); // load c placeholder

    // let instructions = vec![
    //     Word::load_immediate(1, 100),
    //     Word::load_immediate(2, 200),
    //     Word::add(3, 2, 1),
    //     Word::compare(4, 1, 2),
    //     Word::branch_equal(1, 2, 3), // if equal go +3
    //     Word::load_immediate(5, 1),
    //     Word::jump_immediate(9), // jumps past end of program, ends program
    //     Word::load_immediate(6, 1),
    // ];

    let vector_add = vec![
        Word::load_immediate(1, 0),            // * to a
        Word::load_immediate(2, 5),            // * to b
        Word::load_immediate(3, 10),           // * to c
        Word::load_immediate(4, 0),            // inital value of i
        Word::load_immediate(5, 5),            // len of vectors
        Word::branch_greater_equal(4, 5, 100), // go to end of loop, were done
        //loop content
        Word::add(6, 1, 4),           // index of a[i]
        Word::add(7, 2, 4),           // index of b[i]
        Word::add(8, 3, 4),           // index of c[i]
        Word::load_memory(9, 6, 0),   // load a[i] into 9
        Word::load_memory(10, 7, 0),  // load b[i] into 10
        Word::add(11, 9, 10),         // c[i] is in 11
        Word::store_memory(11, 8, 0), // store whats in 11 into memory addr (val of 8)
        // increment i
        Word::add_immediate(4, 4, 1),
        // jump to loop beginning again
        Word::jump_immediate(5),
    ];

    let mut simulator = CPU::new();
    simulator.set_memory(memory);
    simulator.run_program(vector_add);
}
