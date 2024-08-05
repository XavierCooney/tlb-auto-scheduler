use std::fmt::Write as _;

use bit_set::BitSet;

use crate::{
    classes::{Class, Mode, LAB_DURATION_HOURS, TUT_DURATION_HOURS},
    utils::{Day, SessionDuration, TimeOfDay},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionType {
    TutLab,
    LabAssist,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SessionId(u16);

impl SessionId {
    pub fn raw_index(self) -> usize {
        self.0 as _
    }

    pub fn from_index(index: usize) -> Self {
        SessionId(index as _)
    }
}

#[derive(Debug)]
pub struct Session {
    pub session_id: SessionId,
    pub day: Day,
    pub start_time: TimeOfDay,
    pub duration: SessionDuration,
    pub typ: SessionType,
    pub mode: Mode,
    pub class_name: Box<str>,
}

fn class_to_sessions(class: &Class) -> Vec<Session> {
    let mut sessions = Vec::new();

    if !class.ignore_tut {
        sessions.push(Session {
            session_id: SessionId::default(),
            day: class.day,
            start_time: class.start,
            duration: SessionDuration::new(TUT_DURATION_HOURS + LAB_DURATION_HOURS),
            typ: SessionType::TutLab,
            mode: class.mode,
            class_name: class.name.clone().into(),
        });
    }

    if !class.ignore_lab {
        sessions.push(Session {
            session_id: SessionId::default(),
            day: class.day,
            start_time: class.start.add_hr(TUT_DURATION_HOURS),
            duration: SessionDuration::new(LAB_DURATION_HOURS),
            typ: SessionType::LabAssist,
            mode: class.mode,
            class_name: class.name.clone().into(),
        });
    }

    sessions
}

pub fn classes_to_sessions(classes: &[Class]) -> Vec<Session> {
    classes
        .iter()
        .flat_map(class_to_sessions)
        .enumerate()
        .map(|(idx, mut session)| {
            session.session_id = SessionId(idx as _);
            session
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
pub enum OverlapRequirement {
    Sharp,
    WithPadding,
    SameDay,
}

impl Session {
    fn overlaps_with(&self, other: &Session, mut requirement: OverlapRequirement) -> bool {
        if self.day != other.day {
            return false;
        }

        if matches!(requirement, OverlapRequirement::SameDay) {
            return true;
        }

        if self.mode != other.mode {
            // if going from online to in-person or vica versa give some padding
            requirement = OverlapRequirement::WithPadding;
        }

        // self ends before other
        if self.start_time.add_duration(self.duration) < other.start_time {
            return false;
        }
        if matches!(requirement, OverlapRequirement::Sharp)
            && self.start_time.add_duration(self.duration) <= other.start_time
        {
            return false;
        }

        // other ends before self
        if other.start_time.add_duration(self.duration) < self.start_time {
            return false;
        }
        if matches!(requirement, OverlapRequirement::Sharp)
            && other.start_time.add_duration(self.duration) <= self.start_time
        {
            return false;
        }

        true
    }

    pub fn short_description(&self) -> String {
        format!(
            "{} {}",
            self.class_name,
            match self.typ {
                SessionType::TutLab => "tut+lab",
                SessionType::LabAssist => "lab",
            }
        )
    }
}

pub struct OverlapMatrix {
    num_sessions: usize,
    overlaps: BitSet,
}

// A precomputed store of which sessions overlap with each other
impl OverlapMatrix {
    fn get_overlap_index(num_sessions: usize, first: SessionId, second: SessionId) -> usize {
        (first.0 as usize) * num_sessions + (second.0 as usize)
    }

    pub fn from_sessions(sessions: &[Session], requirement: OverlapRequirement) -> OverlapMatrix {
        let num_sessions = sessions.len();
        let mut overlaps = BitSet::with_capacity(num_sessions * num_sessions);

        for session_1 in sessions {
            for session_2 in sessions {
                if session_1.session_id == session_2.session_id {
                    continue;
                }

                if session_1.overlaps_with(session_2, requirement) {
                    overlaps.insert(Self::get_overlap_index(
                        num_sessions,
                        session_1.session_id,
                        session_2.session_id,
                    ));
                }
            }
        }

        OverlapMatrix {
            num_sessions,
            overlaps,
        }
    }

    pub fn summarise(&self, sessions: &[Session]) -> String {
        let mut result = String::new();

        for overlap_index in self.overlaps.iter() {
            let session_1 = overlap_index / self.num_sessions;
            let session_2 = overlap_index % self.num_sessions;
            if session_1 < session_2 {
                writeln!(
                    &mut result,
                    "{} and {} overlap",
                    sessions[session_1].short_description(),
                    sessions[session_2].short_description()
                )
                .unwrap();
            }
        }

        result
    }

    pub fn is_overlap(&self, session_1: SessionId, session_2: SessionId) -> bool {
        self.overlaps.contains(Self::get_overlap_index(
            self.num_sessions,
            session_1,
            session_2,
        ))
    }
}
