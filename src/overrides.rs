use anyhow::{bail, Context, Result};

use crate::{
    availabilities::AvailabilityMatrix,
    instructor::Instructor,
    session::{Session, SessionType},
    talloc::Availability,
    tsv::Tsv,
    utils::match_ignore_case,
};

impl Availability {
    fn from_english_name(name: &str) -> Option<Self> {
        match_ignore_case(
            name,
            &[
                (&["impossible"], Availability::Impossible),
                (&["dislike"], Availability::Dislike),
                (&["possible"], Availability::Possible),
                (&["preferred"], Availability::Preferred),
            ],
        )
    }
}

fn matches_spec(needle: &str, haystack: &str) -> bool {
    let haystack = haystack.trim();
    if haystack == "*" {
        return true;
    }

    haystack
        .split(',')
        .any(|possibility| possibility.eq_ignore_ascii_case(needle))
}

pub fn apply_overrides(
    overrides_tsv: &Tsv,
    availabilities: &mut AvailabilityMatrix,
    instructors: &[Instructor],
    sessions: &[Session],
) -> Result<()> {
    for row in overrides_tsv {
        let override_name = row.get("name")?;
        let zid = row.get("zid")?;
        let class_name = row.get("class")?;
        let class_type = row.get("type")?;

        let raw_availability = row.get("override")?;
        let availability =
            Availability::from_english_name(raw_availability).with_context(|| {
                format!("bad availability for override {override_name}: `{raw_availability}`")
            })?;

        let mut total_applied = 0;

        for instructor in instructors {
            if !matches_spec(&instructor.zid, zid) {
                continue;
            }

            for session in sessions {
                if !matches_spec(&session.class_name, class_name) {
                    continue;
                }

                let this_session_type_name = match session.typ {
                    SessionType::TutLab => "tut",
                    SessionType::LabAssist => "lab",
                };

                if !matches_spec(this_session_type_name, class_type) {
                    continue;
                }

                availabilities.set_availability(
                    session.session_id,
                    instructor.instructor_id,
                    availability,
                );

                total_applied += 1;
            }
        }

        if total_applied == 0 {
            bail!("Override {override_name} didn't apply to any sessions/instructors!")
        }

        println!("Override {override_name}: {total_applied} applied")
    }

    Ok(())
}
