use crate::{
    exercise_types::Measurement,
    exercises::Exercise,
    workouts::{PerformedPortion, PerformedSet},
};
use serde_json::Number;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy)]
pub(super) enum Field {
    Kind,
    Kilograms,
    LoadDescription,
    Quantity,
    Rpe,
    WhiteFlags,
    RedFlags,
    SetNotes,
    PortionNotes,
    SessionNotes,
}
impl Field {
    pub const ALL: [Self; 10] = [
        Self::Kind,
        Self::Kilograms,
        Self::LoadDescription,
        Self::Quantity,
        Self::Rpe,
        Self::WhiteFlags,
        Self::RedFlags,
        Self::SetNotes,
        Self::PortionNotes,
        Self::SessionNotes,
    ];
}

/// Text stays transient until the shared application boundary accepts the complete set.
pub(super) struct SetForm {
    pub exercise: Exercise,
    values: [String; Field::ALL.len()],
    pub set_id: Option<i64>,
    pub portion_id: Option<i64>,
    pub target: String,
}
impl SetForm {
    pub fn new(exercise: Exercise, session_notes: &Option<String>) -> Self {
        let mut values = std::array::from_fn(|_| String::new());
        values[Field::Kind as usize] = "main".into();
        values[Field::SessionNotes as usize] = session_notes.clone().unwrap_or_default();
        Self {
            exercise,
            values,
            set_id: None,
            portion_id: None,
            target: String::new(),
        }
    }
    pub fn from_set(
        exercise: Exercise,
        set: &PerformedSet,
        session_notes: &Option<String>,
        duplicate: bool,
    ) -> Self {
        let mut form = Self::new(exercise, session_notes);
        *form.input(Field::Kind) = set.kind.clone();
        *form.input(Field::Kilograms) = display_number(&set.kilograms);
        *form.input(Field::LoadDescription) = set.load_description.clone().unwrap_or_default();
        if !duplicate {
            form.set_id = set.id;
            form.portion_id = set.portions[0].id;
            let p = &set.portions[0];
            *form.input(Field::Quantity) = display_number(match form.exercise.measurement {
                Measurement::Repetitions => &p.repetitions,
                Measurement::Duration => &p.seconds,
                Measurement::Distance => &p.metres,
            });
            *form.input(Field::Rpe) = display_number(&set.rpe);
            *form.input(Field::WhiteFlags) =
                set.white_flags.map(|n| n.to_string()).unwrap_or_default();
            *form.input(Field::RedFlags) = set.red_flags.map(|n| n.to_string()).unwrap_or_default();
            *form.input(Field::SetNotes) = set.notes.clone().unwrap_or_default();
            *form.input(Field::PortionNotes) = p.notes.clone().unwrap_or_default();
        }
        form
    }
    pub fn value(&self, field: Field) -> &str {
        &self.values[field as usize]
    }
    pub fn input(&mut self, field: Field) -> &mut String {
        &mut self.values[field as usize]
    }
    pub fn label(&self, field: Field) -> &str {
        match field {
            Field::Kind => "Type (warmup/main)",
            Field::Kilograms => "Kilograms (blank = unknown)",
            Field::LoadDescription => "Load description",
            Field::Quantity => match self.exercise.measurement {
                Measurement::Repetitions => "Attempted repetitions",
                Measurement::Duration => "Seconds",
                Measurement::Distance => "Metres",
            },
            Field::Rpe => "RPE (1-10, half steps)",
            Field::WhiteFlags => "White flags",
            Field::RedFlags => "Red flags (total three)",
            Field::SetNotes => "Set notes",
            Field::PortionNotes => "Portion notes",
            Field::SessionNotes => "Session notes",
        }
    }
    pub fn parse(&self) -> Result<PerformedSet> {
        let quantity = parse_number(self.value(Field::Quantity))?
            .ok_or("Actual quantity is required; blank does not mean zero")?;
        let mut portion = PerformedPortion {
            id: self.portion_id,
            exercise_id: self.exercise.id,
            repetitions: None,
            seconds: None,
            metres: None,
            notes: optional_text(self.value(Field::PortionNotes)),
        };
        match self.exercise.measurement {
            Measurement::Repetitions => portion.repetitions = Some(quantity),
            Measurement::Duration => portion.seconds = Some(quantity),
            Measurement::Distance => portion.metres = Some(quantity),
        };
        let flag = |s: &str| -> Result<Option<u8>> {
            if s.trim().is_empty() {
                Ok(None)
            } else {
                Ok(Some(s.trim().parse()?))
            }
        };
        Ok(PerformedSet {
            id: self.set_id,
            kind: self.value(Field::Kind).trim().into(),
            kilograms: parse_number(self.value(Field::Kilograms))?,
            load_description: optional_text(self.value(Field::LoadDescription)),
            rpe: parse_number(self.value(Field::Rpe))?,
            white_flags: flag(self.value(Field::WhiteFlags))?,
            red_flags: flag(self.value(Field::RedFlags))?,
            notes: optional_text(self.value(Field::SetNotes)),
            portions: vec![portion],
        })
    }
}
fn display_number(n: &Option<Number>) -> String {
    n.as_ref().map(ToString::to_string).unwrap_or_default()
}
fn parse_number(s: &str) -> Result<Option<Number>> {
    if s.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(s.trim().parse()?))
    }
}
pub(super) fn optional_text(s: &str) -> Option<String> {
    if s.is_empty() { None } else { Some(s.into()) }
}
