use std::{hash::Hash, collections::VecDeque, fmt::Display};

use fixedbitset::FixedBitSet;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct State {
    /// These are the jobs that have already been scheduled
    pub done: FixedBitSet,
    /// These are the jobs that maybe have already been scheduled
    pub maybe_done: Option<FixedBitSet>,
    /// Consumption profile of each resource 
    pub consumption: Vec<ConsumptionProfile>,
    /// Earliest time that each job can be scheduled
    pub earliest: Vec<isize>,
    /// This is the 'depth' in the schedule, the number of jobs that have already been scheduled
    pub depth: usize,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ConsumptionProfile {
    pub steps: VecDeque<ConsumptionStep>
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ConsumptionStep {
    pub start: isize,
    pub end: isize,
    pub rem_capacity: isize,
}

impl Display for ConsumptionProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for s in self.steps.iter() {
            let res = write!(f, "({} - {}, {})", s.start, s.end, s.rem_capacity);
            if res.is_err() {
                return res;
            }
        }
        Ok(())
    }
}