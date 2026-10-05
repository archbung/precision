use crate::aggregate_values::{decimal_units, number, optional_number, text, validate_id};
use crate::exercise_types::{LoadConvention, Measurement};
use crate::exercises::Store;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::Number;
use std::collections::HashSet;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Workout {
    pub schema_version: u32,
    pub id: Option<i64>,
    #[serde(default = "draft_state")]
    pub state: String,
    pub date: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub notes: Option<String>,
    pub sets: Vec<PerformedSet>,
}
fn draft_state() -> String {
    "draft".into()
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PerformedSet {
    pub white_flags: Option<u8>,
    pub red_flags: Option<u8>,
    pub id: Option<i64>,
    #[serde(rename = "type", default = "main_type")]
    pub kind: String,
    #[serde(default, deserialize_with = "optional_number")]
    pub kilograms: Option<Number>,
    #[serde(default, deserialize_with = "optional_number")]
    pub rpe: Option<Number>,
    pub load_description: Option<String>,
    pub notes: Option<String>,
    pub portions: Vec<PerformedPortion>,
}
fn main_type() -> String {
    "main".into()
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PerformedPortion {
    pub id: Option<i64>,
    pub exercise_id: i64,
    #[serde(default, deserialize_with = "optional_number")]
    pub repetitions: Option<Number>,
    #[serde(default, deserialize_with = "optional_number")]
    pub seconds: Option<Number>,
    #[serde(default, deserialize_with = "optional_number")]
    pub metres: Option<Number>,
    pub notes: Option<String>,
}
impl Store {
    pub fn workout(&self, id: i64) -> Result<Workout> {
        read(&self.connection, id)
    }
    pub fn workouts(&self, drafts: bool) -> Result<Vec<Workout>> {
        let mut statement = self
            .connection
            .prepare("SELECT id FROM workouts WHERE state='finished' OR ?1 ORDER BY id")?;
        let ids = statement
            .query_map([drafts], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<i64>>>()?;
        ids.into_iter().map(|id| self.workout(id)).collect()
    }
    pub fn start_workout(&mut self, date: String, start: Option<String>) -> Result<Workout> {
        validate_times(&date, &start, &None)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT INTO workouts(state,date,start) VALUES ('draft',?1,?2)",
            params![date, start],
        )?;
        let workout = read(&tx, tx.last_insert_rowid())?;
        tx.commit()?;
        Ok(workout)
    }
    pub fn save_workout(&mut self, id: i64, mut workout: Workout) -> Result<Workout> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let original = read(&tx, id)?;
        require_draft(&original)?;
        validate(&tx, &mut workout, Some(&original))?;
        workout.id = Some(id);
        tx.execute(
            "UPDATE workouts SET date=?1,start=?2,end=?3,notes=?4 WHERE id=?5",
            params![workout.date, workout.start, workout.end, workout.notes, id],
        )?;
        tx.execute("DELETE FROM performed_sets WHERE workout_id=?1", [id])?;
        for (position, set) in workout.sets.iter_mut().enumerate() {
            let rpe_half = set
                .rpe
                .as_ref()
                .map(decimal_units)
                .transpose()?
                .map(|units| units / 500_000);
            tx.execute("INSERT INTO performed_sets(id,workout_id,position,type,kilograms,rpe_half,load_description,notes,white_flags,red_flags) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![set.id,workout.id,position as i64,set.kind,text(&set.kilograms),rpe_half,set.load_description,set.notes,set.white_flags,set.red_flags])?;
            set.id = Some(set.id.unwrap_or_else(|| tx.last_insert_rowid()));
            for (position, portion) in set.portions.iter_mut().enumerate() {
                tx.execute("INSERT INTO performed_portions(id,set_id,position,exercise_id,repetitions,seconds,metres,notes) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",params![portion.id,set.id,position as i64,portion.exercise_id,text(&portion.repetitions),text(&portion.seconds),text(&portion.metres),portion.notes])?;
                portion.id = Some(portion.id.unwrap_or_else(|| tx.last_insert_rowid()));
            }
        }
        tx.commit()?;
        Ok(workout)
    }

    pub fn finish_workout(&mut self, id: i64, end: Option<String>) -> Result<Workout> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut workout = read(&tx, id)?;
        require_draft(&workout)?;
        if workout.sets.is_empty() {
            return Err("finish requires at least one performed set".into());
        }
        if end.is_some() {
            workout.end = end;
        }
        validate_times(&workout.date, &workout.start, &workout.end)?;
        tx.execute(
            "UPDATE workouts SET state='finished',end=?1 WHERE id=?2",
            params![workout.end, id],
        )?;
        workout.state = "finished".into();
        tx.commit()?;
        Ok(workout)
    }
    pub fn discard_workout(&mut self, id: i64) -> Result<()> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_draft(&read(&tx, id)?)?;
        tx.execute("DELETE FROM workouts WHERE id=?1", [id])?;
        tx.commit()?;
        Ok(())
    }
}
fn require_draft(workout: &Workout) -> Result<()> {
    if workout.state != "draft" {
        return Err("completed workouts are read-only".into());
    }
    Ok(())
}
fn validate_times(date: &str, start: &Option<String>, end: &Option<String>) -> Result<()> {
    let day = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| "date must be YYYY-MM-DD")?;
    if day.format("%Y-%m-%d").to_string() != date {
        return Err("date must be YYYY-MM-DD".into());
    }
    let parse = |value: &String| {
        chrono::DateTime::parse_from_rfc3339(value)
            .map_err(|_| "timestamp must be RFC3339 with full date and timezone offset")
    };
    let start = start.as_ref().map(parse).transpose()?;
    let end = end.as_ref().map(parse).transpose()?;
    if start.is_some_and(|time| time.date_naive() != day) {
        return Err("start timestamp local date must match workout date".into());
    }
    if let Some(end) = end
        && start.map_or(end.date_naive() < day, |start| end < start)
    {
        return Err("end timestamp cannot precede start".into());
    }
    Ok(())
}
fn read(connection: &Connection, id: i64) -> Result<Workout> {
    let mut workout = connection
        .query_row(
            "SELECT state,date,start,end,notes FROM workouts WHERE id=?1",
            [id],
            |row| {
                Ok(Workout {
                    schema_version: 1,
                    id: Some(id),
                    state: row.get(0)?,
                    date: row.get(1)?,
                    start: row.get(2)?,
                    end: row.get(3)?,
                    notes: row.get(4)?,
                    sets: vec![],
                })
            },
        )
        .optional()?
        .ok_or_else(|| format!("unknown workout ID {id}; use workout list --drafts"))?;
    let mut statement = connection.prepare("SELECT id,type,kilograms,rpe_half,load_description,notes,white_flags,red_flags FROM performed_sets WHERE workout_id=?1 ORDER BY position")?;
    workout.sets = statement
        .query_map([id], |row| {
            let half: Option<i64> = row.get(3)?;
            Ok(PerformedSet {
                white_flags: row.get(6)?,
                red_flags: row.get(7)?,
                id: row.get(0)?,
                kind: row.get(1)?,
                kilograms: number(row.get(2)?)?,
                rpe: number(
                    half.map(|half| format!("{}.{}", half / 2, if half % 2 == 0 { 0 } else { 5 })),
                )?,
                load_description: row.get(4)?,
                notes: row.get(5)?,
                portions: vec![],
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    for set in &mut workout.sets {
        let mut statement = connection.prepare("SELECT id,exercise_id,repetitions,seconds,metres,notes FROM performed_portions WHERE set_id=?1 ORDER BY position")?;
        set.portions = statement
            .query_map([set.id], |row| {
                Ok(PerformedPortion {
                    id: row.get(0)?,
                    exercise_id: row.get(1)?,
                    repetitions: number(row.get(2)?)?,
                    seconds: number(row.get(3)?)?,
                    metres: number(row.get(4)?)?,
                    notes: row.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
    }
    Ok(workout)
}
fn validate(
    connection: &Connection,
    workout: &mut Workout,
    original: Option<&Workout>,
) -> Result<()> {
    if workout.schema_version != 1 {
        return Err("unsupported workout schema_version; expected 1".into());
    }
    if workout.id.is_some() && workout.id != original.and_then(|original| original.id) {
        return Err("workout ID must match the update destination; omit IDs on create".into());
    }
    require_draft(workout)?;
    validate_times(&workout.date, &workout.start, &workout.end)?;
    let old_sets: HashSet<i64> = original
        .into_iter()
        .flat_map(|r| &r.sets)
        .filter_map(|s| s.id)
        .collect();
    let old_portions: HashSet<i64> = original
        .into_iter()
        .flat_map(|r| &r.sets)
        .flat_map(|s| &s.portions)
        .filter_map(|p| p.id)
        .collect();
    let mut sets = HashSet::new();
    let mut portions = HashSet::new();
    for set in &workout.sets {
        validate_id(set.id, &old_sets, &mut sets, "set")?;
        if !matches!(set.kind.as_str(), "warmup" | "main") {
            return Err("set type must be warmup or main".into());
        }
        if set.portions.is_empty() {
            return Err("set requires at least one portion".into());
        }
        let kilograms = set.kilograms.as_ref().map(decimal_units).transpose()?;
        if let Some(rpe) = &set.rpe {
            let units = decimal_units(rpe)?;
            if !(1_000_000..=10_000_000).contains(&units) || units % 500_000 != 0 {
                return Err("RPE must be 1–10 in half-point steps".into());
            }
        }
        match (set.white_flags, set.red_flags) {
            (None, None) => (),
            (Some(white), Some(red)) if u16::from(white) + u16::from(red) == 3 => (),
            _ => return Err("white/red judging flags must both be absent or total three".into()),
        }
        for portion in &set.portions {
            if [&portion.repetitions, &portion.seconds, &portion.metres]
                .into_iter()
                .filter(|v| v.is_some())
                .count()
                != 1
            {
                return Err("performed portion requires exactly one primary measurement".into());
            }
            validate_id(portion.id, &old_portions, &mut portions, "portion")?;
            let definition: Option<(String, String)> = connection
                .query_row(
                    "SELECT measurement,load_convention FROM exercises WHERE id=?1",
                    [portion.exercise_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            let (mode, load) =
                definition.ok_or_else(|| format!("unknown exercise ID {}", portion.exercise_id))?;
            let mode: Measurement = mode.parse()?;
            let load: LoadConvention = load.parse()?;
            if load == LoadConvention::External && kilograms.is_some_and(|kg| kg < 0) {
                return Err(
                    "negative kilograms require all portions to use added-bodyweight load".into(),
                );
            }
            for (field, expected, value) in [
                (
                    "repetitions",
                    Measurement::Repetitions,
                    &portion.repetitions,
                ),
                ("seconds", Measurement::Duration, &portion.seconds),
                ("metres", Measurement::Distance, &portion.metres),
            ] {
                if let Some(value) = value {
                    if mode != expected {
                        return Err(format!(
                            "{field} is incompatible with exercise measurement {mode}"
                        )
                        .into());
                    }
                    let units = decimal_units(value)?;
                    if units < 0 {
                        return Err(format!("{field} must be nonnegative").into());
                    }
                    if field == "repetitions" && units % 1_000_000 != 0 {
                        return Err("repetitions must be whole numbers".into());
                    }
                }
            }
        }
    }
    Ok(())
}
