use core::fmt::{self, Display};

#[derive(Debug, Clone, Copy)]
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
        writeln!(
            f,
            " - Ops/Cycle: {:.2}",
            100.0 * self.instructions_started as f64 / self.cycles as f64
        )?;
        writeln!(f, " - Instructions Started: {}", self.instructions_started)?;
        writeln!(
            f,
            " - Instructions Commited: {}",
            self.instructions_commited
        )?;
        writeln!(
            f,
            " - Instructions Completed rate: {:.2}",
            100.0 * self.instructions_commited as f64 / self.instructions_started as f64
        )?;
        writeln!(f, " - Branch Predictions: {}", self.branch_predictions)?;
        writeln!(
            f,
            " - Branch Mispredictions: {}",
            self.branch_mispredictions
        )?;
        writeln!(
            f,
            " - Branch Misprediction rate: {:.2}",
            100.0 * self.branch_mispredictions as f64 / self.branch_predictions as f64
        )
    }
}
