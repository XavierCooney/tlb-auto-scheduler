use crate::{
    evaluator::{Problem, Solution},
    mutation::Mutation,
    utils::indent_lines,
};
use std::{fmt::Write as _, time::Instant};

#[derive(Debug, Clone, Copy)]
pub struct SolverSeed {
    pub num_rounds: u64,
    pub rng_seed: u64,
}

pub struct SolverOutput {
    pub seed: SolverSeed,
    pub final_cost: Option<u64>,
    pub log: String,
    pub solution: Solution,
}

impl SolverOutput {
    pub fn better_than(&self, other: Option<&SolverOutput>) -> bool {
        match (self.final_cost, other.and_then(|output| output.final_cost)) {
            (None, None) => false,
            (None, Some(_)) => false,
            (Some(_), None) => true,
            (Some(new), Some(old)) => new < old,
        }
    }
}

pub fn solve_once(problem: Problem, initial_solution: &Solution, seed: SolverSeed) -> SolverOutput {
    let mut rng = fastrand::Rng::with_seed(seed.rng_seed);
    let mut solution = initial_solution.clone();

    let mut current_cost = solution
        .evaluate(problem, None)
        .0
        .total_cost(problem.cost_config);
    let mut log = String::new();

    macro_rules! logln {
        ( $( $args:expr ),* ) => {{
            writeln!(&mut log, $( $args ),* ).unwrap();
            // println!($( $args ),* );
        }};
    }

    let start_time = Instant::now();
    logln!("Beginning solve with seed {seed:?}");

    logln!("Initial cost: {:?}", current_cost);
    if current_cost.is_none() {
        logln!("Warning: initial cost is None, you'll probably get a bad result!");
    }
    logln!("Breakdown of initial cost:");
    logln!(
        "{}",
        indent_lines(&solution.evaluate(problem, None).0.to_string(), 4)
    );

    let mut eval_buffer_helper = None;

    for round_num in 0..seed.num_rounds {
        let reporting_interval = 25000;
        if round_num % reporting_interval == 0 {
            logln!("After {round_num:9} rounds current cost is {current_cost:?}")
        }

        let mutation = match Mutation::make_random(problem, &solution, &mut rng) {
            Some(mutation) => mutation,
            None => continue,
        };

        solution.apply_mutation(&mutation);

        let new_evaluation = solution.evaluate(problem, eval_buffer_helper);
        eval_buffer_helper = Some(new_evaluation.1);

        let new_cost = match new_evaluation.0.total_cost(problem.cost_config) {
            Some(new_cost) => new_cost,
            None => {
                solution.reverse_mutation(&mutation);
                continue;
            }
        };

        let is_better = match current_cost {
            Some(current_cost) => {
                if new_cost < current_cost {
                    true
                } else {
                    let cost_diff = (new_cost - current_cost) as f32;
                    let progress = 1.0 - (round_num as f32) / (seed.num_rounds as f32);
                    let temperature = 5000.0 * progress.powi(6) + 0.1;
                    rng.f32() < (-cost_diff / temperature).exp()
                }
            }
            None => true,
        };

        if is_better {
            // logln!(
            //     "improved cost to {new_cost} (diff {diff:?}) on round {round_num}: {mutation:?}"
            // );
            current_cost = Some(new_cost);
        } else {
            solution.reverse_mutation(&mutation);
        }
    }

    logln!(
        "\nFinal cost: {:?}:\n{}",
        current_cost,
        indent_lines(&solution.evaluate(problem, None).0.to_string(), 4)
    );
    logln!(
        "\nSolving took {:.3} seconds",
        start_time.elapsed().as_secs_f32()
    );

    SolverOutput {
        seed,
        final_cost: current_cost,
        log,
        solution,
    }
}
