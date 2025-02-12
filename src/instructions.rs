#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub enum Register {
    ProgramCounter,
    General(u32),
}
impl Register {
    fn pc() -> Self {
        Self::ProgramCounter
    }

    fn g(r: u32) -> Self {
        Self::General(r)
    }
}

#[derive(Debug, Copy, Clone, FromPrimitive, PartialEq)]
pub enum Op {
    NoOp,
    LoadImmediate,
    Add,
    AddImmediate,
    Subtract,
    SubtractImmediate,
    Compare,
    BranchEqual,
    BranchNotEqual,
    BranchGreater,
    BranchGreaterEqual,
    BranchLess,
    BranchLessEqual,
    Jump,
    JumpRegister,
    // JumpAndLink, will add when i add return address register for function call like things!
}
impl Op {
    pub fn is_predictable_branch(&self) -> bool {
        match self {
            Op::BranchEqual
            | Op::BranchNotEqual
            | Op::BranchGreater
            | Op::BranchGreaterEqual
            | Op::BranchLess
            | Op::BranchLessEqual
            | Op::Jump => true,
            _ => false,
        }
    }

    pub fn is_branch(&self) -> bool {
        match self {
            Op::JumpRegister => true,
            _ => self.is_predictable_branch(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Word {
    R(Op, Register, Register, Register), // op, ro, rl, rr
    I(Op, Register, Register, i32),      // op, ro, rl, immediate
                                         // J(Op, i32),                          // op, (value or register depending on op!)
}
impl Word {
    pub fn op(&self) -> Op {
        match self {
            Word::R(op, _, _, _) => *op,
            Word::I(op, _, _, _) => *op,
        }
    }

    pub fn load_immediate(ro: u32, immediate: i32) -> Word {
        Word::I(
            Op::LoadImmediate,
            Register::g(ro),
            Register::g(0),
            immediate,
        )
    }

    pub fn add(ro: u32, rl: u32, rr: u32) -> Word {
        Word::R(Op::Add, Register::g(ro), Register::g(rl), Register::g(rr))
    }

    pub fn add_immediate(ro: u32, rl: u32, immediate: i32) -> Word {
        Word::I(
            Op::AddImmediate,
            Register::g(ro),
            Register::g(rl),
            immediate,
        )
    }

    pub fn subtract(ro: u32, rl: u32, rr: u32) -> Word {
        Word::R(
            Op::Subtract,
            Register::g(ro),
            Register::g(rl),
            Register::g(rr),
        )
    }

    pub fn subtract_immediate(ro: u32, rl: u32, immediate: i32) -> Word {
        Word::I(
            Op::SubtractImmediate,
            Register::g(ro),
            Register::g(rl),
            immediate,
        )
    }

    pub fn compare(ro: u32, rl: u32, rr: u32) -> Word {
        Word::R(
            Op::Compare,
            Register::g(ro),
            Register::g(rl),
            Register::g(rr),
        )
    }

    pub fn branch_equal(rr: u32, rl: u32, relative: i32) -> Word {
        Word::I(Op::BranchEqual, Register::g(rr), Register::g(rl), relative)
    }

    pub fn branch_not_equal(rr: u32, rl: u32, relative: i32) -> Word {
        Word::I(
            Op::BranchNotEqual,
            Register::g(rr),
            Register::g(rl),
            relative,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Instruction {
    pub word: Word,
    pub pc: usize,
    pub branch_taken: bool,
}
impl Instruction {
    pub fn new(word: Word, pc: usize, branch_taken: bool) -> Self {
        Instruction {
            word,
            pc,
            branch_taken,
        }
    }
}
