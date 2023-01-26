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
            consumption,
            earliest: vec![0; inst.n_jobs],
            depth : 0
        };
        let order = Self::toposort(&inst);
        // propagate earliest start times
        for i in order.iter().copied() {
            for j in inst.successors_set[i].iter().copied() {
                state.earliest[j] = state.earliest[j].max(state.earliest[i] + inst.duration[i]);
            }
        }
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
        let mut not_done = state.done.clone();
        not_done.toggle_range(..);
        if state.done.count_ones(..) == state.depth { // must only schedule jobs that are not done
            for i in not_done.ones() {
                if &self.instance.predecessors[i] & &state.done == self.instance.predecessors[i] {
                    f.apply(Decision { variable, value: i as isize })
                }
            }
        } else if let Some(maybe) = &state.maybe_done { // can schedule jobs that are maybe done
            let maybe_done = &state.done | &maybe;
            for i in not_done.ones() {
                if &self.instance.predecessors[i] & &maybe_done == self.instance.predecessors[i] {
                    f.apply(Decision { variable, value: i as isize })
                }
            }
        }
    }

    fn combined_transition(&self, state: &State, d: Decision) -> (State, isize) {
        let d = d.value as usize;

        // println!("transition scheduling job {} at depth {}", d, state.depth);

        let mut done = state.done.clone();
        done.insert(d);
        
        let mut maybe_done = state.maybe_done.clone();
        if let Some(maybe) = maybe_done.as_mut() {
            maybe.insert(d);
        }

        let start_time = self.compute_start_time(&state.consumption, d, state.earliest[d]);
        let end_time = start_time + self.instance.duration[d];
    
        let mut consumption = state.consumption.clone();
        self.add_job_consumption(&mut consumption, d, start_time); // add job consumption to profiles

        let mut earliest = state.earliest.clone();
        earliest[d] = 0;
        for i in self.instance.successors_set[d].iter().copied() {
            earliest[i] = earliest[i].max(end_time);
        }

        let mut earliest_not_done = isize::MAX;
        if state.depth == self.instance.n_jobs - 1 {
            earliest_not_done = 0;
        } else {
            // propagate earliest start times
            for i in self.topo_order.iter() {
                if done.contains(*i) { // propagation already done for those jobs
                    continue;
                }

                earliest[*i] = self.compute_start_time(&consumption, *i, earliest[*i]);
                earliest_not_done = earliest_not_done.min(earliest[*i]);

                if let Some(maybe) = &maybe_done { // propagation may have been done for those jobs
                    if maybe.contains(*i) {
                        continue;
                    }
                }

                for j in self.instance.successors_set[*i].iter() {
                    earliest[*j] = earliest[*j].max(earliest[*i] + self.instance.duration[*i]);
                }
            }
        }

        let cost = state.earliest[self.instance.n_jobs - 1] - earliest[self.instance.n_jobs - 1];

        if earliest_not_done > 0 {
            self.forward_to(&mut consumption, earliest_not_done); // normalize consumption profiles at earliest remaining job start time
            for t in earliest.iter_mut() {
                if *t > 0 {
                    *t -= earliest_not_done;
                }
            }
        }

        (
            State {
                done,
                maybe_done,
                consumption,
                earliest,
                depth: state.depth + 1
            },
            cost
        )
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
    fn compute_start_time(&self, consumption: &Vec<ConsumptionProfile>, job: usize, earliest: isize) -> isize {
        //println!("compute_start_time");
        let mut intervals = vec![(earliest, isize::MAX)];
        let duration = self.instance.duration[job];
        for i in 0..self.instance.n_resources {
            let weight = self.instance.weight[job][i];

            if weight == 0 {
                continue;
            }

            let mut resource_intervals = vec![];
            let mut current: Option<(isize, isize)> = None;

            //println!("insert weight {} during {} (from {})", weight, duration, state.earliest[job]);
            //println!("in {}", state.consumption[i]);

            for step in consumption[i].steps.iter() {
                if let Some(cur) = current.as_mut() {
                    if step.rem_capacity < weight { // the job does not fit
                        cur.1 = step.start;

                        if cur.1 - cur.0 >= duration {
                            resource_intervals.push(*cur);
                        }

                        current = None;
                    }
                } else if step.rem_capacity >= weight { // the job fits
                    current = Some((step.start, isize::MAX));
                }
            }

            if let Some(cur) = current { // can place up to infinity
                resource_intervals.push(cur);
            }

            /*print!("resource {} intervals: ", i);
            resource_intervals.iter().for_each(|i| print!("({} - {})", i.0, i.1));
            println!();*/

            intervals = self.restrict_intervals(&intervals, &resource_intervals, job);
            
            /*print!("intersection: ");
            intervals.iter().for_each(|i| print!("({} - {})", i.0, i.1));
            println!();*/
        }

        //println!("earliest found = {}", intervals[0].0);

        intervals[0].0
    }

    fn restrict_intervals(&self, intervals_1: &Vec<(isize,isize)>, intervals_2: &Vec<(isize,isize)>, job: usize) -> Vec<(isize,isize)> {
        let mut i = 0;
        let mut j = 0;

        let duration = self.instance.duration[job];

        let mut intervals = vec![];

        while i < intervals_1.len() && j < intervals_2.len() {
            let start = intervals_1[i].0.max(intervals_2[j].0);
            let end = intervals_1[i].1.min(intervals_2[j].1);
            
            if end - start >= duration {
                intervals.push((start, end));
            }
            
            if intervals_1[i].1 < intervals_2[j].1 {
                i += 1;
            } else {
                j += 1;
            }
        }

        intervals
    }

    fn forward_to(&self, consumption: &mut Vec<ConsumptionProfile>, earliest: isize) {
        //println!("forward consumption profile to {}", earliest);

        for i in 0..self.instance.n_resources {
            //println!("{}", consumption[i]);

            while !consumption[i].steps.is_empty() && consumption[i].steps[0].end <= earliest {
                consumption[i].steps.pop_front();
            }

            if !consumption[i].steps.is_empty() {
                consumption[i].steps[0].start = earliest;
            }

            for s in consumption[i].steps.iter_mut() {
                s.start -= earliest;
                s.end -= earliest;
            }

            let last = consumption[i].steps.len() - 1;
            consumption[i].steps[last].end = isize::MAX;

            //println!("after forwarding: {}", consumption[i]);
        }
    }

    fn add_job_consumption(&self, consumption: &mut Vec<ConsumptionProfile>, job: usize, start_time: isize) {
        let end_time = start_time + self.instance.duration[job];

        if start_time == end_time {
            return;
        }

        //println!("add job consumption");

        for i in 0..self.instance.n_resources {
            let weight = self.instance.weight[job][i];

            if weight == 0 {
                continue;
            }

            //println!("current consumption: {}", consumption[i]);

            //println!("\nremoving ({} - {}, {})", start_time, end_time, weight);

            let mut j = 0;
            while j < consumption[i].steps.len() && consumption[i].steps[j].start < end_time {
                if start_time < consumption[i].steps[j].end && end_time > consumption[i].steps[j].start {
                    let start = consumption[i].steps[j].start;
                    let end = consumption[i].steps[j].end;
                    let rem_capacity = consumption[i].steps[j].rem_capacity;
                    if start_time <= consumption[i].steps[j].start && end_time >= consumption[i].steps[j].end { // step fully covered by interval
                        consumption[i].steps[j].rem_capacity -= weight;
                    } else if start_time >= consumption[i].steps[j].start && end_time <= consumption[i].steps[j].end { // step contains interval
                        if end_time != end {
                            consumption[i].steps.insert(j + 1, ConsumptionStep { 
                                start: end_time, end: end, 
                                rem_capacity: rem_capacity 
                            });
                        }
                        if start_time != start {
                            consumption[i].steps.insert(j + 1, ConsumptionStep { 
                                start: start_time, end: end_time, 
                                rem_capacity:  rem_capacity - weight
                            });
                            consumption[i].steps[j].end = start_time;
                        } else {
                            consumption[i].steps[j].rem_capacity -= weight;
                            consumption[i].steps[j].end = end_time;
                        }
                        break;
                    } else if start_time > consumption[i].steps[j].start { // step contains start of interval
                        consumption[i].steps.insert(j + 1, ConsumptionStep { 
                            start: start_time, end: end, 
                            rem_capacity: rem_capacity - weight 
                        });
                        consumption[i].steps[j].end = start_time;

                        j += 1; // skip interval just inserted
                    } else if end_time < consumption[i].steps[j].end { // step contains end of interval
                        consumption[i].steps.insert(j + 1, ConsumptionStep { 
                            start: end_time, end: end, 
                            rem_capacity: rem_capacity 
                        });
                        consumption[i].steps[j].end = end_time;
                        consumption[i].steps[j].rem_capacity -= weight;
                        break;
                    }
                }

                j += 1;
            }

            /*for s in consumption[i].steps.iter() {
                if s.rem_capacity < 0 {
                    println!("wtf {}", consumption[i]);
                    break;
                }
            }*/

            // println!("resulting consumption: {}", consumption[i]);
        }
    }

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