use anyhow::{anyhow, bail, Context, Result};
use itertools::Itertools;

use crate::{
    tsv::{Tsv, TsvRow},
    utils::{parse_bool_input, Day, TimeOfDay},
};

pub const TUT_DURATION_HOURS: u8 = 1;
pub const LAB_DURATION_HOURS: u8 = 2;

#[derive(Debug)]
pub struct Class {
    pub name: String,
    pub day: Day,
    pub start: TimeOfDay,
    pub mode: Mode,

    pub ignore_tut: bool,
    pub ignore_lab: bool,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Mode {
    F2F,
    Online,
}

fn extract_meeting(meeting: &str) -> Option<(Day, TimeOfDay, TimeOfDay, Mode)> {
    let (before_paren, after_paren) = meeting.split_once(" (")?;
    let (day, time) = before_paren.split_once(' ')?;
    let (_weeks, location) = after_paren.strip_suffix(')')?.split_once(", ")?;

    let (start, end) = if time.contains('-') {
        let (star_raw, end_raw) = time.split_once('-')?;
        (star_raw.parse().ok()?, end_raw.parse().ok()?)
    } else {
        let start: TimeOfDay = time.parse().ok()?;
        (start, start.add_hr(1))
    };

    Some((
        day.parse().ok()?,
        start,
        end,
        if location.eq_ignore_ascii_case("online") {
            Mode::Online
        } else {
            Mode::F2F
        },
    ))
}

fn extract_and_check_meetings(times: &str) -> Result<(Day, TimeOfDay, Mode)> {
    let (tut_meeting, lab_meeting) = times
        .split("; ")
        .collect_tuple()
        .ok_or_else(|| anyhow!("class time {times:?} doesn't have two meetings"))?;

    let (tut_day, tut_start, tut_end, tut_mode) = extract_meeting(tut_meeting)
        .ok_or_else(|| anyhow!("bad tutorial meeting {tut_meeting:?}"))?;

    let (lab_day, lab_start, lab_end, lab_mode) =
        extract_meeting(lab_meeting).ok_or_else(|| anyhow!("bad lab meeting {lab_meeting:?}"))?;

    if tut_day != lab_day {
        bail!("mismatch between tut and lab days");
    } else if tut_start.add_hr(TUT_DURATION_HOURS) != tut_end {
        bail!("tut is the wrong length");
    } else if tut_end != lab_start {
        bail!("lab is not immediately after tut");
    } else if lab_start.add_hr(LAB_DURATION_HOURS) != lab_end {
        bail!("lab is the wrong length");
    } else if lab_mode != tut_mode {
        bail!("tut and lab mode disagree");
    } else {
        Ok((tut_day, tut_start, tut_mode))
    }
}

impl<'a> TryFrom<TsvRow<'a>> for Class {
    type Error = anyhow::Error;

    fn try_from(row: TsvRow<'a>) -> Result<Self> {
        let name = String::from(row.get("section")?.trim());

        let class_type = row.get("type")?.trim();
        if class_type != "TLB" {
            bail!("bad class type {class_type:?} for {name}, expected \"TLB\"");
        }

        let status = row.get("status")?.trim();
        if status != "Open" && status != "Full" {
            bail!("bad class status {status:?} for {name}, either manually change to \"Open\" or remove it");
        }

        let (day, start, mode) = extract_and_check_meetings(row.get("times")?.trim())
            .with_context(|| format!("error while extracting meeting info for {name}"))?;

        let get_ignore = |field_name: &str| {
            Ok(match row.get(field_name) {
                Ok(field_val) => {
                    if field_val.trim().is_empty() {
                        false
                    } else {
                        return parse_bool_input(field_val.trim());
                    }
                }
                Err(_) => false,
            })
        };

        Ok(Class {
            name,
            day,
            start,
            mode,
            ignore_tut: get_ignore("ignore tut")?,
            ignore_lab: get_ignore("ignore lab")?,
        })
    }
}

impl Class {
    pub fn vec_from_tsv(tsv: &Tsv) -> Result<Vec<Class>> {
        tsv.into_iter().map(Class::try_from).collect()
    }
}
