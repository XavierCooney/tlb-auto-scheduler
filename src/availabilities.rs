use std::fmt::Write as _;

use anyhow::{anyhow, Context, Result};

use crate::{
    instructor::{Instructor, InstructorId},
    session::{Session, SessionId},
    talloc::{Availability, TallocApplication, TallocApps},
};

pub struct AvailabilityMatrix {
    num_instructors: usize,
    availability_session_x_instructor: Vec<Availability>,
}

fn check_availability(application: TallocApplication, session: &Session) -> Option<Availability> {
    (0..session.duration.hours())
        .map(|hour_offset| {
            application.get_availability(
                session.day,
                session.start_time.add_hr(hour_offset),
                session.mode,
            )
        })
        .min()
        .flatten()
}

impl AvailabilityMatrix {
    pub fn build(
        instructors: &[Instructor],
        sessions: &[Session],
        applications: &TallocApps,
    ) -> Result<AvailabilityMatrix> {
        let mut availability_session_x_instructor =
            Vec::with_capacity(instructors.len() * sessions.len());

        for session in sessions.iter() {
            for instructor in instructors.iter() {
                let application =
                    applications
                        .get_application(&instructor.zid)
                        .with_context(|| {
                            format!("{} does not have a talloc application!", instructor.zid)
                        })?;

                availability_session_x_instructor.push(
                    check_availability(application, session).with_context(|| {
                        anyhow!(
                            "failed to lookup {}'s availability for {}",
                            instructor.zid,
                            session.class_name
                        )
                    })?,
                );
            }
        }

        Ok(AvailabilityMatrix {
            num_instructors: instructors.len(),
            availability_session_x_instructor,
        })
    }

    pub fn get_availability(&self, session: SessionId, instructor: InstructorId) -> Availability {
        self.availability_session_x_instructor
            [session.raw_index() * self.num_instructors + instructor.raw_index()]
    }

    pub fn set_availability(
        &mut self,
        session: SessionId,
        instructor: InstructorId,
        updated: Availability,
    ) {
        self.availability_session_x_instructor
            [session.raw_index() * self.num_instructors + instructor.raw_index()] = updated;
    }

    pub fn make_availability_report(
        &self,
        sessions: &[Session],
        instructors: &[Instructor],
    ) -> String {
        let mut report = String::new();

        for instructor in instructors {
            writeln!(
                &mut report,
                "{} ({}) availabilities:",
                instructor.name, instructor.zid
            )
            .unwrap();
            for availability in [
                Availability::Impossible,
                Availability::Dislike,
                Availability::Possible,
                Availability::Preferred,
            ] {
                let matching_sessions = sessions
                    .iter()
                    .filter(|session| {
                        self.get_availability(session.session_id, instructor.instructor_id)
                            == availability
                    })
                    .map(|session| session.short_description())
                    .collect::<Vec<_>>();
                writeln!(
                    &mut report,
                    "    {availability:?} ({} total): {}{}",
                    matching_sessions.len(),
                    matching_sessions.join(", "),
                    if matching_sessions.is_empty() {
                        "none!"
                    } else {
                        ""
                    }
                )
                .unwrap();
            }
        }

        report
    }
}
