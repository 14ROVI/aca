use std::collections::HashMap;

#[allow(dead_code)]
pub enum BranchPredictionMode {
    AlwaysTake,
    NeverTake,
    OneBitSaturating,
    TwoBitSaturating,
    HistoryTwoBitSaturating(u32),
}

struct SaturatingCounter {
    state: u32,
    bits: u32,
}
impl SaturatingCounter {
    pub fn new(state: u32, bits: u32) -> Self {
        Self { state, bits }
    }

    pub fn new_taken(bits: u32) -> Self {
        Self::new(1 << (bits - 1), bits)
    }

    pub fn new_not_taken(bits: u32) -> Self {
        Self::new((1 << (bits - 1)) - 1, bits)
    }

    pub fn update_taken(&mut self) {
        self.state = (self.state + 1).min((1 << self.bits) - 1);
    }

    pub fn update_not_taken(&mut self) {
        self.state = self.state.saturating_sub(1);
    }

    pub fn predict(&self) -> bool {
        self.state >= 1 << (self.bits - 1)
    }
}

pub trait BranchPredictor {
    fn predict(&mut self, pc: usize) -> bool;
    fn update(&mut self, pc: usize, taken: bool);
}

pub struct CoreBranchPredictor {
    bp: Box<dyn BranchPredictor>,
}
impl CoreBranchPredictor {
    pub fn new(mode: BranchPredictionMode) -> Self {
        let bp: Box<dyn BranchPredictor> = match mode {
            BranchPredictionMode::AlwaysTake => Box::new(StaticBranchPredictor::new(true)),
            BranchPredictionMode::NeverTake => Box::new(StaticBranchPredictor::new(false)),
            BranchPredictionMode::OneBitSaturating => Box::new(SaturatingBranchPredictor::new(1)),
            BranchPredictionMode::TwoBitSaturating => Box::new(SaturatingBranchPredictor::new(2)),
            BranchPredictionMode::HistoryTwoBitSaturating(bits) => {
                Box::new(HistoryTwoBitSaturatingPredictor::new(bits))
            }
        };

        Self { bp }
    }
}
impl BranchPredictor for CoreBranchPredictor {
    fn predict(&mut self, pc: usize) -> bool {
        return self.bp.predict(pc);
    }

    fn update(&mut self, pc: usize, taken: bool) {
        self.bp.update(pc, taken);
    }
}

struct StaticBranchPredictor {
    should_take: bool,
}
impl StaticBranchPredictor {
    fn new(should_take: bool) -> Self {
        Self { should_take }
    }
}
impl BranchPredictor for StaticBranchPredictor {
    fn predict(&mut self, _pc: usize) -> bool {
        self.should_take // always take ;)
    }

    fn update(&mut self, _pc: usize, _taken: bool) {
        // nothing to update, dont do anything
    }
}

struct SaturatingBranchPredictor {
    bits: u32,
    state_machines: HashMap<usize, SaturatingCounter>, // pc -> state machine
}
impl SaturatingBranchPredictor {
    fn new(bits: u32) -> Self {
        Self {
            bits,
            state_machines: HashMap::new(),
        }
    }
}
impl BranchPredictor for SaturatingBranchPredictor {
    fn predict(&mut self, pc: usize) -> bool {
        self.state_machines.get(&pc).map_or(
            // first prediction assumes we dont take because of loops!
            false,
            |s| s.predict(),
        )
    }

    fn update(&mut self, pc: usize, taken: bool) {
        let state = self.state_machines.get_mut(&pc);

        match (state, taken) {
            (Some(state), true) => state.update_taken(),
            (Some(state), false) => state.update_not_taken(),
            (None, true) => {
                self.state_machines
                    .insert(pc, SaturatingCounter::new_taken(self.bits));
            }
            (None, false) => {
                self.state_machines
                    .insert(pc, SaturatingCounter::new_not_taken(self.bits));
            }
        };
    }
}

struct HistoryTwoBitSaturatingPredictor {
    bits: u32,
    lhr: HashMap<usize, u32>,                            // pc -> history
    histories: HashMap<(usize, u32), SaturatingCounter>, // (pc, hr) -> predictor
}
impl HistoryTwoBitSaturatingPredictor {
    pub fn new(bits: u32) -> Self {
        Self {
            bits,
            lhr: HashMap::new(),
            histories: HashMap::new(),
        }
    }
}
impl BranchPredictor for HistoryTwoBitSaturatingPredictor {
    fn predict(&mut self, pc: usize) -> bool {
        self.lhr
            .get(&pc)
            .and_then(|history| self.histories.get(&(pc, *history)))
            .map_or(false, |c| c.predict())
    }

    fn update(&mut self, pc: usize, taken: bool) {
        let mut history = *self.lhr.get(&pc).unwrap_or(&0);
        let state = self.histories.get_mut(&(pc, history));

        history = ((history << 1) | (taken as u32)) << (32 - self.bits) >> (32 - self.bits);

        match (state, taken) {
            (Some(state), true) => state.update_taken(),
            (Some(state), false) => state.update_not_taken(),
            (None, true) => {
                self.histories
                    .insert((pc, history), SaturatingCounter::new_taken(2));
            }
            (None, false) => {
                self.histories
                    .insert((pc, history), SaturatingCounter::new_not_taken(2));
            }
        };

        self.lhr.insert(pc, history);
    }
}
