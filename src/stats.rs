use core::fmt::{self, Display};

#[derive(Debug)]
pub struct StatsTracker {
    pub branch_predictions: u64,
    pub branch_mispredictions: u64,
    pub cycles: u64,
    pub instructions_started: u64,
    pub instructions_commited: u64,
}
impl StatsTracker {
    pub fn new() -> Self {
        Self {
            branch_predictions: 0,
            branch_mispredictions: 0,
            cycles: 0,
            instructions_started: 0,
            instructions_commited: 0,
        }
    }
}
impl Display for StatsTracker {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Program stats:")?;
        writeln!(f, " - Cycles: {}", self.cycles)?;
        writeln!(f, " - Instructions Started: {}", self.instructions_started)?;
        writeln!(
            f,
            " - Instructions Commited: {}",
            self.instructions_commited
        )?;
        writeln!(f, " - Branch Predictions: {}", self.branch_predictions)?;
        writeln!(
            f,
            " - Branch Mispredictions: {}",
            self.branch_mispredictions
        )
    }
}
