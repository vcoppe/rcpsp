use ddo::{StateRanking};

use crate::state::State;

#[derive(Debug, Copy, Clone)]
pub struct RcpspRanking;

impl StateRanking for RcpspRanking {
    type State = State;

    fn compare(&self, sa: &Self::State, sb: &Self::State) -> std::cmp::Ordering {
        sa.depth.cmp(&sb.depth)
    }
}
