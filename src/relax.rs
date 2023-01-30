use ddo::{Relaxation, Decision};
use fixedbitset::FixedBitSet;

use crate::{model::Rcpsp, state::State};

#[derive(Clone)]
pub struct RcpspRelax<'a> {
    pub pb: &'a Rcpsp,
}

impl Relaxation for RcpspRelax<'_> {
    type State = State;

    fn merge(&self, states: &mut dyn Iterator<Item = &State>) -> State {
        let mut merged = self.pb.initial.clone();
        merged.done.toggle_range(..);
        merged.profile.iter_mut().for_each(|c| {
            c.steps[0].rem_capacity = 0;
        });

        let mut maybe_done = FixedBitSet::with_capacity(self.pb.instance.n_jobs);

        for state in states {
            merged.done &= &state.done;
            maybe_done |= &state.done;

            if let Some(maybe) = &state.maybe_done {
                maybe_done |= maybe;
            }

            merged.merge_consumption_profile(&state.profile);

            for i in 0..self.pb.instance.n_jobs {
                if !state.done.contains(i) {
                    merged.earliest[i] = merged.earliest[i].min(state.earliest[i]);
                }
            }

            merged.depth = merged.depth.max(state.depth);
        }

        maybe_done ^= &merged.done;
        merged.maybe_done = Some(maybe_done);

        merged
    }

    fn relax(&self, _src: &Self::State, _dest: &Self::State, _merged: &Self::State, _d: Decision, cost: isize) -> isize {
        cost
    }
}
