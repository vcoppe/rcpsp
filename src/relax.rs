use std::{collections::VecDeque, mem::swap};

use ddo::{Relaxation, Decision};
use fixedbitset::FixedBitSet;

use crate::{model::Rcpsp, state::{State, ConsumptionProfile, ConsumptionStep}};

#[derive(Clone)]
pub struct RcpspRelax<'a> {
    pb : &'a Rcpsp,
}
impl <'a> RcpspRelax<'a> {
    pub fn new(pb: &'a Rcpsp) -> Self {
        Self{pb}
    }

    fn merge_consumption(
        &self,
        consumption: &Vec<ConsumptionProfile>,
        other_consumption: &Vec<ConsumptionProfile>,
        result: &mut Vec<ConsumptionProfile>
    ) {
        for r in 0..self.pb.instance.n_resources {

            //println!("consumption 1: {}", consumption[r]);
            //println!("consumption 2: {}", other_consumption[r]);

            let mut i = 0;
            let mut j = 0;

            while i < consumption[r].steps.len() && consumption[r].steps[i].start < other_consumption[r].steps[0].start {
                result[r].steps.push_back(ConsumptionStep { 
                    start: consumption[r].steps[i].start, 
                    end: consumption[r].steps[i].end.min(other_consumption[r].steps[0].start),
                    rem_capacity: consumption[r].steps[i].rem_capacity
                });
                i += 1;
            }

            while j < other_consumption[r].steps.len() && other_consumption[r].steps[j].start < consumption[r].steps[0].start {
                result[r].steps.push_back(ConsumptionStep { 
                    start: other_consumption[r].steps[j].start, 
                    end: other_consumption[r].steps[j].end.min(consumption[r].steps[0].start),
                    rem_capacity: other_consumption[r].steps[j].rem_capacity
                });
                j += 1;
            }

            i = 0;
            j = 0;

            while i < consumption[r].steps.len() && j < other_consumption[r].steps.len() {
                let start = consumption[r].steps[i].start.max(other_consumption[r].steps[j].start);
                let end = consumption[r].steps[i].end.min(other_consumption[r].steps[j].end);
                
                if end > start {
                    let rem_capacity = consumption[r].steps[i].rem_capacity.max(other_consumption[r].steps[j].rem_capacity);
                    result[r].steps.push_back(ConsumptionStep { start, end, rem_capacity });
                }
                
                if consumption[r].steps[i].end < other_consumption[r].steps[j].end {
                    i += 1;
                } else {
                    j += 1;
                }
            }

            // merge consecutive steps with same rem_capacity
            i = 0;
            while i+1 < result[r].steps.len() {
                if result[r].steps[i].rem_capacity == result[r].steps[i+1].rem_capacity {
                    result[r].steps[i].end = result[r].steps[i+1].end;
                    result[r].steps.remove(i+1);
                } else {
                    i += 1;
                }
            }

            //println!("result of merge: {}", result[r]);
        }
    }
}

impl Relaxation for RcpspRelax<'_> {
    type State = State;

    fn merge(&self, states: &mut dyn Iterator<Item = &State>) -> State {
        let mut done = FixedBitSet::with_capacity(self.pb.instance.n_jobs);
        done.toggle_range(..);
        let mut maybe_done = FixedBitSet::with_capacity(self.pb.instance.n_jobs);
        let mut consumption = vec![];
        let mut earliest = vec![isize::MAX; self.pb.instance.n_jobs];
        let mut depth = 0;

        let mut result_consumption = vec![];
        for _i in 0..self.pb.instance.n_resources {
            let mut steps = VecDeque::new();
            steps.push_back(ConsumptionStep { start: 0, end: isize::MAX, rem_capacity: 0 });
            consumption.push(ConsumptionProfile { steps });
            result_consumption.push(ConsumptionProfile { steps: VecDeque::new() });
        }


        for state in states {
            done &= &state.done;
            maybe_done |= &state.done;

            if let Some(maybe) = &state.maybe_done {
                maybe_done |= maybe;
            }

            self.merge_consumption(&consumption, &state.consumption, &mut result_consumption);
            swap(&mut consumption, &mut result_consumption);
            result_consumption.iter_mut().for_each(|c| c.steps.clear());

            let mut not_done = state.done.clone();
            not_done.toggle_range(..);
            for i in not_done.ones() {
                earliest[i] = earliest[i].min(state.earliest[i]);
            }

            depth = depth.max(state.depth);
        }

        maybe_done ^= &done;

        State {
            done,
            maybe_done: Some(maybe_done),
            consumption,
            earliest,
            depth
        }
    }

    fn relax(
        &self,
        _src: &Self::State,
        _dest: &Self::State,
        _merged: &Self::State,
        _: Decision,
        cost: isize,
    ) -> isize
    {
        cost
    }
}
