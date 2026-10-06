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
