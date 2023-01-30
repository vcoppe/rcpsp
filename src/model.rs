use std::{vec, collections::VecDeque};

use ddo::{Problem, Variable, Decision, DecisionCallback};
use fixedbitset::FixedBitSet;

use crate::{instance::RcpspInstance, state::{State, ConsumptionProfile, ConsumptionStep}};


/// This is the structure encapsulating the Rcpsp problem.
#[derive(Debug, Clone)]
pub struct Rcpsp {
    pub instance: RcpspInstance,
    pub initial : State,
    pub topo_order: Vec<usize>,
}
impl Rcpsp {
    pub fn new(inst: RcpspInstance) -> Self {
        let mut consumption = vec![];
        for i in 0..inst.n_resources {
            let mut steps = VecDeque::new();
            steps.push_back(ConsumptionStep { start: 0, end: isize::MAX, rem_capacity: inst.capacity[i]});
            consumption.push(ConsumptionProfile { steps });
        }
        let mut state = State {
            done: FixedBitSet::with_capacity(inst.n_jobs),
            maybe_done: None,
            profile: consumption,
            earliest: vec![0; inst.n_jobs],
            depth : 0
        };
        let order = Self::toposort(&inst);
        state.propagate(&order, &inst.successors_set, &inst.duration, &inst.consumption);
        Self { instance: inst, initial: state, topo_order: order }
    }
}

impl Problem for Rcpsp {
    type State = State;

    fn nb_variables(&self) -> usize {
        self.instance.n_jobs
    }

    fn initial_state(&self) -> State {
        self.initial.clone()
    }

    fn initial_value(&self) -> isize {
        - self.initial.earliest[self.instance.n_jobs - 1]
    }

    fn for_each_in_domain(&self, variable: Variable, state: &Self::State, f: &mut dyn DecisionCallback)
    {
        if state.done.count_ones(..) == state.depth { // must only schedule jobs that are not done
            for i in 0..self.instance.n_jobs {
                if !state.done.contains(i) && &self.instance.predecessors[i] & &state.done == self.instance.predecessors[i] {
                    f.apply(Decision { variable, value: i as isize })
                }
            }
        } else if let Some(maybe) = &state.maybe_done { // can schedule jobs that are maybe done
            let maybe_done = &state.done | &maybe;
            for i in 0..self.instance.n_jobs {
                if !state.done.contains(i) && &self.instance.predecessors[i] & &maybe_done == self.instance.predecessors[i] {
                    f.apply(Decision { variable, value: i as isize })
                }
            }
        }
    }

    fn combined_transition(&self, state: &State, d: Decision) -> (State, isize) {
        let d = d.value as usize;

        let mut successor = state.clone();
        successor.depth = state.depth + 1;
        successor.done.insert(d);
        successor.add_consumption(state.earliest[d], self.instance.duration[d], &self.instance.consumption[d]);
        successor.propagate(&self.topo_order, &self.instance.successors_set, &self.instance.duration, &self.instance.consumption);

        let delta = successor.earliest[self.instance.n_jobs - 1] - state.earliest[self.instance.n_jobs - 1];

        successor.earliest[d] = 0; // clear estimation of the job scheduled

        successor.forward_to_earliest();

        (successor, - delta)
    }

    fn next_variable(&self, depth: usize, _: &mut dyn Iterator<Item = &Self::State>)
        -> Option<Variable> {
        if depth < self.nb_variables() {
            Some(Variable(depth))
        } else {
            None
        }
    }

    fn transition(&self, _state: &Self::State, _decision: Decision) -> Self::State {
        todo!()
    }

    fn transition_cost(&self, _state: &Self::State, _decision: Decision) -> isize {
        todo!()
    }
}

impl Rcpsp {
    fn toposort(instance: &RcpspInstance) -> Vec<usize> {
        let mut predecessors = vec![];
        for i in 0..instance.n_jobs {
            predecessors.push(instance.predecessors_set[i].clone());
        }

        let mut order = vec![];
        let mut open = vec![0];

        while !open.is_empty() {
            let i = open.pop().unwrap();
            order.push(i);

            for j in instance.successors_set[i].iter() {
                if predecessors[*j].remove(&i) && predecessors[*j].is_empty() {
                    open.push(*j);
                }
            }
        }

        order
    }
}