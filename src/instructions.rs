#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
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

#[derive(Debug, Copy, Clone, FromPrimitive)]
pub enum Op {
    LoadImmediate,
    Add,
    AddImmediate,
    Subtract,
    SubtractImmediate,
    Compare,
    JumpRelative,
    JumpAbsolute,
}

#[derive(Debug, Clone, Copy)]
pub enum Word {
    R(Op, Register, Register, Register), // op, ro, rl, rr
    I(Op, Register, Register, i32),      // op, ro, rl, immediate
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

    pub fn jump_relative(rl: u32, immediate: i32) -> Word {
        Word::I(Op::JumpRelative, Register::pc(), Register::g(rl), immediate)
    }

    pub fn jump_absolute(rl: u32, immediate: i32) -> Word {
        Word::I(Op::JumpAbsolute, Register::pc(), Register::g(rl), immediate)
    }
}
