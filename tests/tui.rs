use precision::{
    exercises::Store,
    tui::{Action, Session},
};

#[test]
fn start_quit_and_explicit_resume_preserve_draft() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("training.db");
    let mut session = Session::open(&path, "2026-10-06".into()).unwrap();
    session.act(Action::Start).unwrap();
    assert!(session.render(80, 24).contains("Draft ID: 1"));
    session.act(Action::Quit).unwrap();
    let mut resumed = Session::open(&path, "2026-10-07".into()).unwrap();
    resumed.act(Action::Drafts).unwrap();
    resumed.act(Action::Select).unwrap();
    assert!(resumed.render(80, 24).contains("Actual activity"));
    assert_eq!(
        Store::open(&path).unwrap().workout(1).unwrap().date,
        "2026-10-06"
    );
}

#[test]
fn invalid_start_keeps_input_and_creates_no_draft() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("training.db");
    let mut session = Session::open(&path, "bad date".into()).unwrap();
    assert!(session.act(Action::Start).is_err());
    assert_eq!(session.date, "bad date");
    assert!(session.render(80, 24).contains("Error:"));
    assert!(
        Store::open(&path)
            .unwrap()
            .workouts(true)
            .unwrap()
            .is_empty()
    );
    session.date = "2026-10-06".into();
    session.act(Action::Start).unwrap();
    assert!(session.error.is_empty());
}

#[test]
fn reuse_copies_current_source_and_inspection_keeps_intention_separate() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("training.db");
    let mut store = Store::open(&path).unwrap();
    let routine = store.save_routine(None, serde_json::from_value(serde_json::json!({
        "schema_version":1,"name":"Practice","notes":"Preserved note",
        "sets":[{"notes":"Complex note","portions":[{"exercise_id":1,"repetitions":5,"notes":"Portion note"},{"exercise_id":2}]}]
    })).unwrap()).unwrap();
    let mut session = Session::open(&path, "2026-10-06".into()).unwrap();
    session.routine = routine.id.unwrap().to_string();
    session.act(Action::Start).unwrap();
    assert!(session.render(80, 24).contains("Actual activity"));
    session.act(Action::Tab).unwrap();
    let mut inspected = String::new();
    for _ in 0..100 {
        inspected.push_str(&session.render(80, 24));
        session.act(Action::Down).unwrap();
    }
    for text in [
        "Preserved intention",
        "Preserved note",
        "Complex note",
        "Portion note",
        "repetitions",
    ] {
        assert!(inspected.contains(text), "{text}");
    }
    session.act(Action::Reuse).unwrap();
    session.act(Action::Select).unwrap();
    assert!(session.render(80, 24).contains("Draft ID: 2"));
    assert!(store.workout(2).unwrap().sets.is_empty());
    store.delete_routine(routine.id.unwrap()).unwrap();
    session.act(Action::Reuse).unwrap();
    assert!(session.act(Action::Select).is_err());
    assert_eq!(store.workouts(true).unwrap().len(), 2);
}

#[test]
fn resize_preserves_text_and_compact_layout_shows_shortcuts() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(&dir.path().join("training.db"), "2026-10-06".into()).unwrap();
    session.act(Action::Tab).unwrap();
    for c in "2026-10-06T18:00:00+07:00".chars() {
        session.act(Action::Text(c)).unwrap();
    }
    for (w, h) in [(80, 24), (120, 40), (80, 24)] {
        let screen = session.render(w, h);
        assert!(screen.contains("F1 Help"));
        assert!(screen.contains("F10 Quit"));
        assert!(screen.lines().count() <= h);
        assert!(screen.lines().all(|l| l.chars().count() <= w));
    }
    session.act(Action::Start).unwrap();
    assert!(session.render(80, 24).contains("Draft ID: 1"));
}

#[test]
fn noninteractive_startup_explains_terminal_requirement_without_opening_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("untouched.db");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_precision"))
        .args(["tui", "--db"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires interactive"));
    assert!(!path.exists());
}

#[test]
fn routine_list_scrolls_at_laptop_size() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("training.db");
    let mut store = Store::open(&path).unwrap();
    for n in 1..=30 {
        store.save_routine(None, serde_json::from_value(serde_json::json!({"schema_version":1,"name":format!("Routine {n}"),"sets":[{"portions":[{"exercise_id":1}]}]})).unwrap()).unwrap();
    }
    let mut session = Session::open(&path, "2026-10-06".into()).unwrap();
    for _ in 0..29 {
        session.act(Action::Down).unwrap();
    }
    assert!(session.render(80, 24).contains("30: Routine 30"));
}

#[test]
fn wide_unicode_text_fits_terminal_columns() {
    let dir = tempfile::tempdir().unwrap();
    let session = Session::open(&dir.path().join("training.db"), "界".repeat(80)).unwrap();
    let screen = session.render(80, 24);
    assert!(
        screen
            .lines()
            .all(|l| unicode_width::UnicodeWidthStr::width(l) <= 80)
    );
    assert!(screen.contains("F10 Quit"));
}

#[test]
fn ordinary_set_search_save_cancel_and_restart_preserve_exact_values() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("record.db");
    let mut session = Session::open(&path, "2026-10-06".into()).unwrap();
    session.act(Action::Start).unwrap();
    session.act(Action::Add).unwrap();
    for c in "SQUAT".chars() {
        session.act(Action::Text(c)).unwrap();
    }
    assert!(session.render(80, 24).contains("Barbell"));
    session.act(Action::Select).unwrap();
    session.act(Action::Tab).unwrap();
    for c in "20.123456".chars() {
        session.act(Action::Text(c)).unwrap();
    }
    session.act(Action::Tab).unwrap();
    session.act(Action::Tab).unwrap();
    session.act(Action::Text('0')).unwrap();
    session.act(Action::Save).unwrap();
    session.act(Action::Duplicate).unwrap();
    assert!(session.render(80, 24).contains("Unsaved"));
    assert!(session.act(Action::Save).is_err());
    session.act(Action::Escape).unwrap();
    session.act(Action::Quit).unwrap();
    let mut resumed = Session::open(&path, "2026-10-07".into()).unwrap();
    resumed.act(Action::Drafts).unwrap();
    resumed.act(Action::Select).unwrap();
    let saved = Store::open(&path).unwrap().workout(1).unwrap();
    assert_eq!(saved.sets.len(), 1);
    assert_eq!(
        saved.sets[0].kilograms.as_ref().unwrap().to_string(),
        "20.123456"
    );
    assert_eq!(
        saved.sets[0].portions[0]
            .repetitions
            .as_ref()
            .unwrap()
            .to_string(),
        "0"
    );
}

#[test]
fn recording_prescribed_structure_keeps_targets_separate_and_actual_blank() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("prescribed.db");
    let mut store = Store::open(&path).unwrap();
    let routine = store.save_routine(None, serde_json::from_value(serde_json::json!({
        "schema_version":1,"name":"Ordinary","sets":[{"type":"warmup","kilograms":25.5,"rpe":7.5,"notes":"intended note","portions":[{"exercise_id":1,"repetitions":5}]}]
    })).unwrap()).unwrap();
    let mut session = Session::open(&path, "2026-10-06".into()).unwrap();
    session.routine = routine.id.unwrap().to_string();
    session.act(Action::Start).unwrap();
    session.act(Action::Tab).unwrap();
    session.act(Action::Record).unwrap();
    let screen = session.render(80, 24);
    assert!(screen.contains("Targets only"));
    assert!(screen.contains("minimum successful repetitions 5"));
    assert!(session.act(Action::Save).is_err());
    for _ in 0..3 {
        session.act(Action::Tab).unwrap();
    }
    session.act(Action::Text('2')).unwrap();
    session.act(Action::Save).unwrap();
    let saved = store.workout(1).unwrap();
    assert_eq!(saved.sets[0].kind, "warmup");
    assert_eq!(
        saved.sets[0].portions[0]
            .repetitions
            .as_ref()
            .unwrap()
            .to_string(),
        "2"
    );
    assert_eq!(
        saved.intention.sets[0].portions[0]
            .repetitions
            .as_ref()
            .unwrap()
            .to_string(),
        "5"
    );
    assert!(saved.sets[0].rpe.is_none());
    assert!(saved.sets[0].notes.is_none());
}

#[test]
fn finish_and_discard_require_explicit_confirmations_and_check_loaded_revisions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("finish.db");
    let mut session = Session::open(&path, "2026-10-06".into()).unwrap();
    session.act(Action::Start).unwrap();
    assert!(session.act(Action::Finish).is_err());
    session.act(Action::Add).unwrap();
    session.act(Action::Select).unwrap();
    for _ in 0..3 {
        session.act(Action::Tab).unwrap();
    }
    session.act(Action::Text('5')).unwrap();
    assert!(session.act(Action::Finish).is_err());
    session.act(Action::Save).unwrap();
    session.act(Action::Finish).unwrap();
    assert!(session.render(80, 24).contains("read-only"));
    session.act(Action::Escape).unwrap();
    assert_eq!(
        Store::open(&path).unwrap().workout(1).unwrap().state,
        "draft"
    );
    session.act(Action::Finish).unwrap();
    session.act(Action::Select).unwrap();
    assert_eq!(
        Store::open(&path).unwrap().workout(1).unwrap().state,
        "finished"
    );
    assert!(session.act(Action::Add).is_err());
    session.act(Action::Start).unwrap();
    session.act(Action::Discard).unwrap();
    assert!(session.render(80, 24).contains("permanently"));
    session.act(Action::Escape).unwrap();
    assert!(Store::open(&path).unwrap().workout(2).is_ok());
    session.act(Action::Discard).unwrap();
    session.act(Action::Select).unwrap();
    assert!(Store::open(&path).unwrap().workout(2).is_err());
}

#[test]
fn stale_form_retains_input_and_latest_inspection_never_merges() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("conflict.db");
    let mut session = Session::open(&path, "2026-10-06".into()).unwrap();
    session.act(Action::Start).unwrap();
    session.act(Action::Add).unwrap();
    session.act(Action::Select).unwrap();
    for _ in 0..3 {
        session.act(Action::Tab).unwrap();
    }
    session.act(Action::Text('8')).unwrap();
    let mut other = Store::open(&path).unwrap();
    let mut workout = other.workout(1).unwrap();
    workout.notes = Some("new saved notes".into());
    other.save_workout(1, workout).unwrap();
    assert!(session.act(Action::Save).is_err());
    assert!(session.render(80, 24).contains("stale"));
    session.act(Action::Tab).unwrap();
    assert!(session.render(80, 24).contains("stale"));
    session.act(Action::InspectLatest).unwrap();
    assert!(session.render(80, 24).contains("new saved notes"));
    session.act(Action::Escape).unwrap();
    assert!(session.render(80, 24).contains("Attempted repetitions: 8"));
    assert!(session.act(Action::Save).is_err());
    assert!(other.workout(1).unwrap().sets.is_empty());
    session.act(Action::Reopen).unwrap();
    assert!(session.render(80, 24).contains("Actual activity"));
    session.act(Action::Add).unwrap();
    session.act(Action::Select).unwrap();
    for _ in 0..3 {
        session.act(Action::Tab).unwrap();
    }
    session.act(Action::Text('1')).unwrap();
    session.act(Action::Save).unwrap();
    assert_eq!(
        other.workout(1).unwrap().notes.as_deref(),
        Some("new saved notes")
    );
}

#[test]
fn compact_form_keeps_errors_targets_and_all_actions_discoverable() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(&dir.path().join("compact.db"), "2026-10-06".into()).unwrap();
    session.act(Action::Start).unwrap();
    session.act(Action::Add).unwrap();
    session.act(Action::Select).unwrap();
    assert!(session.act(Action::Save).is_err());
    session.act(Action::Tab).unwrap();
    assert!(
        session
            .render(80, 24)
            .contains("Actual quantity is required")
    );
    session.act(Action::Help).unwrap();
    let help = session.render(80, 24);
    for text in ["F7", "F8", "F9", "F11", "Shift-F11", "F12"] {
        assert!(help.contains(text), "{help}");
    }
    session.act(Action::Escape).unwrap();
    assert!(session.render(80, 24).contains("Unsaved"));
}

fn type_text(session: &mut Session, text: &str) {
    for c in text.chars() {
        session.act(Action::Text(c)).unwrap();
    }
}

#[test]
fn measurement_forms_preserve_signed_exact_load_observations_and_all_notes() {
    use precision::exercise_types::{LoadConvention, Measurement};
    use precision::exercises::Exercise;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("measurements.db");
    let mut store = Store::open(&path).unwrap();
    for (name, measurement) in [
        ("Timed hang", Measurement::Duration),
        ("Measured carry", Measurement::Distance),
    ] {
        store
            .create(Exercise {
                schema_version: 1,
                id: 0,
                name: name.into(),
                measurement,
                load_convention: LoadConvention::AddedBodyweight,
                equipment: vec![4],
                primary_muscle: None,
                secondary_muscles: vec![],
            })
            .unwrap();
    }
    let mut session = Session::open(&path, "2026-10-06".into()).unwrap();
    session.act(Action::Start).unwrap();
    for (name, quantity, label) in [
        ("TIMED HANG", "0.123456", "Seconds"),
        ("measured carry", "1.000001", "Metres"),
    ] {
        session.act(Action::Add).unwrap();
        type_text(&mut session, name);
        session.act(Action::Select).unwrap();
        assert!(session.render(80, 24).contains(label));
        for _ in 0..4 {
            session.act(Action::Backspace).unwrap();
        }
        type_text(&mut session, "warmup");
        session.act(Action::Tab).unwrap();
        type_text(&mut session, "-10.123456");
        session.act(Action::Tab).unwrap();
        type_text(&mut session, "purple band");
        session.act(Action::Tab).unwrap();
        type_text(&mut session, quantity);
        session.act(Action::Tab).unwrap();
        type_text(&mut session, "9.5");
        session.act(Action::Tab).unwrap();
        type_text(&mut session, "1");
        session.act(Action::Tab).unwrap();
        type_text(&mut session, "2");
        session.act(Action::Tab).unwrap();
        type_text(&mut session, "set observation");
        session.act(Action::Tab).unwrap();
        type_text(&mut session, "portion observation");
        session.act(Action::Tab).unwrap();
        // Existing session notes survive the second set form unchanged.
        if store.workout(1).unwrap().notes.is_none() {
            type_text(&mut session, "session observation");
        }
        session.act(Action::Save).unwrap();
    }
    let saved = store.workout(1).unwrap();
    assert_eq!(saved.notes.as_deref(), Some("session observation"));
    assert_eq!(
        saved.sets[0].portions[0]
            .seconds
            .as_ref()
            .unwrap()
            .to_string(),
        "0.123456"
    );
    assert_eq!(
        saved.sets[1].portions[0]
            .metres
            .as_ref()
            .unwrap()
            .to_string(),
        "1.000001"
    );
    for set in &saved.sets {
        assert_eq!(set.kilograms.as_ref().unwrap().to_string(), "-10.123456");
        assert_eq!(set.rpe.as_ref().unwrap().to_string(), "9.5");
        assert_eq!((set.white_flags, set.red_flags), (Some(1), Some(2)));
        assert_eq!(set.notes.as_deref(), Some("set observation"));
        assert_eq!(
            set.portions[0].notes.as_deref(),
            Some("portion observation")
        );
    }
    session.act(Action::Select).unwrap();
    for _ in 0..3 {
        session.act(Action::Tab).unwrap();
    }
    for _ in 0..8 {
        session.act(Action::Backspace).unwrap();
    }
    type_text(&mut session, "0");
    session.act(Action::Save).unwrap();
    assert_eq!(
        store.workout(1).unwrap().sets[1].portions[0]
            .metres
            .as_ref()
            .unwrap()
            .to_string(),
        "0"
    );
}

#[test]
fn validation_and_storage_failures_retain_text_and_support_retry() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("retry.db");
    let mut session = Session::open(&path, "2026-10-06".into()).unwrap();
    session.act(Action::Start).unwrap();
    session.act(Action::Add).unwrap();
    session.act(Action::Select).unwrap();
    session.act(Action::Tab).unwrap();
    type_text(&mut session, "0.0000001");
    session.act(Action::Tab).unwrap();
    session.act(Action::Tab).unwrap();
    type_text(&mut session, "3");
    assert!(session.act(Action::Save).is_err());
    assert!(session.render(80, 24).contains("0.0000001"));
    session.act(Action::Up).unwrap();
    session.act(Action::Up).unwrap();
    session.act(Action::Backspace).unwrap();
    session.act(Action::Backspace).unwrap();
    type_text(&mut session, "1");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch("BEGIN IMMEDIATE").unwrap();
    assert!(session.act(Action::Save).is_err());
    assert!(session.render(80, 24).contains("Attempted repetitions: 3"));
    connection.execute_batch("ROLLBACK").unwrap();
    session.act(Action::Save).unwrap();
    let saved = Store::open(&path).unwrap().workout(1).unwrap();
    assert_eq!(
        saved.sets[0].kilograms.as_ref().unwrap().to_string(),
        "0.000001"
    );
    assert_eq!(saved.revision, Some(1));
}

#[test]
fn navigating_long_activity_keeps_selected_set_visible_with_wrapped_notes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("navigation.db");
    let mut store = Store::open(&path).unwrap();
    store
        .start_workout("2026-10-06".into(), None, None)
        .unwrap();
    let sets = (1..=12).map(|reps| serde_json::json!({"notes":"long observation ".repeat(20),"portions":[{"exercise_id":1,"repetitions":reps}]})).collect::<Vec<_>>();
    store.save_workout(1,serde_json::from_value(serde_json::json!({"schema_version":1,"revision":0,"date":"2026-10-06","sets":sets})).unwrap()).unwrap();
    let saved = store.workout(1).unwrap();
    let mut session = Session::open(&path, "2026-10-06".into()).unwrap();
    session.act(Action::Drafts).unwrap();
    session.act(Action::Select).unwrap();
    for i in 1..12 {
        session.act(Action::Down).unwrap();
        let screen = session.render(80, 24);
        assert!(
            screen.contains(&format!("> Set {} ID {}", i + 1, saved.sets[i].id.unwrap())),
            "{screen}"
        );
        assert!(
            screen.contains(&format!("{} attempted repetitions", i + 1)),
            "{screen}"
        );
    }
    session.act(Action::Up).unwrap();
    assert!(session.render(80, 24).contains("> Set 11 ID"));
}

#[test]
fn duplicate_clears_observations_without_copying_rest_or_superset_membership() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("duplicate.db");
    let mut store = Store::open(&path).unwrap();
    store
        .start_workout("2026-10-06".into(), None, None)
        .unwrap();
    let set = serde_json::json!({"type":"warmup","kilograms":15.125,"load_description":"bar and plates","rpe":7.5,"white_flags":1,"red_flags":2,"notes":"set observation","portions":[{"exercise_id":1,"repetitions":4,"notes":"portion observation"}]});
    store.save_workout(1,serde_json::from_value(serde_json::json!({"schema_version":1,"revision":0,"date":"2026-10-06","notes":"session note","rest":[30],"sets":[set.clone(),set]})).unwrap()).unwrap();
    let mut original = store.workout(1).unwrap();
    let ids = original
        .sets
        .iter()
        .map(|s| s.id.unwrap())
        .collect::<Vec<_>>();
    original.supersets.push(precision::organization::Superset {
        id: None,
        set_ids: ids.clone(),
    });
    store.save_workout(1, original).unwrap();
    let mut session = Session::open(&path, "2026-10-06".into()).unwrap();
    session.act(Action::Drafts).unwrap();
    session.act(Action::Select).unwrap();
    session.act(Action::Duplicate).unwrap();
    assert!(session.act(Action::Save).is_err());
    let screen = session.render(80, 24);
    assert!(!screen.contains("set observation"));
    assert!(!screen.contains("portion observation"));
    for _ in 0..3 {
        session.act(Action::Tab).unwrap();
    }
    type_text(&mut session, "0");
    session.act(Action::Save).unwrap();
    let saved = store.workout(1).unwrap();
    let duplicate = &saved.sets[2];
    assert_eq!(duplicate.kind, "warmup");
    assert_eq!(duplicate.kilograms.as_ref().unwrap().to_string(), "15.125");
    assert_eq!(
        duplicate.load_description.as_deref(),
        Some("bar and plates")
    );
    assert!(duplicate.rpe.is_none());
    assert_eq!((duplicate.white_flags, duplicate.red_flags), (None, None));
    assert!(duplicate.notes.is_none());
    assert!(duplicate.portions[0].notes.is_none());
    assert_eq!(saved.supersets[0].set_ids, ids);
    assert_eq!(
        saved.rest.as_ref().unwrap()[0]
            .0
            .as_ref()
            .unwrap()
            .to_string(),
        "30"
    );
    assert!(saved.rest.as_ref().unwrap()[1].0.is_none());
    assert_eq!(saved.notes.as_deref(), Some("session note"));
}
