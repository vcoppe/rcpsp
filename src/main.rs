use std::{fs::File, time::{Duration, Instant}};

use clap::Parser;
use ddo::{FixedWidth, NoCutoff, MaxUB, Solver, Completion, NoDupFringe, ParBarrierSolverFc, NbUnassignedWitdh, Problem, WidthHeuristic, TimeBudget, Cutoff};
use heuristics::RcpspRanking;
use instance::RcpspInstance;
use model::Rcpsp;
use relax::RcpspRelax;

mod instance;
mod model;
mod state;
mod relax;
mod heuristics;

#[derive(Debug, clap::Parser)]
struct Args {
    /// Max width of any layer (defaults to the same number of 
    /// nodes as there are unassigned variables)
    #[clap(short, long)]
    width: Option<usize>,
    /// Timeout for the resolution of the problem
    #[clap(short, long)]
    duration: Option<u64>,
    /// Number of threads used to solve the instance
    #[clap(short, long)]
    threads: Option<usize>,
    /// Path to the instance file containing the instance to solve
    instance: String,
}

fn main() {
    let args = Args::parse();

    let instance = RcpspInstance::from(File::open(&args.instance).unwrap());
    let problem = Rcpsp::new(instance);
    let relaxation = RcpspRelax{pb: &problem};
    let ranking = RcpspRanking;

    let width: Box<dyn WidthHeuristic<_> + Send + Sync> = if let Some(w) = args.width {
        Box::new(FixedWidth(w))
    } else {
        Box::new(NbUnassignedWitdh(problem.nb_variables()))
    };
    let cutoff: Box<dyn Cutoff + Send + Sync> = if let Some(d) = args.duration {
        Box::new(TimeBudget::new(Duration::from_secs(d)))
    } else {
        Box::new(NoCutoff)
    };

    let mut fringe = NoDupFringe::new(MaxUB::new(&ranking));
    let mut solver = ParBarrierSolverFc::new(
        &problem, 
        &relaxation, 
        &ranking, 
        width.as_ref(), 
        cutoff.as_ref(), 
        &mut fringe);

    if let Some(threads) = args.threads {
        solver = solver.with_nb_threads(threads);
    }
    
    let time = Instant::now();
    let Completion{is_exact, best_value} = solver.maximize();
    let duration = time.elapsed();
    let best = best_value.map_or(isize::MIN, |value| - value);

    println!("Best value: {}", best);
    println!("Optimal   : {}", is_exact);
    println!("Elapsed   : {}", duration.as_secs_f64());
}
