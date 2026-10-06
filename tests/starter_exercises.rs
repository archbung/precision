use std::{path::Path, process::Command};
use tempfile::TempDir;

fn list(path: &Path) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_precision"))
        .arg("--db")
        .arg(path)
        .args(["exercise", "list", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn fresh_database_seeds_exercises_and_reopening_preserves_them() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("db.sqlite3");
    let exercises = list(&path);
    assert_eq!(exercises.as_array().unwrap().len(), 60);
    assert_eq!(exercises[0]["name"], "Barbell clean");
    assert_eq!(exercises[0]["equipment"], serde_json::json!([1]));
    assert_eq!(exercises[0]["primary_muscle"], 12);
    assert_eq!(exercises[53]["primary_muscle"], 13);
    assert_eq!(exercises[57]["load_convention"], "added-bodyweight");
    assert_eq!(
        exercises[57]["secondary_muscles"],
        serde_json::json!([4, 6])
    );
    assert_eq!(list(&path), exercises);
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        8
    );
    assert!(
        !connection
            .prepare("PRAGMA foreign_key_check")
            .unwrap()
            .exists([])
            .unwrap()
    );
}

#[test]
fn upgrading_preserves_existing_catalog_even_when_empty() {
    for populated in [false, true] {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("db.sqlite3");
        list(&path);
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection.execute_batch("DELETE FROM exercise_equipment; DELETE FROM exercise_secondary_muscles; DELETE FROM exercises; PRAGMA user_version=7;").unwrap();
        if populated {
            connection.execute_batch("INSERT INTO exercises(id,name,name_key,measurement,load_convention) VALUES(1,'My exercise','my exercise','distance','external');").unwrap();
        }
        drop(connection);
        let exercises = list(&path);
        assert_eq!(exercises.as_array().unwrap().len(), usize::from(populated));
        if populated {
            assert_eq!(exercises[0]["id"], 1);
            assert_eq!(exercises[0]["name"], "My exercise");
        }
    }
}
