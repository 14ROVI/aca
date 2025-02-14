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
    LoadMemory,
    StoreMemory,
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
    pub fn is_alu(&self) -> bool {
        match self {
            Op::Add | Op::AddImmediate | Op::Compare | Op::Subtract | Op::SubtractImmediate => true,
            _ => false,
        }
    }

    pub fn is_predictable_branch(&self) -> bool {
        match self {
            Op::BranchEqual
            | Op::BranchNotEqual
            | Op::BranchGreater
            | Op::BranchGreaterEqual
            | Op::BranchLess
            | Op::BranchLessEqual => true,
            _ => false,
        }
    }

    pub fn is_branch(&self) -> bool {
        match self {
            Op::JumpRegister | Op::Jump => true,
            _ => self.is_predictable_branch(),
        }
    }

    pub fn is_memory(&self) -> bool {
        match self {
            Op::LoadImmediate | Op::LoadMemory | Op::StoreMemory => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Word {
    R(Op, Register, Register, Register), // op, ro, rl, rr
    I(Op, Register, Register, i32),      // op, ro, rl, immediate
    JI(Op, i32),                         // op, immediate value
    JR(Op, Register),                    // op, register containing jump value
}
impl Word {
    pub fn op(&self) -> Op {
        match self {
            Word::R(op, _, _, _) => *op,
            Word::I(op, _, _, _) => *op,
            Word::JI(op, _) => *op,
            Word::JR(op, _) => *op,
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

    pub fn load_memory(ro: u32, address: u32, offset: i32) -> Word {
        Word::I(
            Op::LoadMemory,
            Register::g(ro),
            Register::g(address),
            offset,
        )
    }

    pub fn store_memory(ri: u32, address: u32, offset: i32) -> Word {
        Word::I(
            Op::StoreMemory,
            Register::g(ri),
            Register::g(address),
            offset,
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

    pub fn branch_less(rr: u32, rl: u32, relative: i32) -> Word {
        Word::I(Op::BranchLess, Register::g(rr), Register::g(rl), relative)
    }

    pub fn branch_greater_equal(rr: u32, rl: u32, relative: i32) -> Word {
        Word::I(
            Op::BranchGreaterEqual,
            Register::g(rr),
            Register::g(rl),
            relative,
        )
    }

    pub fn jump_immediate(absolute: i32) -> Word {
        Word::JI(Op::Jump, absolute)
    }

    pub fn jump_reg(r: u32) -> Word {
        Word::JR(Op::JumpRegister, Register::g(r))
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
