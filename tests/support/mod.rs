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

/// Legacy domain fixtures take a fresh snapshot at each independent mutation.
/// Revision-contract tests deliberately bypass this helper.
#[allow(dead_code)]
pub fn with_workout_revision(path: &Path, args: &[&str]) -> Vec<String> {
    let mut result: Vec<String> = args.iter().map(|s| (*s).into()).collect();
    let actual = args.starts_with(&["workout", "update"]);
    let intention = args.starts_with(&["workout", "intention", "update"]);
    let lifecycle =
        args.starts_with(&["workout", "finish"]) || args.starts_with(&["workout", "discard"]);
    if !actual && !intention && !lifecycle {
        return result;
    }
    let id_index = if intention { 3 } else { 2 };
    let id = args[id_index].parse().unwrap();
    let workout = precision::exercises::Store::open(path)
        .unwrap()
        .workout(id)
        .unwrap();
    let revision = workout.revision.unwrap();
    if actual {
        let file = args[args.iter().position(|a| *a == "--file").unwrap() + 1];
        let mut document: serde_json::Value =
            serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
        if document.get("revision").is_none() {
            document["revision"] = serde_json::json!(revision);
            std::fs::write(file, serde_json::to_vec(&document).unwrap()).unwrap();
        }
    } else if !args.contains(&"--revision") {
        result.extend(["--revision".into(), revision.to_string()]);
    }
    result
}
