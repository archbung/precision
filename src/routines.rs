use crate::aggregate_values::{decimal_units, number, optional_number, text, validate_id};
use crate::{
    exercise_types::{LoadConvention, Measurement},
    exercises::Store,
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::Number;
use std::collections::HashSet;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Routine {
    #[serde(default, deserialize_with = "crate::organization::rest_array")]
    pub rest: Option<Vec<crate::organization::Rest>>,
    #[serde(default)]
    pub supersets: Vec<crate::organization::Superset>,
    pub schema_version: u32,
    pub id: Option<i64>,
    pub name: String,
    pub notes: Option<String>,
    pub sets: Vec<PrescribedSet>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PrescribedSet {
    pub id: Option<i64>,
    #[serde(rename = "type", default = "main_type")]
    pub kind: String,
    #[serde(default, deserialize_with = "optional_number")]
    pub kilograms: Option<Number>,
    #[serde(default, deserialize_with = "optional_number")]
    pub rpe: Option<Number>,
    pub load_description: Option<String>,
    pub notes: Option<String>,
    pub portions: Vec<PrescribedPortion>,
}
fn main_type() -> String {
    "main".into()
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PrescribedPortion {
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
    pub fn routine(&self, id: i64) -> Result<Routine> {
        read(&self.connection, id)
    }
    pub fn routines(&self) -> Result<Vec<Routine>> {
        let mut statement = self
            .connection
            .prepare("SELECT id FROM routines ORDER BY id")?;
        let ids = statement
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<i64>>>()?;
        ids.into_iter().map(|id| self.routine(id)).collect()
    }
    pub fn save_routine(&mut self, id: Option<i64>, mut routine: Routine) -> Result<Routine> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let original = id.map(|id| read(&tx, id)).transpose()?;
        validate(&tx, &mut routine, original.as_ref())?;
        persist(&tx, id, &mut routine, None)?;
        tx.commit()?;
        Ok(routine)
    }
    pub fn delete_routine(&mut self, id: i64) -> Result<()> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx.execute("DELETE FROM routines WHERE id=?1", [id])? == 0 {
            return Err(format!("unknown routine ID {id}; use routine list").into());
        }
        tx.commit()?;
        Ok(())
    }
}
pub(crate) fn read(connection: &Connection, id: i64) -> Result<Routine> {
    let mut routine = connection
        .query_row("SELECT name,notes FROM routines WHERE id=?1", [id], |row| {
            Ok(Routine {
                schema_version: 1,
                id: Some(id),
                name: row.get(0)?,
                notes: row.get(1)?,
                rest: None,
                supersets: vec![],
                sets: vec![],
            })
        })
        .optional()?
        .ok_or_else(|| format!("unknown routine ID {id}; use routine list"))?;
    routine.sets = read_sets(connection, id, false)?;
    (routine.rest, routine.supersets) = crate::organization::read(connection, "prescribed", id)?;
    Ok(routine)
}
pub(crate) fn validate(
    connection: &Connection,
    routine: &mut Routine,
    original: Option<&Routine>,
) -> Result<()> {
    if routine.schema_version != 1 {
        return Err("unsupported routine schema_version; expected 1".into());
    }
    if routine.id.is_some() && routine.id != original.and_then(|original| original.id) {
        return Err("routine ID must match the update destination; omit IDs on create".into());
    }
    routine.name = routine.name.trim().to_owned();
    if routine.name.is_empty() {
        return Err("routine name must not be empty".into());
    }
    if routine.sets.is_empty() {
        return Err("routine requires at least one prescribed set".into());
    }
    crate::organization::validate(
        &mut routine.rest,
        &routine.supersets,
        &routine.sets.iter().map(|s| s.id).collect::<Vec<_>>(),
        &original
            .map(|r| r.sets.iter().map(|s| s.id).collect::<Vec<_>>())
            .unwrap_or_default(),
        original.map(|r| r.supersets.as_slice()).unwrap_or(&[]),
    )?;
    validate_sets(
        connection,
        &routine.sets,
        original.map(|r| r.sets.as_slice()).unwrap_or(&[]),
    )
}
pub(crate) fn validate_sets(
    connection: &Connection,
    sets_to_validate: &[PrescribedSet],
    original: &[PrescribedSet],
) -> Result<()> {
    let old_sets: HashSet<i64> = original.iter().filter_map(|s| s.id).collect();
    let old_portions: HashSet<i64> = original
        .iter()
        .flat_map(|s| &s.portions)
        .filter_map(|p| p.id)
        .collect();
    let mut sets = HashSet::new();
    let mut portions = HashSet::new();
    for set in sets_to_validate {
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
        for portion in &set.portions {
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

fn sql(query: &str, intention: bool) -> String {
    if intention {
        query
            .replace("prescribed_sets", "intention_sets")
            .replace("prescribed_portions", "intention_portions")
            .replace("routine_id", "workout_id")
    } else {
        query.to_owned()
    }
}
pub(crate) fn write_sets(
    connection: &Connection,
    owner_id: i64,
    sets: &mut [PrescribedSet],
    intention: bool,
) -> Result<()> {
    for (position, set) in sets.iter_mut().enumerate() {
        let rpe_half = set
            .rpe
            .as_ref()
            .map(decimal_units)
            .transpose()?
            .map(|units| units / 500_000);
        connection.execute(&sql("INSERT INTO prescribed_sets(id,routine_id,position,type,kilograms,rpe_half,load_description,notes) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)", intention),params![set.id,owner_id,position as i64,set.kind,text(&set.kilograms),rpe_half,set.load_description,set.notes])?;
        set.id = Some(set.id.unwrap_or_else(|| connection.last_insert_rowid()));
        for (position, portion) in set.portions.iter_mut().enumerate() {
            connection.execute(&sql("INSERT INTO prescribed_portions(id,set_id,position,exercise_id,repetitions,seconds,metres,notes) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)", intention),params![portion.id,set.id,position as i64,portion.exercise_id,text(&portion.repetitions),text(&portion.seconds),text(&portion.metres),portion.notes])?;
            portion.id = Some(portion.id.unwrap_or_else(|| connection.last_insert_rowid()));
        }
    }
    Ok(())
}
pub(crate) fn read_sets(
    connection: &Connection,
    id: i64,
    intention: bool,
) -> Result<Vec<PrescribedSet>> {
    let mut statement = connection.prepare(&sql("SELECT id,type,kilograms,rpe_half,load_description,notes FROM prescribed_sets WHERE routine_id=?1 ORDER BY position", intention))?;
    let mut sets: Vec<PrescribedSet> = statement
        .query_map([id], |row| {
            let half: Option<i64> = row.get(3)?;
            Ok(PrescribedSet {
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
    for set in &mut sets {
        let mut statement = connection.prepare(&sql("SELECT id,exercise_id,repetitions,seconds,metres,notes FROM prescribed_portions WHERE set_id=?1 ORDER BY position", intention))?;
        set.portions = statement
            .query_map([set.id], |row| {
                Ok(PrescribedPortion {
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
    Ok(sets)
}

pub(crate) fn persist(
    tx: &Connection,
    id: Option<i64>,
    routine: &mut Routine,
    local_ids: Option<&[Option<i64>]>,
) -> Result<()> {
    if let Some(id) = id {
        tx.execute(
            "UPDATE routines SET name=?1,notes=?2,revision=revision+1 WHERE id=?3",
            params![routine.name, routine.notes, id],
        )?;
        tx.execute("DELETE FROM prescribed_sets WHERE routine_id=?1", [id])?;
    } else {
        tx.execute(
            "INSERT INTO routines(name,notes) VALUES (?1,?2)",
            params![routine.name, routine.notes],
        )?;
    }
    routine.id = Some(id.unwrap_or_else(|| tx.last_insert_rowid()));
    write_sets(tx, routine.id.unwrap(), &mut routine.sets, false)?;
    if let Some(local_ids) = local_ids {
        for group in &mut routine.supersets {
            for member in &mut group.set_ids {
                let position = local_ids
                    .iter()
                    .position(|s| *s == Some(*member))
                    .ok_or("foreign superset member")?;
                *member = routine.sets[position].id.unwrap();
            }
        }
    }
    crate::organization::write(
        tx,
        "prescribed",
        routine.id.unwrap(),
        routine.rest.as_deref().unwrap(),
        &mut routine.supersets,
    )?;
    Ok(())
}
