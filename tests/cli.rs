use std::process::{Command, Output};
use tempfile::TempDir;

fn run(db: &TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_precision"))
        .arg("--db")
        .arg(db.path().join("precision.sqlite3"))
        .args(args)
        .output()
        .unwrap()
}
fn success(db: &TempDir, args: &[&str]) -> String {
    let output = run(db, args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}
#[test]
fn exercise_survives_restart_with_stable_identity() {
    let db = TempDir::new().unwrap();
    assert_eq!(success(&db, &["exercise", "list", "--json"]).trim(), "[]");
    success(
        &db,
        &[
            "exercise",
            "create",
            "--name",
            "  Squat  ",
            "--measurement",
            "repetitions",
            "--load-convention",
            "external",
            "--equipment",
            "1",
            "--equipment",
            "4",
            "--primary-muscle",
            "9",
            "--secondary-muscle",
            "8",
        ],
    );
    let value: serde_json::Value =
        serde_json::from_str(&success(&db, &["exercise", "show", "1", "--json"])).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["name"], "Squat");
    assert_eq!(value["equipment"], serde_json::json!([1, 4]));
    assert_eq!(value["primary_muscle"], 9);
    assert_eq!(value["secondary_muscles"], serde_json::json!([8]));
    assert!(success(&db, &["exercise", "list"]).contains("Squat"));
}

fn failure(db: &TempDir, args: &[&str], message: &str) {
    let output = run(db, args);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(message),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn create(db: &TempDir, name: &str, measurement: &str, load: &str) {
    success(
        db,
        &[
            "exercise",
            "create",
            "--name",
            name,
            "--measurement",
            measurement,
            "--load-convention",
            load,
        ],
    );
}
fn show(db: &TempDir, id: &str) -> serde_json::Value {
    serde_json::from_str(&success(db, &["exercise", "show", id, "--json"])).unwrap()
}
#[test]
fn catalogs_are_exact_and_never_seed_exercises() {
    let db = TempDir::new().unwrap();
    assert_eq!(
        success(&db, &["catalog", "equipment"]),
        "1: Barbell\n2: Dumbbell\n3: Kettlebell\n4: Band\n5: Cable\n6: Machine\n7: Sled\n8: Weight Plate\n"
    );
    assert_eq!(
        success(&db, &["catalog", "muscles"]),
        "1: Chest\n2: Back\n3: Shoulders\n4: Biceps\n5: Triceps\n6: Forearms\n7: Abdominals\n8: Glutes\n9: Quadriceps\n10: Hamstrings\n11: Calves\n12: Fullbody\n"
    );
    assert_eq!(success(&db, &["exercise", "list", "--json"]).trim(), "[]");
    failure(&db, &["catalog", "create"], "invalid value");
}
#[test]
fn unicode_names_are_unique_and_failed_updates_are_atomic() {
    let db = TempDir::new().unwrap();
    create(&db, "Straße", "repetitions", "added-bodyweight");
    failure(
        &db,
        &[
            "exercise",
            "create",
            "--name",
            " STRASSE ",
            "--measurement",
            "repetitions",
            "--load-convention",
            "external",
        ],
        "already exists",
    );
    create(&db, "Σ", "duration", "external");
    create(&db, "Carry", "distance", "external");
    let before = show(&db, "3");
    failure(
        &db,
        &[
            "exercise",
            "update",
            "3",
            "--name",
            "ς",
            "--primary-muscle",
            "2",
        ],
        "already exists",
    );
    assert_eq!(show(&db, "3"), before);
    success(&db, &["exercise", "update", "2", "--name", "ς"]);
    assert_eq!(show(&db, "2")["name"], "ς");
}
#[test]
fn metadata_edits_replace_sets_and_keep_defining_fields() {
    let db = TempDir::new().unwrap();
    create(&db, "Pull-up", "repetitions", "added-bodyweight");
    success(
        &db,
        &[
            "exercise",
            "update",
            "1",
            "--name",
            " Chin-up ",
            "--primary-muscle",
            "2",
            "--secondary-muscle",
            "4",
            "--secondary-muscle",
            "4",
            "--secondary-muscle",
            "6",
        ],
    );
    let edited = show(&db, "1");
    assert_eq!(edited["name"], "Chin-up");
    assert_eq!(edited["secondary_muscles"], serde_json::json!([4, 6]));
    assert_eq!(edited["equipment"], serde_json::json!([]));
    assert_eq!(edited["measurement"], "repetitions");
    assert_eq!(edited["load_convention"], "added-bodyweight");
    for (args, message) in [
        (
            vec!["exercise", "update", "1", "--name", " "],
            "must not be empty",
        ),
        (
            vec!["exercise", "update", "1", "--primary-muscle", "4"],
            "primary muscle",
        ),
        (
            vec![
                "exercise",
                "update",
                "1",
                "--name",
                "Changed",
                "--secondary-muscle",
                "999",
            ],
            "unknown muscles ID",
        ),
        (
            vec!["exercise", "update", "1", "--measurement", "distance"],
            "unexpected argument",
        ),
        (
            vec!["exercise", "update", "1", "--equipment", "1"],
            "unexpected argument",
        ),
        (
            vec!["exercise", "update", "1", "--load-convention", "external"],
            "unexpected argument",
        ),
    ] {
        failure(&db, &args, message);
        assert_eq!(show(&db, "1"), edited);
    }
    success(&db, &["exercise", "update", "1", "--clear-primary-muscle"]);
    assert_eq!(show(&db, "1")["primary_muscle"], serde_json::Value::Null);
    assert_eq!(
        show(&db, "1")["secondary_muscles"],
        serde_json::json!([4, 6])
    );
    success(
        &db,
        &["exercise", "update", "1", "--clear-secondary-muscles"],
    );
    assert_eq!(show(&db, "1")["secondary_muscles"], serde_json::json!([]));
}
#[test]
fn invalid_creates_leave_no_partial_exercises() {
    let db = TempDir::new().unwrap();
    for extra in [
        vec!["--equipment", "999"],
        vec!["--primary-muscle", "999"],
        vec!["--secondary-muscle", "999"],
        vec!["--primary-muscle", "1", "--secondary-muscle", "1"],
    ] {
        let mut args = vec![
            "exercise",
            "create",
            "--name",
            "Invalid",
            "--measurement",
            "duration",
            "--load-convention",
            "external",
        ];
        args.extend(extra);
        let output = run(&db, &args);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
        assert_eq!(success(&db, &["exercise", "list", "--json"]).trim(), "[]");
    }
    failure(&db, &["exercise", "show", "999"], "unknown exercise ID");
    failure(
        &db,
        &["exercise", "update", "999", "--name", "New"],
        "unknown exercise ID",
    );
    failure(&db, &["exercise", "update", "1"], "supply --name");
}
#[test]
fn every_measurement_and_load_convention_is_retained() {
    let db = TempDir::new().unwrap();
    let mut id = 0;
    for measurement in ["repetitions", "duration", "distance"] {
        for load in ["external", "added-bodyweight"] {
            id += 1;
            create(&db, &format!("{measurement} {load}"), measurement, load);
            let exercise = show(&db, &id.to_string());
            assert_eq!(exercise["measurement"], measurement);
            assert_eq!(exercise["load_convention"], load);
        }
    }
}
#[test]
fn human_output_explains_catalog_references() {
    let db = TempDir::new().unwrap();
    success(
        &db,
        &[
            "exercise",
            "create",
            "--name",
            "Press",
            "--measurement",
            "repetitions",
            "--load-convention",
            "external",
            "--equipment",
            "1",
            "--equipment",
            "1",
            "--primary-muscle",
            "1",
        ],
    );
    assert_eq!(show(&db, "1")["equipment"], serde_json::json!([1]));
    let human = success(&db, &["exercise", "show", "1"]);
    assert!(human.contains("Barbell"));
    assert!(human.contains("Chest"));
}

#[test]
fn database_option_works_after_subcommands_and_default_is_documented() {
    let home = TempDir::new().unwrap();
    let invoke = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_precision"))
            .env("HOME", home.path())
            .args(args)
            .output()
            .unwrap()
    };
    let help = invoke(&["--help"]);
    assert!(String::from_utf8_lossy(&help.stdout).contains("$HOME/.precision/precision.sqlite3"));
    let created = invoke(&[
        "exercise",
        "create",
        "--name",
        "Walk",
        "--measurement",
        "distance",
        "--load-convention",
        "external",
    ]);
    assert!(created.status.success());
    let path = home.path().join(".precision/precision.sqlite3");
    let read = invoke(&[
        "exercise",
        "show",
        "1",
        "--json",
        "--db",
        path.to_str().unwrap(),
    ]);
    assert!(read.status.success());
    let value: serde_json::Value = serde_json::from_slice(&read.stdout).unwrap();
    assert_eq!(value["name"], "Walk");
}

#[test]
fn concurrent_first_use_and_duplicate_names_save_only_one_exercise() {
    let db = TempDir::new().unwrap();
    let spawn = || {
        Command::new(env!("CARGO_BIN_EXE_precision"))
            .arg("--db")
            .arg(db.path().join("precision.sqlite3"))
            .args([
                "exercise",
                "create",
                "--name",
                "Squat",
                "--measurement",
                "repetitions",
                "--load-convention",
                "external",
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap()
    };
    let first = spawn();
    let second = spawn();
    let outputs = [
        first.wait_with_output().unwrap(),
        second.wait_with_output().unwrap(),
    ];
    assert_eq!(
        outputs
            .iter()
            .filter(|output| output.status.success())
            .count(),
        1
    );
    let failed = outputs
        .iter()
        .find(|output| !output.status.success())
        .unwrap();
    assert!(String::from_utf8_lossy(&failed.stderr).contains("already exists"));
    let list: serde_json::Value =
        serde_json::from_str(&success(&db, &["exercise", "list", "--json"])).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 1);
}
