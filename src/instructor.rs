use crate::{
    errors::{Error, Result},
    tsv::{Tsv, TsvRow},
    utils::parse_bool_input,
};

const ZID_FIELD: &str = "zid";

#[derive(Debug)]
pub struct Instructor {
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

impl<'a> TryFrom<TsvRow<'a>> for Instructor {
    type Error = Box<Error>;

    fn try_from(row: TsvRow) -> Result<Self> {
        let name = row.get("name")?.into();
        let zid = row.get(ZID_FIELD)?.into();

        let class_type_requirement = row.try_into()?;
        let seniority = row.try_into()?;

        Ok(Instructor {
            name,
            zid,
            class_type_requirement,
            seniority,
        })
    }
}

impl Instructor {
    pub fn vec_from_tsv(tsv: &Tsv) -> Result<Vec<Instructor>> {
        tsv.into_iter().map(Instructor::try_from).collect()
    }
}

impl<'a> TryFrom<TsvRow<'a>> for ClassTypeRequirement {
    type Error = Box<Error>;

    fn try_from(row: TsvRow) -> Result<Self> {
        let zid = row.get(ZID_FIELD)?;

        let get_requirement = |field: &str| {
            row.get(field)?.parse::<u8>().map_err(|err| {
                Box::new(Error::BadClassTypeRequirement {
                    zid: zid.into(),
                    field: field.into(),
                    err,
                })
            })
        };

        Ok(ClassTypeRequirement {
            min_tutes: get_requirement("minT")?,
            max_tutes: get_requirement("maxT")?,
            min_lab_assists: get_requirement("minA")?,
            max_lab_assists: get_requirement("maxA")?,
            min_total_classes: get_requirement("minC")?,
            max_total_classes: get_requirement("maxC")?,
        })
    }
}

impl<'a> TryFrom<TsvRow<'a>> for Option<TutorSeniority> {
    type Error = Box<Error>;

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