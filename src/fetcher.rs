use std::collections::VecDeque;

use crate::{
    branch_prediction::BranchPredictor,
    instructions::{Op, Register, Word},
    register_alias_table::{RegisterAliasTable, Tag},
    registers::Registers,
};

#[derive(Debug)]
pub struct FetchedWord {
    pub word: Word,
    pub branch_taken: bool,
    pub pc: usize,
}

/// Fetches instructions from the instruction memeory every cycle. Contains the instruction
/// buffer.
#[derive(Debug)]
pub struct Fetcher {
    fetch_amount: usize,
    pub buffer: VecDeque<FetchedWord>,
    buffer_capacity: usize,
}
impl Fetcher {
    /// Creates a new Fetcher that can fetch at least `fetch_amount` per cycle and holds `buffer_capacity` instructions in a buffer.
    pub fn new(fetch_amount: usize, buffer_capacity: usize) -> Self {
        Self {
            fetch_amount,
            buffer: VecDeque::new(),
            buffer_capacity,
        }
    }

    pub fn flush(&mut self) {
        *self = Self::new(self.fetch_amount, self.buffer_capacity);
    }

    fn fetch_one(
        &mut self,
        instructions: &Vec<Word>,
        registers: &mut Registers,
        branch_predictor: &mut BranchPredictor,
        rat: &RegisterAliasTable,
    ) {
        let pc = registers.pc();

        // We are past the end of the program, we've finished executing!
        if pc >= instructions.len() {
            return;
        }

        let word = instructions[pc];
        let mut branch_taken = false;

        if word.op().is_predictable_branch() && branch_predictor.predict(pc) {
            // branch prediction
            if let Word::I(_, _, _, immediate) = word {
                let branch = (pc as i32) + immediate;
                registers.set(Register::ProgramCounter, branch);
                branch_taken = true;
            }
        } else if let Word::JI(Op::Jump, val) = word {
            registers.set(Register::ProgramCounter, val);
            return; // this instruction is now not needed to go though the processor!
        } else if let Word::JR(Op::Jump, reg) = word {
            if let Tag::Register(_) = rat.get(reg) {
                let val = registers.get(reg);
                registers.set(Register::ProgramCounter, val);
                return; // this instruction is now not needed to go though the processor!
            }
        } else if let Word::I(Op::JumpAndLink, _, _, immediate) = word {
            registers.set(Register::ProgramCounter, immediate);
            branch_taken = true;
        } else {
            // normal incrememnt
            registers.inc_pc();
        }

        let fetched_word = FetchedWord {
            word,
            pc,
            branch_taken,
        };

        self.buffer.push_back(fetched_word);
    }

    pub fn fetch(
        &mut self,
        instructions: &Vec<Word>,
        registers: &mut Registers,
        branch_predictor: &mut BranchPredictor,
        rat: &RegisterAliasTable,
    ) {
        let num_to_fetch = self
            .fetch_amount
            .min(self.buffer_capacity - self.buffer.len());

        for _ in 0..num_to_fetch {
            self.fetch_one(instructions, registers, branch_predictor, rat);
        }
    }

    pub fn get_oldest(&mut self) -> Option<&FetchedWord> {
        self.buffer.front()
    }

    pub fn take_oldest(&mut self) -> Option<FetchedWord> {
        self.buffer.pop_front()
    }
}
