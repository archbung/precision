use std::{path::Path, process::Command};

// Exercise-focused tests use an empty catalog, as in databases created before v8.
pub fn empty_database(path: &Path) {
    if path.exists() {
        return;
    }
    let output = Command::new(env!("CARGO_BIN_EXE_precision"))
        .arg("--db")
        .arg(path)
        .args(["catalog", "equipment"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let connection = rusqlite::Connection::open(path).unwrap();
    connection
        .execute_batch(
            "DELETE FROM exercise_equipment;
         DELETE FROM exercise_secondary_muscles;
         DELETE FROM exercises;
         DELETE FROM sqlite_sequence WHERE name='exercises';",
        )
        .unwrap();
}
