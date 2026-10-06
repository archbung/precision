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

fn main () {
    assert_eq!(Measurement::Duration.as_str(), "duration");
}
