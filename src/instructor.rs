use anyhow::{anyhow, Context, Result};

use crate::{
    tsv::{Tsv, TsvRow},
    utils::parse_bool_input,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InstructorId(u16);

impl InstructorId {
    pub fn raw_index(self) -> usize {
        self.0 as _
    }

    pub fn from_index(index: usize) -> Self {
        InstructorId(index as _)
    }
}

#[derive(Debug)]
pub struct Instructor {
    pub instructor_id: InstructorId,
    pub name: String,
    pub zid: String,
    pub class_type_requirement: ClassTypeRequirement,

    pub seniority: Option<TutorSeniority>,
}

#[derive(Debug)]
pub struct ClassTypeRequirement {
    pub min_tutes: u8,
    pub max_tutes: u8,
    pub min_lab_assists: u8,
    pub max_lab_assists: u8,
    pub min_total_classes: u8,
    pub max_total_classes: u8,
}

#[derive(Debug)]
pub struct TutorSeniority {
    pub is_senior_tutor: bool,
    pub is_new_tutor: bool,
}

impl TryFrom<TsvRow<'_>> for Option<Instructor> {
    type Error = anyhow::Error;

    fn try_from(row: TsvRow) -> Result<Self> {
        if let Ok(ignore) = row.get("ignore") {
            if !ignore.trim().is_empty()
                && parse_bool_input(ignore).context("bad ignore on instructor")?
            {
                return Ok(None);
            }
        }

        // instructor_id is set in Instructor::vec_from_tsv
        let instructor_id = InstructorId::default();

        let name = row.get("name")?.into();
        let zid = row.get("zid")?.into();

        let class_type_requirement = row
            .try_into()
            .with_context(|| anyhow!("could not parse class requirements for {zid} ({name})"))?;
        let seniority = row
            .try_into()
            .with_context(|| anyhow!("could not parse seniority status for {zid} ({name})"))?;

        Ok(Some(Instructor {
            instructor_id,
            name,
            zid,
            class_type_requirement,
            seniority,
        }))
    }
}

impl Instructor {
    pub fn vec_from_tsv(tsv: &Tsv) -> Result<Vec<Instructor>> {
        Ok(tsv
            .into_iter()
            .map(Option::<Instructor>::try_from)
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .enumerate()
            .map(|(idx, mut instructor)| {
                instructor.instructor_id = InstructorId(idx as _);
                instructor
            })
            .collect())
    }
}

impl TryFrom<TsvRow<'_>> for ClassTypeRequirement {
    type Error = anyhow::Error;

    fn try_from(row: TsvRow) -> Result<Self> {
        let get_requirement = |field: &str| {
            row.get(field)?
                .parse::<u8>()
                .with_context(|| anyhow!("could not parse value of field {field} as number"))
        };

        let get_requirement_or_default = |field: &str, default: u8| match row.get(field) {
            Err(_) | Ok("-") => Ok(default),
            Ok(val) => val
                .parse::<u8>()
                .with_context(|| anyhow!("could not parse value of field {field} as number")),
        };

        let min_tutes = get_requirement("minT")?;
        let max_tutes = get_requirement("maxT")?;
        let min_lab_assists = get_requirement("minA")?;
        let max_lab_assists = get_requirement("maxA")?;

        let min_total_classes = get_requirement_or_default("minC", min_tutes + min_lab_assists)?;
        let max_total_classes = get_requirement_or_default("maxC", max_tutes + max_lab_assists)?;

        Ok(ClassTypeRequirement {
            min_tutes,
            max_tutes,
            min_lab_assists,
            max_lab_assists,
            min_total_classes,
            max_total_classes,
        })
    }
}

impl TryFrom<TsvRow<'_>> for Option<TutorSeniority> {
    type Error = anyhow::Error;

    fn try_from(row: TsvRow) -> Result<Self> {
        let senior_tutor_raw = row.get("senior tutor");
        let new_tutor_raw = row.get("new tutor");

        Ok(match (senior_tutor_raw, new_tutor_raw) {
            (Ok(senior_tutor_raw), Ok(new_tutor_raw)) => {
                let is_senior_tutor = parse_bool_input(senior_tutor_raw)?;
                let is_new_tutor = parse_bool_input(new_tutor_raw)?;

                Some(TutorSeniority {
                    is_senior_tutor,
                    is_new_tutor,
                })
            }
            (Ok(_), Err(err)) => return Err(err),
            (Err(err), Ok(_)) => return Err(err),
            (Err(_), Err(_)) => None,
        })
    }
}
