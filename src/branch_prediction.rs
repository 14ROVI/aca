use std::collections::HashMap;

pub enum BranchState {
    StrongNotTaken,
    WeakNotTaken,
    WeakTaken,
    StrongTaken,
}
impl BranchState {
    pub fn update_taken(&mut self) {
        *self = match self {
            Self::StrongNotTaken => Self::WeakNotTaken,
            Self::WeakNotTaken => Self::WeakTaken,
            Self::WeakTaken => Self::StrongTaken,
            Self::StrongTaken => Self::StrongTaken,
        };
    }

    pub fn update_not_taken(&mut self) {
        *self = match self {
            Self::StrongNotTaken => Self::StrongNotTaken,
            Self::WeakNotTaken => Self::StrongNotTaken,
            Self::WeakTaken => Self::WeakNotTaken,
            Self::StrongTaken => Self::WeakTaken,
        };
    }

    pub fn predict(&self) -> bool {
        match self {
            Self::StrongNotTaken | Self::WeakNotTaken => false,
            Self::StrongTaken | Self::WeakTaken => true,
        }
    }
}

pub struct BranchPredictor {
    state_machines: HashMap<usize, BranchState>, // pc -> state machine
}
impl BranchPredictor {
    pub fn new() -> Self {
        BranchPredictor {
            state_machines: HashMap::new(),
        }
    }

    pub fn predict(&mut self, pc: usize) -> bool {
        self.state_machines.get(&pc).map_or(
            // first prediction assumes we take because of loops!
            true,
            |s| s.predict(),
        )
    }

    pub fn update(&mut self, pc: usize, taken: bool) {
        let state = self.state_machines.get_mut(&pc);

        match (state, taken) {
            (Some(state), true) => state.update_taken(),
            (Some(state), false) => state.update_not_taken(),
            (None, true) => {
                self.state_machines.insert(pc, BranchState::WeakTaken);
            }
            (None, false) => {
                self.state_machines.insert(pc, BranchState::WeakNotTaken);
            }
        };
    }
}
