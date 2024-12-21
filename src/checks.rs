use crate::{
    costs::Constraint,
    evaluator::Problem,
    instructor::{ClassTypeRequirement, Instructor},
    session::SessionType,
};

#[allow(non_snake_case)]
fn check_instructor_class_reqs(instructor: &Instructor) {
    let zid = &instructor.zid;
    let name = &instructor.name;

    let minT = instructor.class_type_requirement.min_tutes;
    let maxT = instructor.class_type_requirement.max_tutes;
    let minA = instructor.class_type_requirement.min_lab_assists;
    let maxA = instructor.class_type_requirement.max_lab_assists;
    let minC = instructor.class_type_requirement.min_total_classes;
    let maxC = instructor.class_type_requirement.max_total_classes;

    macro_rules! check_constraint {
        ($cond:expr) => {
            if !$cond {
                println!(
                    "Warning! Bad constraints for {zid} ({name}): Condition `{}` violated",
                    stringify!($cond)
                );
            }
        };
    }

    check_constraint!(minT <= maxT);
    check_constraint!(minA <= maxA);
    check_constraint!(minC <= maxC);
    check_constraint!(minT + minA <= maxC);
    check_constraint!(minC <= maxA + maxT);

    check_constraint!(minT + minA <= minC);
    check_constraint!(maxC <= maxA + maxT);
}

#[allow(non_snake_case)]
pub fn check_problem(problem: Problem) {
    for instructor in problem.instructors {
        check_instructor_class_reqs(instructor);
    }

    let total_actual_tuts = problem
        .sessions
        .iter()
        .filter(|session| matches!(session.typ, SessionType::TutLab))
        .count();
    let total_actual_labs = problem
        .sessions
        .iter()
        .filter(|session| matches!(session.typ, SessionType::LabAssist))
        .count();
    let total_actual_classes = problem.sessions.len();

    let sum_requirement = |f: fn(&ClassTypeRequirement) -> u8| {
        problem
            .instructors
            .iter()
            .map(|instructor| f(&instructor.class_type_requirement) as usize)
            .sum()
    };

    let sum_minT = sum_requirement(|r| r.min_tutes);
    let sum_maxT = sum_requirement(|r| r.max_tutes);
    let sum_minA = sum_requirement(|r| r.min_lab_assists);
    let sum_maxA = sum_requirement(|r| r.max_lab_assists);
    let sum_minC = sum_requirement(|r| r.min_total_classes);
    let sum_maxC = sum_requirement(|r| r.max_total_classes);

    macro_rules! check_constraint {
        ($a:ident $comparison:tt $b:ident, $resolution:expr) => {
            if !($a $comparison $b) {
                println!(
                    "Warning! Condition `{}` violated: you probably want to {}\nNote {} = {} and {} = {}",
                    stringify!($a $comparison $b),
                    $resolution,
                    stringify!($a), $a,
                    stringify!($b), $b,
                );
            }
        };
    }

    check_constraint!(
        sum_minT <= total_actual_tuts,
        "decrease some of the instructor's minT values"
    );
    check_constraint!(
        total_actual_tuts <= sum_maxT,
        "increase some of the instructor's maxT values or add more instructors"
    );

    check_constraint!(
        sum_minA <= total_actual_labs,
        "decrease some of the instructor's minA values"
    );
    check_constraint!(
        total_actual_labs <= sum_maxA,
        "increase some of the instructor's minA values or add more instructors"
    );

    check_constraint!(
        sum_minC <= total_actual_classes,
        "decrease some of the instructor's minC values"
    );
    check_constraint!(
        total_actual_classes <= sum_maxC,
        "increase some of the instructor's maxC values or add more instructors"
    );

    if problem
        .cost_config
        .should_count(Constraint::MismatchedInitialSolution)
        && !problem.initial_solution.is_nontrivial
    {
        println!("Warning: mismatched_initial_solution used without an explicit initial solution!");
    }
}
