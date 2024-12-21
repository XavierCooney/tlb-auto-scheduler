use std::{path::PathBuf, sync::Mutex};

use anyhow::{Context, Result};
use availabilities::AvailabilityMatrix;
use checks::check_problem;
use clap::Parser;
use classes::{Class, Mode};
use costs::CostConfig;
use evaluator::Problem;
use initial_solution::get_initial_solution;
use instructor::Instructor;
use overrides::apply_overrides;
use scoped_threadpool::Pool;
use session::{classes_to_sessions, OverlapMatrix, OverlapRequirement};
use solution_output::{instructor_stats_from_solution, output_solution};
use solver::{solve_once, SolverSeed};
use talloc::TallocApps;
use tsv::Tsv;
use utils::indent_lines;

mod availabilities;
mod checks;
mod classes;
mod costs;
mod evaluator;
mod initial_solution;
mod instructor;
mod mutation;
mod overrides;
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
    #[arg(long, default_value_t = 1)]
    cpus: u32,
    #[arg(long)]
    initial_costs: bool,
    #[arg(long)]
    start_seed: Option<u64>,
    #[arg(long, default_value_t = 20)]
    total_attempts: u64,
    #[arg(long, default_value_t = 75_000_000)]
    num_rounds: u64,
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

    let mut availabilities = AvailabilityMatrix::build(&instructors, &sessions, &applications)?;

    // the applications are pretty big, so free up some memory now
    drop(applications);

    let overrides_tsv_path = args.get_file_path("overrides.tsv");
    if overrides_tsv_path.exists() {
        apply_overrides(
            &Tsv::read_from_path(&overrides_tsv_path)?,
            &mut availabilities,
            &instructors,
            &sessions,
        )
        .context("Failed to process overrides")?;
    } else {
        println!("No overrides applied");
    }

    let cost_config = CostConfig::read_from_toml(&args.get_file_path("costs.toml"))?;

    let initial_solution =
        get_initial_solution(&args.get_file_path("initial.tsv"), &sessions, &instructors)
            .context("Failed to process initial solution\n")?;

    let problem = Problem {
        sessions: &sessions,
        instructors: &instructors,
        availabilities: &availabilities,
        overlap_sharp: &overlaps_sharp,
        overlap_padded: &overlaps_padded,
        overlap_same_day: &overlaps_same_day,
        cost_config: &cost_config,
        initial_solution: &initial_solution,
    };
    check_problem(problem);

    if args.initial_costs {
        println!(
            "\nBreakdown of initial solution:\n{}",
            indent_lines(&initial_solution.evaluate(problem, None).0.to_string(), 4)
        );
        print!(
            "{}",
            instructor_stats_from_solution(&problem, &initial_solution)?
        );
    }
    println!();

    let mut thread_pool = Pool::new(args.cpus);

    let best_result = &Mutex::new(None);
    let initial_solution = &initial_solution;

    let run_with_seed = |seed| {
        let new_result = solve_once(problem, initial_solution, seed);
        let mut best_result = best_result.lock().unwrap();

        if new_result.better_than(best_result.as_ref()) {
            output_solution(problem, &new_result).unwrap();
            *best_result = Some(new_result);
        } else {
            println!(
                "Did not get improvement from {seed:?} (cost {:?})",
                new_result.final_cost
            )
        }
    };

    thread_pool.scoped(|pool_scope| {
        println!("Starting solving...");

        if args.start_seed.is_none() {
            pool_scope.execute(move || {
                run_with_seed(SolverSeed {
                    num_rounds: args.num_rounds / 20,
                    rng_seed: 0,
                });
            });
        }

        for i in 0..args.total_attempts {
            pool_scope.execute(move || {
                run_with_seed(SolverSeed {
                    num_rounds: args.num_rounds,
                    rng_seed: args.start_seed.unwrap_or(1) + i,
                });
            });
        }
    });

    Ok(())
}

fn main() {
    match main_impl() {
        Ok(_) => {}
        Err(err) => println!("\nError: {:?}", err),
    }
}
