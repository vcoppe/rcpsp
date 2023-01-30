use std::{hash::Hash, collections::{VecDeque, HashSet}, fmt::Display, vec};

use fixedbitset::FixedBitSet;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct State {
    /// These are the jobs that have already been scheduled
    pub done: FixedBitSet,
    /// These are the jobs that maybe have already been scheduled
    pub maybe_done: Option<FixedBitSet>,
    /// Consumption profile of each resource 
    pub profile: Vec<ConsumptionProfile>,
    /// Earliest time that each job can be scheduled
    pub earliest: Vec<isize>,
    /// This is the 'depth' in the schedule, the number of jobs that have already been scheduled
    pub depth: usize,
}

impl State {
    pub fn add_consumption(&mut self, start_time: isize, duration: isize, consumption: &Vec<isize>) {
        if duration > 0 {
            for (i, c) in consumption.iter().copied().enumerate() {
                if c > 0 {
                    self.profile[i].add_consumption(start_time, duration, c);
                }
            }
        }
    }

    pub fn get_earliest_start(&self, earliest: isize, duration: isize, consumption: &Vec<isize>) -> isize {
        let mut index = vec![0; self.profile.len()];
        let mut earliest = earliest;

        loop {
            let mut moved_earliest = false;
            for (i, profile) in self.profile.iter().enumerate() {
                if consumption[i] == 0 {
                    continue;
                }

                loop {
                    // find first step after earliest that has enough capacity
                    while profile.steps[index[i]].end <= earliest || profile.steps[index[i]].rem_capacity < consumption[i] {
                        index[i] += 1;
                    }

                    // check if enough time with the step found and the next ones to insert the job
                    let mut cumul_time: isize = 0;
                    let mut j = index[i];
                    loop {
                        if profile.steps[j].rem_capacity >= consumption[i] {
                            let time = profile.steps[j].end - profile.steps[j].start.max(earliest);
                            cumul_time = cumul_time.saturating_add(time);

                            if cumul_time >= duration {
                                break;
                            }
                        } else {
                            break;
                        }

                        j += 1;
                    }

                    // exit loop if enough time and capacity
                    if cumul_time >= duration {
                        break;
                    } else { // otherwise go to step after the one with not enough capacity
                        index[i] = j + 1;
                    }
                }

                // break the loop if earliest has changed
                if profile.steps[index[i]].start > earliest {
                    earliest = profile.steps[index[i]].start;
                    moved_earliest = true;
                    break;
                }
            }

            // exit loop if all profiles have agreed on a start time
            if !moved_earliest {
                break;
            }
        }

        earliest
    }

    pub fn propagate(&mut self, topo_order: &Vec<usize>, successors: &Vec<HashSet<usize>>, duration: &Vec<isize>, consumption: &Vec<Vec<isize>>) {
        for i in topo_order.iter().copied() {
            if self.done.contains(i) { // propagation already done for this job
                continue;
            }

            self.earliest[i] = self.get_earliest_start(self.earliest[i], duration[i], &consumption[i]);

            if let Some(maybe) = &self.maybe_done { // propagation may have been done for those jobs
                if maybe.contains(i) {
                    continue;
                }
            }

            for j in successors[i].iter().copied() {
                if !self.done.contains(j) {
                    self.earliest[j] = self.earliest[j].max(self.earliest[i] + duration[i]);
                }
            }
        }
    }

    pub fn forward_to_earliest(&mut self) {
        let mut earliest = None;
        for (i, e) in self.earliest.iter().copied().enumerate() {
            if !self.done.contains(i) {
                earliest = match earliest {
                    None => Some(e),
                    Some(value) => Some(value.min(e)),
                };
            }
        }

        if let Some(earliest) = earliest {
            if earliest > 0 {
                for profile in self.profile.iter_mut() {
                    profile.forward_by(earliest);
                }
                self.earliest.iter_mut().enumerate().for_each(|(i, e)| {
                    if !self.done.contains(i) {
                        *e -= earliest;
                    }
                });
            }
        }
    }

    pub fn merge_consumption_profile(&mut self, profile: &Vec<ConsumptionProfile>) {
        for i in 0..self.profile.len() {
            self.profile[i].merge_consumption_profile(&profile[i]);
        }
    }
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = writeln!(f, "done: {}", self.done);
        if res.is_err() {
            return res;
        }

        if let Some(maybe) = &self.maybe_done {
            let res = writeln!(f, "maybe_done: {}", maybe);
            if res.is_err() {
                return res;
            }
        }

        /*for (i, earliest) in self.earliest.iter().copied().enumerate() {
            let res = writeln!(f, "{}: {}", i, earliest);
            if res.is_err() {
                return res;
            }
        }*/

        for (i, profile) in self.profile.iter().enumerate() {
            let res = writeln!(f, "{}: {}", i, profile);
            if res.is_err() {
                return res;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ConsumptionProfile {
    pub steps: VecDeque<ConsumptionStep>
}

impl ConsumptionProfile {
    fn add_consumption(&mut self, start_time: isize, duration: isize, consumption: isize) {
        let end_time = start_time + duration;

        let mut i = 0;
        while i < self.steps.len() && self.steps[i].start < end_time {
            if start_time < self.steps[i].end && end_time > self.steps[i].start {
                let start = self.steps[i].start;
                let end = self.steps[i].end;
                let rem_capacity = self.steps[i].rem_capacity;
                if start_time <= start && end_time >= end { // step contained in interval
                    self.steps[i].rem_capacity -= consumption;
                } else if start_time >= start && end_time <= end { // interval contained in step
                    if end_time != end {
                        self.steps.insert(i + 1, ConsumptionStep { 
                            start: end_time, end: end, 
                            rem_capacity: rem_capacity 
                        });
                    }
                    if start_time != start {
                        self.steps.insert(i + 1, ConsumptionStep { 
                            start: start_time, end: end_time, 
                            rem_capacity:  rem_capacity - consumption
                        });
                        self.steps[i].end = start_time;
                    } else {
                        self.steps[i].rem_capacity -= consumption;
                        self.steps[i].end = end_time;
                    }
                    break;
                } else if start_time > start { // step contains start of interval
                    self.steps.insert(i + 1, ConsumptionStep { 
                        start: start_time, end: end, 
                        rem_capacity: rem_capacity - consumption 
                    });
                    self.steps[i].end = start_time;

                    i += 1; // skip interval just inserted
                } else if end_time < end { // step contains end of interval
                    self.steps.insert(i + 1, ConsumptionStep { 
                        start: end_time, end: end, 
                        rem_capacity: rem_capacity 
                    });
                    self.steps[i].end = end_time;
                    self.steps[i].rem_capacity -= consumption;
                    break;
                }
            }

            i += 1;
        }
    }

    pub fn forward_by(&mut self, delta: isize) {
        while !self.steps.is_empty() && self.steps[0].end <= delta {
            self.steps.pop_front();
        }

        self.steps[0].start = delta;

        for s in self.steps.iter_mut() {
            s.start -= delta;
            s.end -= delta;
        }

        let last = self.steps.len() - 1;
        self.steps[last].end = isize::MAX;
    }

    fn merge_consumption_profile(&mut self, other: &ConsumptionProfile) {
        let mut result = VecDeque::default();

        let mut i = 0;
        let mut j = 0;

        while i < self.steps.len() && self.steps[i].start < other.steps[0].start {
            result.push_back(ConsumptionStep { 
                start: self.steps[i].start, 
                end: self.steps[i].end.min(other.steps[0].start),
                rem_capacity: self.steps[i].rem_capacity
            });
            i += 1;
        }

        while j < other.steps.len() && other.steps[j].start < self.steps[0].start {
            result.push_back(ConsumptionStep { 
                start: other.steps[j].start, 
                end: other.steps[j].end.min(self.steps[0].start),
                rem_capacity: other.steps[j].rem_capacity
            });
            j += 1;
        }

        i = 0;
        j = 0;

        while i < self.steps.len() && j < other.steps.len() {
            let start = self.steps[i].start.max(other.steps[j].start);
            let end = self.steps[i].end.min(other.steps[j].end);
            
            if end > start {
                let rem_capacity = self.steps[i].rem_capacity.max(other.steps[j].rem_capacity);
                result.push_back(ConsumptionStep { start, end, rem_capacity });
            }
            
            if self.steps[i].end < other.steps[j].end {
                i += 1;
            } else {
                j += 1;
            }
        }

        // merge consecutive steps with same rem_capacity
        i = 0;
        while i+1 < result.len() {
            if result[i].rem_capacity == result[i+1].rem_capacity {
                result[i].end = result[i+1].end;
                result.remove(i+1);
            } else {
                i += 1;
            }
        }

        self.steps = result;
    }
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

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ConsumptionStep {
    pub start: isize,
    pub end: isize,
    pub rem_capacity: isize,
}