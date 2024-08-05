use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use itertools::Itertools;

use crate::{
    evaluator::{Problem, Solution},
    session::SessionType,
    tsv::Tsv,
};

pub fn get_initial_solution(initial_tsv_path: &Path, problem: Problem) -> Result<Solution> {
    if !initial_tsv_path.is_file() {
        println!("Using empty initial solution");
        Ok(Solution::empty(problem.sessions.len()))
    } else {
        let mut assignment = vec![None; problem.sessions.len()];

        for row in &Tsv::read_from_path(initial_tsv_path)? {
            let class_name = row.get("class")?.trim();

            let mut set_session = |field_name, typ| -> Result<_> {
                let zid = row.get(field_name)?.trim();
                if zid == "-" {
                    return Ok(());
                };

                let (instructor_id,) = problem
                    .instructors
                    .iter()
                    .filter(|instructor| instructor.zid == zid)
                    .map(|instructor| instructor.instructor_id)
                    .collect_tuple()
                    .with_context(|| {
                        anyhow!("cannot find instructor {zid} for class {class_name}")
                    })?;

                let (session_id,) = problem
                    .sessions
                    .iter()
                    .filter(|session| {
                        session.class_name.as_ref() == class_name && session.typ == typ
                    })
                    .map(|session| session.session_id)
                    .collect_tuple()
                    .with_context(|| anyhow!("cannot find class {class_name} {field_name}"))?;

                if assignment[session_id.raw_index()]
                    .is_some_and(|current| current != instructor_id)
                {
                    bail!("class {class_name} {field_name} already has an instuctor assigned!");
                }

                assignment[session_id.raw_index()] = Some(instructor_id);

                Ok(())
            };

            set_session("tutor", SessionType::TutLab)?;
            set_session("lab assist", SessionType::LabAssist)?;
        }

        Ok(Solution::new(assignment.into_boxed_slice()))
    }
}
