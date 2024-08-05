use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use anyhow::{anyhow, Context, Result};

use crate::{evaluator::Problem, solver::SolverOutput, utils::indent_lines};

impl<'a> Problem<'a> {
    pub fn details(&self) -> String {
        let mut result = String::new();

        result.push_str("Sessions:\n");
        result.push_str(&indent_lines(&format!("{:#?}", self.sessions), 4));

        result.push_str("\nInstructors:\n");
        result.push_str(&indent_lines(&format!("{:#?}", self.instructors), 4));

        result.push_str("\nAvailabilities:\n");
        result.push_str(&indent_lines(
            &self
                .availabilities
                .make_availability_report(self.sessions, self.instructors),
            4,
        ));

        result.push_str("\nDirect overlaps:\n");
        result.push_str(&indent_lines(
            &self.overlap_sharp.summarise(self.sessions),
            4,
        ));

        result.push_str("\nCosts:\n");
        result.push_str(&indent_lines(&format!("{:#?}", self.cost_config), 4));

        result
    }
}

static OUTPUTTER_MUTEX: Mutex<()> = Mutex::new(());

pub fn output_solution(problem: Problem, output: &SolverOutput) -> Result<()> {
    let outputter_guard = OUTPUTTER_MUTEX.lock().unwrap();

    let new_output_dir: &Path = &(0..)
        .filter_map(|disambiguator| {
            let hostname = hostname::get()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|_| "out".into());

            let output_dir = PathBuf::from("output").join(format!("{hostname}-{disambiguator:06}"));
            if !output_dir.exists() {
                Some(output_dir)
            } else {
                None
            }
        })
        .next()
        .unwrap();

    for output_dir in [new_output_dir, &PathBuf::from("output").join("latest")] {
        // slight race with creation in another process but that doesn't matter
        fs::create_dir_all(output_dir)
            .with_context(|| anyhow!("failed to create directory {}", output_dir.display()))?;

        fs::write(output_dir.join("solver_log.txt"), &output.log).with_context(|| {
            format!(
                "failed to write to {}",
                output_dir.join("solver_log.txt").display()
            )
        })?;

        fs::write(output_dir.join("problem.txt"), problem.details()).with_context(|| {
            format!(
                "failed to write to {}",
                output_dir.join("solver_log.txt").display()
            )
        })?;
    }

    println!(
        "New output in {} (cost {:?}, from {:?})",
        new_output_dir.display(),
        output.final_cost,
        output.seed
    );

    drop(outputter_guard);
    Ok(())
}
