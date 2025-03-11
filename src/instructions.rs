use crate::{execution_units::EUType, reorder_buffer::RobType};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub enum Register {
    ProgramCounter,
    General(u32),
    Vector(u32),
}
impl Register {
    pub fn g(r: u32) -> Self {
        Self::General(r)
    }

    pub fn v(r: u32) -> Self {
        Self::Vector(r)
    }

    pub fn is_vector(&self) -> bool {
        match self {
            Self::Vector(_) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Op {
    LoadImmediate,
    LoadMemory,
    StoreMemory,
    Add,
    AddImmediate,
    Subtract,
    SubtractImmediate,
    Multiply,
    MultiplyNoOverflow,
    Divide,
    Compare,
    BitAnd,
    BitAndImmediate,
    BitOr,
    BitOrImmediate,
    LeftShift,
    RightShift,
    BranchEqual,
    BranchNotEqual,
    BranchGreater,
    BranchGreaterEqual,
    BranchLess,
    BranchLessEqual,
    Jump,
    JumpRegister,
    FLoadImmediate,
    FAdd,
    FAddImmediate,
    FSubtract,
    FSubtractImmediate,
    FMultiply,
    FDivide,
    FCompare,
    VLoadMemory,
    VStoreMemory,
    VLeftShift,
    VRightShift,
    VAdd,
    VSubtract,
    VMultiply,
    VDivide,
    VFAdd,
    VFSubtract,
    VFMultiply,
    VFDivide,
}
impl Op {
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

    pub fn rob_type(&self) -> RobType {
        match self {
            Op::LoadImmediate => RobType::Register,
            Op::LoadMemory => RobType::LoadMemory,
            Op::StoreMemory => RobType::StoreMemory,
            Op::Add => RobType::Register,
            Op::AddImmediate => RobType::Register,
            Op::Subtract => RobType::Register,
            Op::SubtractImmediate => RobType::Register,
            Op::Multiply => RobType::Register,
            Op::MultiplyNoOverflow => RobType::Register,
            Op::Divide => RobType::Register,
            Op::Compare => RobType::Register,
            Op::BitAnd => RobType::Register,
            Op::BitAndImmediate => RobType::Register,
            Op::BitOr => RobType::Register,
            Op::BitOrImmediate => RobType::Register,
            Op::LeftShift => RobType::Register,
            Op::RightShift => RobType::Register,
            Op::BranchEqual => RobType::Branch,
            Op::BranchNotEqual => RobType::Branch,
            Op::BranchGreater => RobType::Branch,
            Op::BranchGreaterEqual => RobType::Branch,
            Op::BranchLess => RobType::Branch,
            Op::BranchLessEqual => RobType::Branch,
            Op::Jump => RobType::Branch,
            Op::JumpRegister => RobType::Branch,
            Op::FLoadImmediate => RobType::Register,
            Op::FAdd => RobType::Register,
            Op::FAddImmediate => RobType::Register,
            Op::FSubtract => RobType::Register,
            Op::FSubtractImmediate => RobType::Register,
            Op::FMultiply => RobType::Register,
            Op::FDivide => RobType::Register,
            Op::FCompare => RobType::Register,
            Op::VLoadMemory => RobType::LoadMemory,
            Op::VStoreMemory => RobType::StoreMemory,
            Op::VLeftShift => RobType::Register,
            Op::VRightShift => RobType::Register,
            Op::VAdd => RobType::Register,
            Op::VSubtract => RobType::Register,
            Op::VMultiply => RobType::Register,
            Op::VDivide => RobType::Register,
            Op::VFAdd => RobType::Register,
            Op::VFSubtract => RobType::Register,
            Op::VFMultiply => RobType::Register,
            Op::VFDivide => RobType::Register,
        }
    }

    pub fn needs_eu_type(&self) -> EUType {
        match self {
            Op::LoadImmediate => EUType::Memory,
            Op::LoadMemory => EUType::Memory,
            Op::StoreMemory => EUType::Memory,
            Op::Add => EUType::ALU,
            Op::AddImmediate => EUType::ALU,
            Op::Subtract => EUType::ALU,
            Op::SubtractImmediate => EUType::ALU,
            Op::Multiply => EUType::ALU,
            Op::MultiplyNoOverflow => EUType::ALU,
            Op::Divide => EUType::ALU,
            Op::Compare => EUType::ALU,
            Op::BitAnd => EUType::ALU,
            Op::BitAndImmediate => EUType::ALU,
            Op::BitOr => EUType::ALU,
            Op::BitOrImmediate => EUType::ALU,
            Op::LeftShift => EUType::ALU,
            Op::RightShift => EUType::ALU,
            Op::BranchEqual => EUType::Branch,
            Op::BranchNotEqual => EUType::Branch,
            Op::BranchGreater => EUType::Branch,
            Op::BranchGreaterEqual => EUType::Branch,
            Op::BranchLess => EUType::Branch,
            Op::BranchLessEqual => EUType::Branch,
            Op::Jump => EUType::Branch,
            Op::JumpRegister => EUType::Branch,
            Op::FLoadImmediate => EUType::Memory,
            Op::FAdd => EUType::FPU,
            Op::FAddImmediate => EUType::FPU,
            Op::FSubtract => EUType::FPU,
            Op::FSubtractImmediate => EUType::FPU,
            Op::FMultiply => EUType::FPU,
            Op::FDivide => EUType::FPU,
            Op::FCompare => EUType::FPU,
            Op::VLoadMemory => EUType::Memory,
            Op::VStoreMemory => EUType::Memory,
            Op::VLeftShift => EUType::VPU,
            Op::VRightShift => EUType::VPU,
            Op::VAdd => EUType::VPU,
            Op::VSubtract => EUType::VPU,
            Op::VMultiply => EUType::VPU,
            Op::VDivide => EUType::VPU,
            Op::VFAdd => EUType::VPU,
            Op::VFSubtract => EUType::VPU,
            Op::VFMultiply => EUType::VPU,
            Op::VFDivide => EUType::VPU,
        }
    }

    pub fn cycles_needed(&self) -> usize {
        match self {
            Op::LoadImmediate => 1,
            Op::LoadMemory => 2,
            Op::StoreMemory => 2,
            Op::Add => 1,
            Op::AddImmediate => 1,
            Op::Subtract => 1,
            Op::SubtractImmediate => 1,
            Op::Multiply => 3,
            Op::MultiplyNoOverflow => 3,
            Op::Divide => 5,
            Op::Compare => 1,
            Op::BitAnd => 1,
            Op::BitAndImmediate => 1,
            Op::BitOr => 1,
            Op::BitOrImmediate => 1,
            Op::LeftShift => 1,
            Op::RightShift => 1,
            Op::BranchEqual => 2,
            Op::BranchNotEqual => 2,
            Op::BranchGreater => 2,
            Op::BranchGreaterEqual => 2,
            Op::BranchLess => 2,
            Op::BranchLessEqual => 2,
            Op::Jump => 1,
            Op::JumpRegister => 1,
            Op::FLoadImmediate => 1,
            Op::FAdd => 2,
            Op::FAddImmediate => 2,
            Op::FSubtract => 2,
            Op::FSubtractImmediate => 2,
            Op::FMultiply => 4,
            Op::FDivide => 6,
            Op::FCompare => 2,
            Op::VLoadMemory => 4,
            Op::VStoreMemory => 4,
            Op::VLeftShift => 3,
            Op::VRightShift => 3,
            Op::VAdd => 3,
            Op::VSubtract => 3,
            Op::VMultiply => 6,
            Op::VDivide => 7,
            Op::VFAdd => 3,
            Op::VFSubtract => 6,
            Op::VFMultiply => 7,
            Op::VFDivide => 3,
        }
    }

    pub fn updates_rat(&self) -> bool {
        match self.rob_type() {
            RobType::Register => true,
            RobType::Branch => false,
            RobType::LoadMemory => true,
            RobType::StoreMemory => false,
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

    pub fn multiply(ro: u32, rl: u32, rr: u32) -> Word {
        Word::R(
            Op::Multiply,
            Register::g(ro),
            Register::g(rl),
            Register::g(rr),
        )
    }

    pub fn multiply_no_overflow(ro: u32, rl: u32, rr: u32) -> Word {
        // sets to to sig bits, insig bits to ro + 1
        Word::R(
            Op::MultiplyNoOverflow,
            Register::g(ro),
            Register::g(rl),
            Register::g(rr),
        )
    }

    pub fn divide(ro: u32, rl: u32, rr: u32) -> Word {
        // sets ro to quotient, remainder to ro + 1
        Word::R(
            Op::Divide,
            Register::g(ro),
            Register::g(rl),
            Register::g(rr),
        )
    }

    pub fn bit_and(ro: u32, rl: u32, rr: u32) -> Word {
        Word::R(
            Op::BitAnd,
            Register::g(ro),
            Register::g(rl),
            Register::g(rr),
        )
    }

    pub fn bit_and_immediate(ro: u32, rl: u32, immediate: i32) -> Word {
        Word::I(
            Op::BitAndImmediate,
            Register::g(ro),
            Register::g(rl),
            immediate,
        )
    }

    pub fn bit_or(ro: u32, rl: u32, rr: u32) -> Word {
        Word::R(Op::BitOr, Register::g(ro), Register::g(rl), Register::g(rr))
    }

    pub fn bit_or_immediate(ro: u32, rl: u32, immediate: i32) -> Word {
        Word::I(
            Op::BitOrImmediate,
            Register::g(ro),
            Register::g(rl),
            immediate,
        )
    }

    pub fn left_shift(ro: u32, rl: u32, immediate: i32) -> Word {
        Word::I(Op::LeftShift, Register::g(ro), Register::g(rl), immediate)
    }

    pub fn right_shift(ro: u32, rl: u32, immediate: i32) -> Word {
        Word::I(Op::RightShift, Register::g(ro), Register::g(rl), immediate)
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

    pub fn branch_less_equal(rr: u32, rl: u32, relative: i32) -> Word {
        Word::I(
            Op::BranchLessEqual,
            Register::g(rr),
            Register::g(rl),
            relative,
        )
    }

    pub fn branch_greater(rr: u32, rl: u32, relative: i32) -> Word {
        Word::I(
            Op::BranchGreater,
            Register::g(rr),
            Register::g(rl),
            relative,
        )
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

    pub fn fload_immediate(ro: u32, immediate: f32) -> Word {
        Word::I(
            Op::FLoadImmediate,
            Register::g(ro),
            Register::g(0),
            i32::from_be_bytes(immediate.to_be_bytes()),
        )
    }

    pub fn fadd(ro: u32, rl: u32, rr: u32) -> Word {
        Word::R(Op::FAdd, Register::g(ro), Register::g(rl), Register::g(rr))
    }

    pub fn fadd_immediate(ro: u32, rl: u32, immediate: f32) -> Word {
        Word::I(
            Op::FAddImmediate,
            Register::g(ro),
            Register::g(rl),
            i32::from_be_bytes(immediate.to_be_bytes()),
        )
    }

    pub fn fsubtract(ro: u32, rl: u32, rr: u32) -> Word {
        Word::R(
            Op::FSubtract,
            Register::g(ro),
            Register::g(rl),
            Register::g(rr),
        )
    }

    pub fn fsubtract_immediate(ro: u32, rl: u32, immediate: f32) -> Word {
        Word::I(
            Op::FSubtractImmediate,
            Register::g(ro),
            Register::g(rl),
            i32::from_be_bytes(immediate.to_be_bytes()),
        )
    }

    pub fn fmultiply(ro: u32, rl: u32, rr: u32) -> Word {
        Word::R(
            Op::FMultiply,
            Register::g(ro),
            Register::g(rl),
            Register::g(rr),
        )
    }

    pub fn fdivide(ro: u32, rl: u32, rr: u32) -> Word {
        Word::R(
            Op::FDivide,
            Register::g(ro),
            Register::g(rl),
            Register::g(rr),
        )
    }

    pub fn fcompare(ro: u32, rl: u32, rr: u32) -> Word {
        Word::R(
            Op::FCompare,
            Register::g(ro),
            Register::g(rl),
            Register::g(rr),
        )
    }

    pub fn v_load_memory(ro: u32, rl: u32, offset: i32) -> Word {
        Word::I(Op::VLoadMemory, Register::v(ro), Register::g(rl), offset)
    }

    pub fn v_store_memory(ro: u32, rl: u32, offset: i32) -> Word {
        Word::I(Op::VStoreMemory, Register::v(ro), Register::g(rl), offset)
    }

    pub fn v_left_shift(ro: u32, rl: u32, immediate: i32) -> Word {
        Word::I(Op::VLeftShift, Register::v(ro), Register::v(rl), immediate)
    }

    pub fn v_right_shift(ro: u32, rl: u32, immediate: i32) -> Word {
        Word::I(Op::VRightShift, Register::v(ro), Register::v(rl), immediate)
    }

    pub fn v_add(ro: u32, rl: u32, rr: u32) -> Word {
        Word::R(Op::VAdd, Register::v(ro), Register::v(rl), Register::v(rr))
    }

    pub fn v_subtract(ro: u32, rl: u32, rr: u32) -> Word {
        Word::R(
            Op::VSubtract,
            Register::v(ro),
            Register::v(rl),
            Register::v(rr),
        )
    }

    pub fn v_multiply(ro: u32, rl: u32, rr: u32) -> Word {
        Word::R(
            Op::VMultiply,
            Register::v(ro),
            Register::v(rl),
            Register::v(rr),
        )
    }

    pub fn v_divide(ro: u32, rl: u32, rr: u32) -> Word {
        Word::R(
            Op::VDivide,
            Register::v(ro),
            Register::v(rl),
            Register::v(rr),
        )
    }

    pub fn v_fadd(ro: u32, rl: u32, rr: u32) -> Word {
        Word::R(Op::VFAdd, Register::v(ro), Register::v(rl), Register::v(rr))
    }

    pub fn v_fsubtract(ro: u32, rl: u32, rr: u32) -> Word {
        Word::R(
            Op::VFSubtract,
            Register::v(ro),
            Register::v(rl),
            Register::v(rr),
        )
    }

    pub fn v_fmultiply(ro: u32, rl: u32, rr: u32) -> Word {
        Word::R(
            Op::VFMultiply,
            Register::v(ro),
            Register::v(rl),
            Register::v(rr),
        )
    }

    pub fn v_fdivide(ro: u32, rl: u32, rr: u32) -> Word {
        Word::R(
            Op::VFDivide,
            Register::v(ro),
            Register::v(rl),
            Register::v(rr),
        )
    }
}
