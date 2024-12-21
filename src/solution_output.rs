use std::{
    fmt::Write,
    fs::{self},
    path::{Path, PathBuf},
    sync::Mutex,
};

use anyhow::{anyhow, Context, Result};
use itertools::Itertools;

use crate::{
    evaluator::{Problem, Solution},
    instructor::InstructorId,
    session::SessionType,
    solver::SolverOutput,
    utils::indent_lines,
};

impl Problem<'_> {
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

pub fn instructor_stats_from_solution(problem: &Problem, solution: &Solution) -> Result<String> {
    let mut output = String::from("Instructor allocation stats:\n");

    for instructor in problem.instructors {
        writeln!(output, "{} ({})", instructor.name, instructor.zid)?;

        let class_constraints = &instructor.class_type_requirement;
        writeln!(
            output,
            "    Had minT = {}, maxT = {}, minA = {}, maxA = {}, minC = {}, maxC = {}",
            class_constraints.min_tutes,
            class_constraints.max_tutes,
            class_constraints.min_lab_assists,
            class_constraints.max_lab_assists,
            class_constraints.min_total_classes,
            class_constraints.max_total_classes
        )?;

        let matching_sessions = problem
            .sessions
            .iter()
            .filter(|session| {
                solution.assignment[session.session_id.raw_index()]
                    == Some(instructor.instructor_id)
            })
            .collect::<Vec<_>>();

        let actual_tutes = matching_sessions
            .iter()
            .filter(|session| matches!(session.typ, SessionType::TutLab))
            .count();

        let actual_labs = matching_sessions
            .iter()
            .filter(|session| matches!(session.typ, SessionType::LabAssist))
            .count();

        writeln!(
            output,
            "    Actual tutes = {}, actual labs = {}, actual classes = {}",
            actual_tutes,
            actual_labs,
            matching_sessions.len()
        )?;

        for session in matching_sessions {
            let var_name = writeln!(
                output,
                "    {} {}: {:?}",
                session.class_name,
                match session.typ {
                    SessionType::TutLab => "T",
                    SessionType::LabAssist => "L",
                },
                problem
                    .availabilities
                    .get_availability(session.session_id, instructor.instructor_id)
            );
            var_name?;
        }
    }

    Ok(output)
}

fn solution_output_tsv(problem: &Problem, solution: &Solution) -> String {
    String::from("class\ttype\tzid\tname\n")
        + &problem
            .sessions
            .iter()
            .map(|session| {
                let session_id = session.session_id;
                let session = &problem.sessions[session_id.raw_index()];

                let assigned = solution.assignment[session_id.raw_index()];

                let instructor =
                    assigned.map(|instructor_id| &problem.instructors[instructor_id.raw_index()]);

                format!(
                    "{}\t{}\t{}\t{}",
                    session.class_name,
                    match session.typ {
                        SessionType::TutLab => "tut+lab",
                        SessionType::LabAssist => "lab",
                    },
                    instructor
                        .map(|instructor| instructor.zid.as_str())
                        .unwrap_or("-"),
                    instructor
                        .map(|instructor| instructor.name.as_str())
                        .unwrap_or("-"),
                )
            })
            .join("\n")
        + "\n"
}

fn show_diff(problem: &Problem, solution: &Solution) -> String {
    let mut output = String::from("Difference from initial solution:\n");

    for session in problem.sessions {
        let session_id = session.session_id;
        let initial_assignment = problem.initial_solution.assignment[session_id.raw_index()];
        let new_assignment = solution.assignment[session_id.raw_index()];

        let show_instructor = |instructor_id: Option<InstructorId>| match instructor_id {
            Some(instructor_id) => {
                let instructor = &problem.instructors[instructor_id.raw_index()];
                format!("{} ({})", instructor.name, instructor.zid)
            }
            None => String::from("no assignment"),
        };

        if initial_assignment != new_assignment {
            output.push_str(&format!(
                "    {}: {} ==> {}\n",
                session.short_description(),
                show_instructor(initial_assignment),
                show_instructor(new_assignment)
            ));
        }
    }

    output
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

        fs::write(
            output_dir.join("solution.tsv"),
            solution_output_tsv(&problem, &output.solution),
        )?;

        fs::write(
            output_dir.join("instructor_stats.txt"),
            instructor_stats_from_solution(&problem, &output.solution)?,
        )?;

        if problem.initial_solution.is_nontrivial {
            fs::write(
                output_dir.join("diff.txt"),
                show_diff(&problem, &output.solution),
            )?;
        }
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
