use rusqlite::{Connection, Transaction, TransactionBehavior, params};
use serde::Serialize;
use std::{path::Path, time::Duration};
use unicode_casefold::UnicodeCaseFold;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Serialize)]
pub struct Exercise {
    pub schema_version: u32,
    pub id: i64,
    pub name: String,
    pub measurement: String,
    pub load_convention: String,
    pub equipment: Vec<i64>,
    pub primary_muscle: Option<i64>,
    pub secondary_muscles: Vec<i64>,
}
pub struct Store {
    connection: Connection,
}
impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let version: u32 = tx.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > 1 {
            return Err(format!(
                "database schema version {version} is newer than supported version 1"
            )
            .into());
        }
        if version == 0 {
            tx.execute_batch(include_str!("../migrations/0001.sql"))?;
            for (table, names) in [
                (
                    "equipment",
                    &[
                        "Barbell",
                        "Dumbbell",
                        "Kettlebell",
                        "Band",
                        "Cable",
                        "Machine",
                        "Sled",
                        "Weight Plate",
                    ][..],
                ),
                (
                    "muscles",
                    &[
                        "Chest",
                        "Back",
                        "Shoulders",
                        "Biceps",
                        "Triceps",
                        "Forearms",
                        "Abdominals",
                        "Glutes",
                        "Quadriceps",
                        "Hamstrings",
                        "Calves",
                        "Fullbody",
                    ][..],
                ),
            ] {
                for (index, name) in names.iter().enumerate() {
                    tx.execute(
                        &format!("INSERT INTO {table}(id,name) VALUES (?1,?2)"),
                        params![index as i64 + 1, name],
                    )?;
                }
            }
            tx.pragma_update(None, "user_version", 1)?;
        }
        tx.commit()?;
        Ok(Self { connection })
    }
    pub fn catalog(&self, table: &str) -> Result<Vec<(i64, String)>> {
        let mut statement = self
            .connection
            .prepare(&format!("SELECT id,name FROM {table} ORDER BY id"))?;
        Ok(statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?)
    }
    pub fn show(&self, id: i64) -> Result<Exercise> {
        read(&self.connection, id)
    }
    pub fn list(&self) -> Result<Vec<Exercise>> {
        let mut statement = self
            .connection
            .prepare("SELECT id FROM exercises ORDER BY id")?;
        let ids = statement
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<i64>>>()?;
        ids.into_iter().map(|id| self.show(id)).collect()
    }
    pub fn create(&mut self, mut exercise: Exercise) -> Result<Exercise> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        validate(&tx, &mut exercise)?;
        tx.execute("INSERT INTO exercises(name,name_key,measurement,load_convention,primary_muscle) VALUES (?1,?2,?3,?4,?5)", params![exercise.name, exercise.name.case_fold().collect::<String>(), exercise.measurement, exercise.load_convention, exercise.primary_muscle])?;
        exercise.id = tx.last_insert_rowid();
        for id in &exercise.equipment {
            tx.execute(
                "INSERT INTO exercise_equipment VALUES (?1,?2)",
                params![exercise.id, id],
            )?;
        }
        save_secondary(&tx, &exercise)?;
        tx.commit()?;
        Ok(exercise)
    }
    pub fn update(
        &mut self,
        id: i64,
        name: Option<String>,
        primary: Option<i64>,
        secondary: Option<Vec<i64>>,
        clear_primary: bool,
    ) -> Result<Exercise> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut exercise = read(&tx, id)?;
        if let Some(name) = name {
            exercise.name = name;
        }
        if clear_primary {
            exercise.primary_muscle = None;
        } else if primary.is_some() {
            exercise.primary_muscle = primary;
        }
        if let Some(secondary) = secondary {
            exercise.secondary_muscles = secondary;
        }
        validate(&tx, &mut exercise)?;
        tx.execute(
            "UPDATE exercises SET name=?1,name_key=?2,primary_muscle=?3 WHERE id=?4",
            params![
                exercise.name,
                exercise.name.case_fold().collect::<String>(),
                exercise.primary_muscle,
                id
            ],
        )?;
        tx.execute(
            "DELETE FROM exercise_secondary_muscles WHERE exercise_id=?1",
            [id],
        )?;
        save_secondary(&tx, &exercise)?;
        tx.commit()?;
        Ok(exercise)
    }
}
fn read(connection: &Connection, id: i64) -> Result<Exercise> {
    let result = connection.query_row(
        "SELECT id,name,measurement,load_convention,primary_muscle FROM exercises WHERE id=?1",
        [id],
        |row| {
            Ok(Exercise {
                schema_version: 1,
                id: row.get(0)?,
                name: row.get(1)?,
                measurement: row.get(2)?,
                load_convention: row.get(3)?,
                primary_muscle: row.get(4)?,
                equipment: vec![],
                secondary_muscles: vec![],
            })
        },
    );
    let mut exercise = match result {
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            return Err(format!("unknown exercise ID {id}; use exercise list").into());
        }
        other => other?,
    };
    for (table, column, destination) in [
        (
            "exercise_equipment",
            "equipment_id",
            &mut exercise.equipment,
        ),
        (
            "exercise_secondary_muscles",
            "muscle_id",
            &mut exercise.secondary_muscles,
        ),
    ] {
        let mut statement = connection.prepare(&format!(
            "SELECT {column} FROM {table} WHERE exercise_id=?1 ORDER BY {column}"
        ))?;
        *destination = statement
            .query_map([id], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
    }
    Ok(exercise)
}
fn validate(tx: &Transaction<'_>, exercise: &mut Exercise) -> Result<()> {
    exercise.name = exercise.name.trim().to_owned();
    if exercise.name.is_empty() {
        return Err("exercise name must not be empty".into());
    }
    let duplicate: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM exercises WHERE name_key=?1 AND id<>?2)",
        params![exercise.name.case_fold().collect::<String>(), exercise.id],
        |row| row.get(0),
    )?;
    if duplicate {
        return Err(
            "exercise name already exists (Unicode case-insensitive); choose a different name"
                .into(),
        );
    }
    exercise.equipment.sort_unstable();
    exercise.equipment.dedup();
    exercise.secondary_muscles.sort_unstable();
    exercise.secondary_muscles.dedup();
    if exercise
        .primary_muscle
        .is_some_and(|id| exercise.secondary_muscles.contains(&id))
    {
        return Err("primary muscle cannot also be a secondary muscle".into());
    }
    for (table, ids) in [
        ("equipment", exercise.equipment.clone()),
        (
            "muscles",
            exercise
                .primary_muscle
                .into_iter()
                .chain(exercise.secondary_muscles.iter().copied())
                .collect(),
        ),
    ] {
        for id in ids {
            let exists: bool = tx.query_row(
                &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?1)"),
                [id],
                |row| row.get(0),
            )?;
            if !exists {
                return Err(format!("unknown {table} ID {id}; use catalog {table}").into());
            }
        }
    }
    Ok(())
}
fn save_secondary(tx: &Transaction<'_>, exercise: &Exercise) -> Result<()> {
    for id in &exercise.secondary_muscles {
        tx.execute(
            "INSERT INTO exercise_secondary_muscles VALUES (?1,?2)",
            params![exercise.id, id],
        )?;
    }
    Ok(())
}
