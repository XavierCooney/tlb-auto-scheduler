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
        Ok(Solution::empty(problem.sessions.len(), false))
    } else {
        let mut assignment = vec![None; problem.sessions.len()];

        for row in &Tsv::read_from_path(initial_tsv_path)? {
            let class_name = row.get("class")?;
            let class_type = match row.get("type")? {
                "tut+lab" => SessionType::TutLab,
                "lab" => SessionType::LabAssist,
                bad_type => bail!("bad session type {:?} for {class_name}", bad_type),
            };
            let instructor_zid = row.get("zid")?;

            if instructor_zid == "-" {
                continue;
            };

            let (instructor_id,) = problem
                .instructors
                .iter()
                .filter(|instructor| instructor.zid == instructor_zid)
                .map(|instructor| instructor.instructor_id)
                .collect_tuple()
                .with_context(|| {
                    anyhow!("cannot find instructor {instructor_zid} for class {class_name}")
                })?;

            let (session_id,) = problem
                .sessions
                .iter()
                .filter(|session| {
                    session.class_name.as_ref() == class_name && session.typ == class_type
                })
                .map(|session| session.session_id)
                .collect_tuple()
                .with_context(|| anyhow!("cannot find class {class_name} {class_type:?}"))?;

            if assignment[session_id.raw_index()].is_some_and(|current| current != instructor_id) {
                bail!("class {class_name} {class_type:?} already has an instuctor assigned!");
            }

            assignment[session_id.raw_index()] = Some(instructor_id);
        }

        Ok(Solution::new(assignment.into_boxed_slice()))
    }
}
