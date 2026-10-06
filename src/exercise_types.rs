use clap::ValueEnum;
use serde::Serialize;
use std::{fmt, str::FromStr};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Measurement {
    Repetitions,
    Duration,
    Distance,
}
impl Measurement {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Repetitions => "repetitions",
            Self::Duration => "duration",
            Self::Distance => "distance",
        }
    }
}
impl FromStr for Measurement {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "repetitions" => Ok(Self::Repetitions),
            "duration" => Ok(Self::Duration),
            "distance" => Ok(Self::Distance),
            _ => Err(format!(
                "unknown measurement {value:?}; expected repetitions, duration, or distance"
            )),
        }
    }
}
impl fmt::Display for Measurement {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LoadConvention {
    External,
    AddedBodyweight,
}
impl LoadConvention {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::External => "external",
            Self::AddedBodyweight => "added-bodyweight",
        }
    }
}
impl FromStr for LoadConvention {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "external" => Ok(Self::External),
            "added-bodyweight" => Ok(Self::AddedBodyweight),
            _ => Err(format!(
                "unknown load convention {value:?}; expected external or added-bodyweight"
            )),
        }
    }
}
impl fmt::Display for LoadConvention {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl ValueEnum for Measurement {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Repetitions, Self::Duration, Self::Distance]
    }
    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.as_str()))
    }
}
impl ValueEnum for LoadConvention {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::External, Self::AddedBodyweight]
    }
    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.as_str()))
    }
}
