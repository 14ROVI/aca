use crate::instructions::Register;

#[derive(Debug, Clone, PartialEq)]
pub enum RobType {
    Branch,
    LoadMemory,
    StoreMemory,
    Register,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Destination {
    Reg(Register),
    Memory(usize),
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RobState {
    Issued,
    Executing,
    Finished,
}

#[derive(Debug, Clone)]
pub struct RobInst {
    pub index: usize,
    pub inst: RobType,
    pub destination: Destination,
    pub value: i32,
    pub state: RobState,
}

#[derive(Debug, Clone)]
pub struct ReorderBuffer {
    pub buffer: Vec<Option<RobInst>>,
    size: usize,
    max_retire: usize,
    head: usize,
    tail: usize,
}
impl ReorderBuffer {
    pub fn new(size: usize, max_retire: usize) -> Self {
        let buffer = vec![None; size];

        Self {
            buffer,
            size,
            max_retire,
            head: 0,
            tail: 0,
        }
    }

    pub fn is_full(&self) -> bool {
        self.buffer[self.head].is_some() // our head has reached the tail (could do maths too)
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.iter().all(|item| item.is_none())
    }

    pub fn flush(&mut self) {
        *self = Self::new(self.size, self.max_retire);
    }

    pub fn add_instruction(&mut self, mut instruction: RobInst) -> usize {
        if self.is_full() {
            panic!("Tried to add instruction to rob but doesn't have capacity!");
        } else {
            let index = self.head;
            instruction.index = index;
            self.buffer[index] = Some(instruction);
            self.head = (self.head + 1) % self.size;

            return index;
        }
    }

    pub fn get_mut(&mut self, index: usize) -> &mut Option<RobInst> {
        &mut self.buffer[index]
    }

    pub fn retire(&mut self) -> Vec<RobInst> {
        let mut retired = Vec::new();

        for _ in 0..self.max_retire {
            if let Some(inst_op) = &self.buffer[self.tail] {
                if inst_op.state == RobState::Finished {
                    let inst_op = self.buffer[self.tail].take().unwrap();
                    retired.push(inst_op);
                    self.tail = (self.tail + 1) % self.size;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        return retired;
    }
}
