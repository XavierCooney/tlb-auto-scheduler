use std::fmt::{self};
use std::{fs, path::Path};

use anyhow::{anyhow, Context, Result};
use enum_map::EnumMap;
use serde::de::Error as _;
use serde::Deserialize;
use strum::IntoStaticStr;

pub type CostValue = u64;

#[derive(Debug, Deserialize, Default)]
enum CostPossibility {
    #[serde(alias = "inf", alias = "infinity")]
    #[default]
    Infinity,
    #[serde(untagged)]
    Value(CostValue),
}

#[derive(Debug, enum_map::Enum, Deserialize, IntoStaticStr, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Constraint {
    AssignedPreferred,
    AssignedPossible,
    AssignedDislike,
    AssignedImpossible,
    UnassignedSession,
    BelowMinTut,
    BelowMinLab,
    BelowMinClass,
    AboveMaxTut,
    AboveMaxLab,
    AboveMaxClass,
    DirectOverlap,
    PaddedOverlap,
    SameDayOverlap,
    MismatchedInitialSolution,
}

impl Constraint {
    fn default_value(self) -> Option<CostPossibility> {
        Some(match self {
            Self::AssignedPreferred => CostPossibility::Value(0),
            Self::AssignedImpossible => CostPossibility::Infinity,
            Self::MismatchedInitialSolution => CostPossibility::Value(0),
            _ => return None,
        })
    }
}

type CostCountNum = u32;

pub struct CostCount {
    counts: EnumMap<Constraint, CostCountNum>,
}

impl CostCount {
    pub fn add_cost(&mut self, category: Constraint, count: impl Into<CostCountNum>) {
        self.counts[category] += count.into();
    }

    pub fn add_cost_1(&mut self, category: Constraint) {
        self.add_cost(category, 1 as CostCountNum);
    }

    pub fn total_cost(&self, config: &CostConfig) -> Option<CostValue> {
        self.counts
            .iter()
            .map(|(constraint, &count)| match config.map[constraint] {
                CostPossibility::Value(val) => (count as CostValue).checked_mul(val),
                CostPossibility::Infinity => {
                    if count > 0 {
                        None
                    } else {
                        Some(0)
                    }
                }
            })
            .sum::<Option<CostValue>>()
    }

    pub fn new() -> Self {
        CostCount {
            counts: EnumMap::default(),
        }
    }
}

impl fmt::Display for CostCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (constraint, count) in self.counts {
            let constraint_name: &str = constraint.into();
            writeln!(f, "{constraint_name}: {count}")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct CostConfig {
    map: EnumMap<Constraint, CostPossibility>,
}

impl CostConfig {
    pub fn read_from_toml(path: &Path) -> Result<Self> {
        let toml_string = fs::read_to_string(path)
            .with_context(|| anyhow!("failed to read costs toml at {}", path.display()))?;
        toml::from_str(&toml_string)
            .with_context(|| anyhow!("failed to parse cost config at {}", path.display()))
    }

    pub fn should_count(&self, constraint: Constraint) -> bool {
        match self.map[constraint] {
            CostPossibility::Infinity => true,
            CostPossibility::Value(val) => val != 0,
        }
    }
}

// Although EnumMap implements Deserialize it doesn't quite suit what we need
// here so do a custom implementation instead
impl<'de> Deserialize<'de> for CostConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(CostConfigVisitor)
    }
}

struct CostConfigVisitor;

impl<'de> serde::de::Visitor<'de> for CostConfigVisitor {
    type Value = CostConfig;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a map")
    }

    fn visit_map<M: serde::de::MapAccess<'de>>(
        self,
        mut access: M,
    ) -> Result<Self::Value, M::Error> {
        let mut entries: EnumMap<Constraint, Option<_>> = EnumMap::default();

        while let Some((constraint, value)) = access.next_entry()? {
            if entries[constraint].is_some() {
                return Err(M::Error::duplicate_field(constraint.into()));
            }
            entries[constraint] = Some(value);
        }

        Ok(CostConfig {
            map: entries
                .into_iter()
                .map(
                    |(constraint, val)| match val.or_else(|| constraint.default_value()) {
                        Some(val) => Ok((constraint, val)),
                        None => Err(M::Error::missing_field(constraint.into())),
                    },
                )
                .collect::<Result<_, _>>()?,
        })
    }
}
