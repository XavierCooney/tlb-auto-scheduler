use std::path::PathBuf;

use anyhow::Result;
use availabilities::AvailabilityMatrix;
use checks::check_problem;
use clap::Parser;
use classes::{Class, Mode};
use costs::CostConfig;
use evaluator::Problem;
use initial_solution::get_initial_solution;
use instructor::Instructor;
use session::{classes_to_sessions, OverlapMatrix, OverlapRequirement};
use solution_output::output_solution;
use solver::{solve_once, SolverSeed};
use talloc::TallocApps;
use tsv::Tsv;

mod availabilities;
mod checks;
mod classes;
mod costs;
mod evaluator;
mod initial_solution;
mod instructor;
mod mutation;
mod session;
mod solution_output;
mod solver;
mod talloc;
mod tsv;
mod utils;

#[derive(Debug, clap::Parser)]
struct Args {
    config_dir: PathBuf,
    #[arg(long)]
    ignore_no_talloc: bool,
}

impl Args {
    fn get_file_path(&self, filename: &str) -> PathBuf {
        self.config_dir.join(filename)
    }
}

fn main_impl() -> Result<()> {
    let args = Args::parse();

    let instructors = Instructor::vec_from_tsv(&Tsv::read_from_path(
        &args.get_file_path("instructors.tsv"),
    )?)?;
    println!("Loaded {} instructors", instructors.len());

    let classes = Class::vec_from_tsv(&Tsv::read_from_path(&args.get_file_path("classes.tsv"))?)?;
    println!(
        "Loaded {} classes ({} face to face, {} online)",
        classes.len(),
        classes
            .iter()
            .filter(|class| class.mode == Mode::F2F)
            .count(),
        classes
            .iter()
            .filter(|class| class.mode == Mode::Online)
            .count()
    );

    let sessions = classes_to_sessions(&classes);

    let overlaps_sharp = OverlapMatrix::from_sessions(&sessions, OverlapRequirement::Sharp);
    let overlaps_padded = OverlapMatrix::from_sessions(&sessions, OverlapRequirement::WithPadding);
    let overlaps_same_day = OverlapMatrix::from_sessions(&sessions, OverlapRequirement::SameDay);

    let applications = TallocApps::fetch(
        &args.get_file_path("talloc_cache.json"),
        args.ignore_no_talloc,
    )?;

    for instructor in &instructors {
        if applications
            .get_application(&instructor.zid)
            .is_some_and(|app| app.is_default())
        {
            println!(
                "Using 'all impossible' default application for {} ({})",
                instructor.zid, instructor.name
            )
        }
    }

    let availabilities = AvailabilityMatrix::build(&instructors, &sessions, &applications)?;

    drop(applications);

    let cost_config = CostConfig::read_from_toml(&args.get_file_path("costs.toml"))?;

    let problem = Problem {
        sessions: &sessions,
        instructors: &instructors,
        availabilities: &availabilities,
        overlap_sharp: &overlaps_sharp,
        overlap_padded: &overlaps_padded,
        overlap_same_day: &overlaps_same_day,
        cost_config: &cost_config,
    };
    println!();
    check_problem(problem);

    let empty_solution = get_initial_solution(&args.get_file_path("initial.tsv"), problem)?;

    let mut best_result = solve_once(
        problem,
        empty_solution.clone(),
        SolverSeed {
            // num_rounds: 50000000,
            num_rounds: 1000000,
            rng_seed: 4,
        },
    );
    output_solution(problem, &best_result)?;

    for i in 1..=10 {
        let seed = SolverSeed {
            num_rounds: 20000000,
            rng_seed: i + 100,
        };
        let new_result = solve_once(problem, empty_solution.clone(), seed);
        if new_result.better_than(&best_result) {
            output_solution(problem, &new_result)?;
            best_result = new_result;
        } else {
            println!(
                "Did not get improvement from {seed:?} (cost {:?})",
                new_result.final_cost
            )
        }
    }

    Ok(())
}

fn main() {
    match main_impl() {
        Ok(_) => {}
        Err(err) => println!("\nError: {:?}", err),
    }
}
