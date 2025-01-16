#[derive(Debug, Copy, Clone, FromPrimitive)]
pub enum Op {
    LoadImmediate,
    Add,
    AddImmediate,
    Subtract,
    SubtractImmediate,
    Compare,
}
impl Op {
    pub fn load_immediate(ro: u32, immediate: i32) -> Word {
        Word::I(Op::LoadImmediate, ro, 0, immediate)
    }

    pub fn add(ro: u32, rl: u32, rr: u32) -> Word {
        Word::R(Op::Add, ro, rl, rr, 0, 0)
    }

    pub fn add_immediate(ro: u32, rl: u32, immediate: i32) -> Word {
        Word::I(Op::AddImmediate, ro, rl, immediate)
    }

    pub fn subtract(ro: u32, rl: u32, rr: u32) -> Word {
        Word::R(Op::Subtract, ro, rl, rr, 0, 0)
    }

    pub fn subtract_immediate(ro: u32, rl: u32, immediate: i32) -> Word {
        Word::I(Op::SubtractImmediate, ro, rl, immediate)
    }

    pub fn compare(ro: u32, rl: u32, rr: u32) -> Word {
        Word::R(Op::Compare, ro, rl, rr, 0, 0)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Word {
    R(Op, u32, u32, u32, u32, u32), // op, ro, rl, rr, shamr, funct
    I(Op, u32, u32, i32),           // op, ro, rl, immediate
    J(Op, u32),                     // op, address
}
impl Word {
    pub fn op(&self) -> Op {
        match self {
            Word::R(op, _, _, _, _, _) => *op,
            Word::I(op, _, _, _) => *op,
            Word::J(op, _) => *op,
        }
    }
}
