use crate::{
    availabilities::AvailabilityMatrix,
    costs::{Constraint, CostConfig, CostCount},
    instructor::{Instructor, InstructorId},
    session::{OverlapMatrix, Session, SessionId, SessionType},
    talloc::Availability,
    utils::TwoCombIter,
};

#[derive(Clone, Copy)]
pub struct Problem<'a> {
    pub sessions: &'a [Session],
    pub instructors: &'a [Instructor],
    pub availabilities: &'a AvailabilityMatrix,

    pub overlap_sharp: &'a OverlapMatrix,
    pub overlap_padded: &'a OverlapMatrix,
    pub overlap_same_day: &'a OverlapMatrix,

    pub cost_config: &'a CostConfig,

    pub initial_solution: &'a Solution,
}

#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub struct Solution {
    pub is_nontrivial: bool,
    pub assignment: Box<[Option<InstructorId>]>,
}

pub struct EvalBuffer {
    instructor_allocations: Vec<Vec<SessionId>>,
}

impl Solution {
    pub fn empty(num_sessions: usize, is_nontrivial: bool) -> Self {
        Solution {
            is_nontrivial,
            assignment: vec![None; num_sessions].into_boxed_slice(),
        }
    }

    pub fn new(assignment: Box<[Option<InstructorId>]>) -> Self {
        Solution {
            is_nontrivial: true,
            assignment,
        }
    }

    pub fn evaluate(
        &self,
        problem: Problem,
        buffer: Option<EvalBuffer>,
    ) -> (CostCount, EvalBuffer) {
        let mut costs = CostCount::new();

        let mut buffer = buffer.unwrap_or_else(|| EvalBuffer {
            instructor_allocations: vec![vec![]; problem.instructors.len()],
        });
        let instructor_allocations = &mut buffer.instructor_allocations;
        for alloc in instructor_allocations.iter_mut() {
            alloc.clear();
        }

        for (assignment, session) in self.assignment.iter().copied().zip(problem.sessions) {
            match assignment {
                Some(instructor_id) => {
                    let availability = problem
                        .availabilities
                        .get_availability(session.session_id, instructor_id);
                    costs.add_cost_1(match availability {
                        Availability::Impossible => Constraint::AssignedImpossible,
                        Availability::Dislike => Constraint::AssignedDislike,
                        Availability::Possible => Constraint::AssignedPossible,
                        Availability::Preferred => Constraint::AssignedPreferred,
                    });

                    instructor_allocations[instructor_id.raw_index()].push(session.session_id);
                }
                None => costs.add_cost_1(Constraint::UnassignedSession),
            }

            if problem
                .cost_config
                .should_count(Constraint::MismatchedInitialSolution)
            {
                if let Some(old_assignment) =
                    problem.initial_solution.assignment[session.session_id.raw_index()]
                {
                    if Some(old_assignment) != assignment {
                        costs.add_cost_1(Constraint::MismatchedInitialSolution);
                    }
                }
            }
        }

        for (instructor, instructor_allocation) in
            problem.instructors.iter().zip(instructor_allocations)
        {
            let num_classes = instructor_allocation.len();
            let num_tuts = instructor_allocation
                .iter()
                .filter(|session_id| {
                    matches!(
                        problem.sessions[session_id.raw_index()].typ,
                        SessionType::TutLab
                    )
                })
                .count();
            let num_labs = num_classes - num_tuts;

            let mut add_minmax_cost = |actual, min, max, below, above| {
                let actual = actual as u8;
                if actual < min {
                    costs.add_cost(below, min - actual);
                }
                if actual > max {
                    costs.add_cost(above, actual - max);
                }
            };

            add_minmax_cost(
                num_tuts,
                instructor.class_type_requirement.min_tutes,
                instructor.class_type_requirement.max_tutes,
                Constraint::BelowMinTut,
                Constraint::AboveMaxTut,
            );
            add_minmax_cost(
                num_labs,
                instructor.class_type_requirement.min_lab_assists,
                instructor.class_type_requirement.max_lab_assists,
                Constraint::BelowMinLab,
                Constraint::AboveMaxLab,
            );
            add_minmax_cost(
                num_classes,
                instructor.class_type_requirement.min_total_classes,
                instructor.class_type_requirement.max_total_classes,
                Constraint::BelowMinClass,
                Constraint::AboveMaxClass,
            );

            for (session_1, session_2) in TwoCombIter::new(instructor_allocation) {
                if problem.overlap_sharp.is_overlap(session_1, session_2) {
                    costs.add_cost_1(Constraint::DirectOverlap)
                } else if problem.cost_config.should_count(Constraint::PaddedOverlap)
                    && problem.overlap_padded.is_overlap(session_1, session_2)
                {
                    costs.add_cost_1(Constraint::PaddedOverlap)
                } else if problem.cost_config.should_count(Constraint::SameDayOverlap)
                    && problem.overlap_same_day.is_overlap(session_1, session_2)
                {
                    costs.add_cost_1(Constraint::SameDayOverlap)
                }
            }
        }

        (costs, buffer)
    }
}
