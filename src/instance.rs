use std::{fs::File, io::{BufRead, BufReader, Lines, Read}, collections::HashSet};

use fixedbitset::FixedBitSet;

/// This structure represents the RCPSP instance.
#[derive(Debug, Clone)]
pub struct RcpspInstance {
    // Number of jobs
    pub n_jobs: usize,
    // Number of resources
    pub n_resources: usize,
    // Precedence constraints
    pub predecessors: Vec<FixedBitSet>,
    pub successors: Vec<FixedBitSet>,
    pub predecessors_set: Vec<HashSet<usize>>,
    pub successors_set: Vec<HashSet<usize>>,
    // Duration of the jobs
    pub duration: Vec<isize>,
    // Consumption of the jobs for each resource
    pub consumption: Vec<Vec<isize>>,
    // Capacity of the resources
    pub capacity: Vec<isize>,
}

impl From<File> for RcpspInstance {
    fn from(file: File) -> Self {
        Self::from(BufReader::new(file))
    }
}
impl <S: Read> From<BufReader<S>> for RcpspInstance {
    fn from(buf: BufReader<S>) -> Self {
        Self::from(buf.lines())
    }
}
impl <B: BufRead> From<Lines<B>> for RcpspInstance {
    fn from(lines: Lines<B>) -> Self {
        let mut lc = 0;
        
        let mut n_jobs = 0;
        let mut n_resources = 0;
        let mut predecessors = vec![];
        let mut successors = vec![];
        let mut predecessors_set = vec![];
        let mut successors_set = vec![];
        let mut duration = vec![];
        let mut weight = vec![];
        let mut capacity = vec![];

        for line in lines {
            let line = line.unwrap();
            let line = line.trim();

            if lc == 0 {
                let mut it = line.split_whitespace();
                n_jobs = it.next().unwrap().to_string().parse::<usize>().unwrap();
                n_resources = it.next().unwrap().to_string().parse::<usize>().unwrap();

                (0..n_jobs).for_each(|_| {
                    predecessors.push(FixedBitSet::with_capacity(n_jobs));
                    successors.push(FixedBitSet::with_capacity(n_jobs));
                    predecessors_set.push(HashSet::new());
                    successors_set.push(HashSet::new());
                    duration.push(0);
                });
                weight = vec![vec![0; n_resources]; n_jobs];
            } else if lc == 1 {
                for cap in line.split_whitespace() {
                    capacity.push(cap.to_string().parse::<isize>().unwrap());
                }
            } else if (2..(2+n_jobs)).contains(&lc) {
                let i = (lc - 2) as usize;
                let mut it = line.split_whitespace();

                duration[i] = it.next().unwrap().to_string().parse::<isize>().unwrap();

                for j in 0..n_resources {
                    weight[i][j] = it.next().unwrap().to_string().parse::<isize>().unwrap();
                }

                let n_successors = it.next().unwrap().to_string().parse::<usize>().unwrap();
                for _ in 0..n_successors {
                    let j = it.next().unwrap().to_string().parse::<usize>().unwrap() - 1;
                    predecessors[j].insert(i);
                    successors[i].insert(j);
                    predecessors_set[j].insert(i);
                    successors_set[i].insert(j);
                }
            }
            
            lc += 1;
        }

        RcpspInstance { n_jobs, n_resources, predecessors, successors, predecessors_set, successors_set, duration, consumption: weight, capacity }
    }
}
