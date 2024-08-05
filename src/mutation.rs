use crate::{
    evaluator::{Problem, Solution},
    instructor::InstructorId,
    session::SessionId,
    talloc::Availability,
};

#[derive(Clone, Debug)]
pub enum Mutation {
    Mult(Box<Mutation>, Box<Mutation>),
    Remove(SessionId, InstructorId),
    Add(SessionId, InstructorId),
    Swap(SessionId, InstructorId, InstructorId),
    // Rotate(SessionId, SessionId),
}

impl Mutation {
    pub fn make_random(
        problem: Problem,
        solution: &Solution,
        rng: &mut fastrand::Rng,
    ) -> Option<Self> {
        if rng.u8(0..8) == 3 {
            return Some(Mutation::Mult(
                Box::new(Mutation::make_random(problem, solution, rng)?),
                Box::new(Mutation::make_random(problem, solution, rng)?),
            ));
        }

        let session_index = rng.usize(0..problem.sessions.len());
        let session_id = SessionId::from_index(session_index);

        let rand_instructor_for_session = |rng: &mut fastrand::Rng| {
            for _ in 0..16 {
                let instructor_id =
                    InstructorId::from_index(rng.usize(0..problem.instructors.len()));
                if problem
                    .availabilities
                    .get_availability(session_id, instructor_id)
                    != Availability::Impossible
                {
                    return Some(instructor_id);
                }
            }
            None
        };

        match solution.assignment[session_index] {
            Some(old_instructor) => {
                let decision = rng.u8(0..8);

                if decision == 1 {
                    Some(Mutation::Remove(session_id, old_instructor))
                } else if decision == 2 {
                    let other_session = rng.usize(0..problem.sessions.len());
                    if other_session == session_index {
                        return None;
                    }
                    let other_instructor = solution.assignment[other_session]?;

                    Some(Mutation::Mult(
                        Box::new(Mutation::Swap(session_id, old_instructor, other_instructor)),
                        Box::new(Mutation::Swap(
                            SessionId::from_index(other_session),
                            other_instructor,
                            old_instructor,
                        )),
                    ))
                } else {
                    let new_instructor = rand_instructor_for_session(rng)?;
                    Some(Mutation::Swap(session_id, old_instructor, new_instructor))
                }
            }
            None => {
                let instructor_id = rand_instructor_for_session(rng)?;
                Some(Mutation::Add(session_id, instructor_id))
            }
        }
    }
}

impl Solution {
    pub fn apply_mutation(&mut self, mutation: &Mutation) {
        match mutation {
            Mutation::Mult(a, b) => {
                self.apply_mutation(a);
                self.apply_mutation(b);
            }
            Mutation::Remove(session, _removed) => self.assignment[session.raw_index()] = None,
            Mutation::Add(session, instructor) => {
                self.assignment[session.raw_index()] = Some(*instructor)
            }
            Mutation::Swap(session, _old, new) => self.assignment[session.raw_index()] = Some(*new),
            // Mutation::Rotate(a, b) => {
            //     let a = a.raw_index();
            //     let b = b.raw_index();
            //     self.assignment.swap(a, b);
            // }
        }
    }

    pub fn reverse_mutation(&mut self, mutation: &Mutation) {
        match mutation {
            Mutation::Mult(a, b) => {
                self.reverse_mutation(b);
                self.reverse_mutation(a);
            }
            Mutation::Remove(session, removed) => {
                self.assignment[session.raw_index()] = Some(*removed)
            }
            Mutation::Add(session, _added) => self.assignment[session.raw_index()] = None,
            Mutation::Swap(session, old, _new) => self.assignment[session.raw_index()] = Some(*old),
            // Mutation::Rotate(a, b) => {
            //     let a = a.raw_index();
            //     let b = b.raw_index();
            //     self.assignment.swap(a, b);
            // }
        }
    }
}
