use std::{result, str::FromStr};

use anyhow::{bail, Result};

pub fn match_ignore_case<T: Copy>(input: &str, cases: &[(&[&str], T)]) -> Option<T> {
    for (matches, value) in cases {
        if matches
            .iter()
            .any(|expected| expected.eq_ignore_ascii_case(input))
        {
            return Some(*value);
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Day {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
}

impl FromStr for Day {
    type Err = ();

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        match_ignore_case(
            s,
            &[
                (&["mon", "monday"], Day::Mon),
                (&["tue", "tuesday"], Day::Tue),
                (&["wed", "wednesday"], Day::Wed),
                (&["thu", "thursday"], Day::Thu),
                (&["fri", "friday"], Day::Fri),
            ],
        )
        .ok_or(())
    }
}

impl Day {
    pub fn short_lowercase(self) -> &'static str {
        match self {
            Day::Mon => "mon",
            Day::Tue => "tue",
            Day::Wed => "wed",
            Day::Thu => "thu",
            Day::Fri => "fri",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeOfDay(u8);

impl TimeOfDay {
    pub fn as_24_hours(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SessionDuration {
    hours: u8,
}

impl SessionDuration {
    pub fn new(hours: u8) -> SessionDuration {
        SessionDuration { hours }
    }

    pub fn hours(self) -> u8 {
        self.hours
    }
}

impl FromStr for TimeOfDay {
    type Err = ();

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        let stripped = s.strip_suffix(":00").unwrap_or(s);
        let time = stripped.parse().map_err(|_| ())?;
        if time < 24 {
            Ok(TimeOfDay(time))
        } else {
            Err(())
        }
    }
}

impl TimeOfDay {
    pub fn add_hr(self, hour: u8) -> Self {
        let new_time = self.0.saturating_add(hour);
        assert!(new_time < 24);
        TimeOfDay(new_time)
    }

    pub fn add_duration(self, duration: SessionDuration) -> Self {
        self.add_hr(duration.hours)
    }
}

pub fn parse_bool_input(value: &str) -> Result<bool> {
    let matches_any_ignore_ascii_case = |possibilities: &[&str]| {
        possibilities
            .iter()
            .any(|expected| value.eq_ignore_ascii_case(expected))
    };

    if matches_any_ignore_ascii_case(&["y", "yes", "true", "1"]) {
        return Ok(true);
    }

    if matches_any_ignore_ascii_case(&["n", "no", "false", "0"]) {
        return Ok(false);
    }

    bail!("could not parse {value:?} as a boolean")
}

pub struct TwoCombIter<'a, T> {
    slice: &'a [T],
    outer_index: usize,
    inner_index: usize,
}

impl<'a, T> TwoCombIter<'a, T> {
    pub fn new(slice: &'a [T]) -> Self {
        TwoCombIter {
            slice,
            outer_index: 1,
            inner_index: 0,
        }
    }
}

impl<T> Iterator for TwoCombIter<'_, T>
where
    T: Copy,
{
    type Item = (T, T);

    fn next(&mut self) -> Option<Self::Item> {
        let (old_inner, old_outer) = (self.inner_index, self.outer_index);

        if self.inner_index == self.outer_index {
            self.outer_index += 1;
            self.inner_index = 0;
        } else {
            self.inner_index += 1;
        }

        if old_outer < self.slice.len() {
            Some((self.slice[old_inner], self.slice[old_outer]))
        } else {
            None
        }
    }
}

pub fn indent_lines(msg: &str, indentation: usize) -> String {
    #[allow(clippy::format_collect)]
    msg.lines()
        .map(|line| format!("{}{line}\n", " ".repeat(indentation)))
        .collect::<String>()
}
