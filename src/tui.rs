//! Keyboard adapter and headless draft session. Storage remains authoritative.
mod form;
use crate::{
    exercises::{Exercise, Store},
    workouts::Workout,
};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use form::{Field, SetForm};
use std::{
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
};
use unicode_casefold::UnicodeCaseFold;
use unicode_width::UnicodeWidthChar;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy)]
pub enum Action {
    Help,
    InspectLatest,
    Reopen,
    Finish,
    Discard,
    Record,
    Add,
    Save,
    Duplicate,
    Start,
    Drafts,
    Reuse,
    Select,
    Up,
    Down,
    PageUp,
    PageDown,
    Tab,
    Escape,
    Quit,
    Text(char),
    Backspace,
}
#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Home,
    Drafts,
    Reuse,
    Workout,
    Picker,
    Form,
    Confirm,
    Latest,
    Help,
}
pub struct Session {
    help_return: Screen,
    latest: Option<Workout>,
    return_screen: Screen,
    conflicted: bool,
    discarding: bool,
    end: String,
    form: Option<SetForm>,
    editing: Option<usize>,
    query: String,
    exercises: Vec<Exercise>,
    width: std::cell::Cell<usize>,
    store: Store,
    path: PathBuf,
    screen: Screen,
    pub date: String,
    pub start: String,
    pub routine: String,
    field: usize,
    selected: usize,
    entries: Vec<Workout>,
    workout: Option<Workout>,
    intention: bool,
    scroll: usize,
    pub error: String,
    pub quit: bool,
}
impl Session {
    pub fn open(path: &Path, today: String) -> Result<Self> {
        Ok(Self {
            help_return: Screen::Home,
            latest: None,
            return_screen: Screen::Workout,
            conflicted: false,
            discarding: false,
            end: String::new(),
            form: None,
            editing: None,
            query: String::new(),
            exercises: vec![],
            width: std::cell::Cell::new(80),
            store: Store::open(path)?,
            path: path.into(),
            screen: Screen::Home,
            date: today,
            start: String::new(),
            routine: String::new(),
            field: 0,
            selected: 0,
            entries: vec![],
            workout: None,
            intention: false,
            scroll: 0,
            error: String::new(),
            quit: false,
        })
    }
    pub fn act(&mut self, action: Action) -> Result<()> {
        let resolves = matches!(
            action,
            Action::Save | Action::Select | Action::Start | Action::Reopen
        ) || (matches!(action, Action::Escape)
            && !matches!(self.screen, Screen::Help | Screen::Latest));
        let result = self.apply(action);
        match &result {
            Err(e) => {
                if e.downcast_ref::<crate::workouts::RevisionConflict>()
                    .is_some()
                {
                    self.conflicted = true;
                }
                self.error = e.to_string();
            }
            Ok(_) if resolves && !self.conflicted => self.error.clear(),
            Ok(_) => {}
        }
        result
    }
    fn apply(&mut self, action: Action) -> Result<()> {
        if matches!(action, Action::Help) {
            if self.screen != Screen::Help {
                self.help_return = self.screen;
            }
            self.screen = Screen::Help;
            return Ok(());
        }
        if self.screen == Screen::Help {
            match action {
                Action::Escape => self.screen = self.help_return,
                Action::Quit => self.quit = true,
                _ => {}
            }
            return Ok(());
        }
        if matches!(action, Action::InspectLatest) {
            let id = self
                .workout
                .as_ref()
                .and_then(|w| w.id)
                .ok_or("Select a workout first")?;
            self.latest = Some(self.store.workout(id)?);
            if self.screen != Screen::Latest {
                self.return_screen = self.screen;
            }
            self.screen = Screen::Latest;
            self.scroll = 0;
            return Ok(());
        }
        if matches!(action, Action::Reopen) {
            let id = self
                .workout
                .as_ref()
                .and_then(|w| w.id)
                .ok_or("Select a workout first")?;
            let latest = self.store.workout(id)?;
            self.workout = Some(latest);
            self.form = None;
            self.latest = None;
            self.conflicted = false;
            self.intention = false;
            self.screen = Screen::Workout;
            self.selected = 0;
            self.scroll = 0;
            return Ok(());
        }
        if self.screen == Screen::Latest {
            match action {
                Action::Escape => self.screen = self.return_screen,
                Action::Quit => self.quit = true,
                Action::Up => self.scroll = self.scroll.saturating_sub(1),
                Action::PageUp => self.scroll = self.scroll.saturating_sub(10),
                Action::Down | Action::PageDown => {
                    let step = if matches!(action, Action::PageDown) {
                        10
                    } else {
                        1
                    };
                    self.scroll = (self.scroll + step).min(
                        wrap_lines(
                            self.activity_lines(self.latest.as_ref().unwrap())?,
                            self.width.get(),
                        )
                        .len()
                        .saturating_sub(1),
                    )
                }
                _ => {
                    return Err(
                        "Latest activity is read-only; Esc returns, Shift-F11 abandons/reopens"
                            .into(),
                    );
                }
            }
            return Ok(());
        }
        if self.screen == Screen::Confirm {
            match action {
                Action::Quit => self.quit = true,
                Action::Escape => self.screen = Screen::Workout,
                Action::Text(c) if !self.discarding => self.end.push(c),
                Action::Backspace if !self.discarding => {
                    self.end.pop();
                }
                Action::Select => {
                    if self.conflicted {
                        return Err(crate::workouts::RevisionConflict.into());
                    }
                    let w = self.workout.as_ref().unwrap();
                    if self.discarding {
                        self.store
                            .discard_workout(w.id.unwrap(), w.revision.unwrap())?;
                        self.workout = None;
                        self.screen = Screen::Home;
                    } else {
                        self.workout = Some(self.store.finish_workout(
                            w.id.unwrap(),
                            w.revision.unwrap(),
                            form::optional_text(&self.end),
                        )?);
                        self.screen = Screen::Workout;
                    }
                }
                _ => return Err("Enter confirms; Esc cancels".into()),
            }
            return Ok(());
        }
        if self.screen == Screen::Form {
            match action {
                Action::Quit => self.quit = true,
                Action::Escape => {
                    self.form = None;
                    self.screen = Screen::Workout;
                }
                Action::Tab | Action::Down => self.field = (self.field + 1) % Field::ALL.len(),
                Action::Up => self.field = (self.field + Field::ALL.len() - 1) % Field::ALL.len(),
                Action::Text(c) => self
                    .form
                    .as_mut()
                    .unwrap()
                    .input(Field::ALL[self.field])
                    .push(c),
                Action::Backspace => {
                    self.form
                        .as_mut()
                        .unwrap()
                        .input(Field::ALL[self.field])
                        .pop();
                }
                Action::Save | Action::Select => self.save_form()?,
                _ => return Err("Save or cancel the unresolved form first".into()),
            }
            return Ok(());
        }
        if self.screen == Screen::Picker {
            match action {
                Action::Quit => self.quit = true,
                Action::Escape => self.screen = Screen::Workout,
                Action::Text(c) => {
                    self.query.push(c);
                    self.search()?;
                }
                Action::Backspace => {
                    self.query.pop();
                    self.search()?;
                }
                Action::Up => self.selected = self.selected.saturating_sub(1),
                Action::Down => {
                    self.selected = (self.selected + 1).min(self.exercises.len().saturating_sub(1))
                }
                Action::Select => {
                    let id = self
                        .exercises
                        .get(self.selected)
                        .ok_or("No matching exercises")?
                        .id;
                    self.form = Some(SetForm::new(
                        self.store.show(id)?,
                        &self.workout.as_ref().unwrap().notes,
                    ));
                    self.editing = None;
                    self.field = 0;
                    self.screen = Screen::Form;
                }
                _ => return Err("Choose an exercise or cancel first".into()),
            }
            return Ok(());
        }
        match action {
            Action::InspectLatest | Action::Reopen | Action::Help => unreachable!(),
            Action::Add => {
                self.require_workout()?;
                self.query.clear();
                self.search()?;
                self.screen = Screen::Picker;
            }
            Action::Duplicate => self.edit_set(true)?,
            Action::Record => self.record_structure()?,
            Action::Finish | Action::Discard => {
                self.require_workout()?;
                self.discarding = matches!(action, Action::Discard);
                if !self.discarding && self.workout.as_ref().unwrap().sets.is_empty() {
                    return Err("Finish requires saved performed activity".into());
                }
                self.end.clear();
                self.screen = Screen::Confirm;
            }
            Action::Save => return Err("Open a form before saving".into()),
            Action::Quit => self.quit = true,
            Action::Escape => {
                self.screen = Screen::Home;
                self.scroll = 0;
            }
            Action::Start => {
                let routine = if self.routine.trim().is_empty() {
                    None
                } else {
                    Some(self.routine.trim().parse::<i64>()?)
                };
                self.workout = Some(self.store.start_workout(
                    self.date.clone(),
                    self.optional_start(),
                    routine,
                )?);
                self.screen = Screen::Workout;
                self.scroll = 0;
                self.intention = false;
                self.conflicted = false;
                self.selected = 0;
            }
            Action::Drafts | Action::Reuse => {
                let drafts = matches!(action, Action::Drafts);
                self.entries = self
                    .store
                    .workouts(true)?
                    .into_iter()
                    .filter(|w| !drafts || w.state == "draft")
                    .collect();
                self.screen = if drafts {
                    Screen::Drafts
                } else {
                    Screen::Reuse
                };
                self.selected = 0;
            }
            Action::Select => match self.screen {
                Screen::Home => return self.apply(Action::Start),
                Screen::Drafts | Screen::Reuse => {
                    let id = self
                        .entries
                        .get(self.selected)
                        .ok_or("No workouts available")?
                        .id
                        .unwrap();
                    self.workout = Some(if self.screen == Screen::Reuse {
                        self.store
                            .reuse_workout(id, self.date.clone(), self.optional_start())?
                    } else {
                        self.store.workout(id)?
                    });
                    self.screen = Screen::Workout;
                    self.scroll = 0;
                    self.intention = false;
                    self.conflicted = false;
                    self.selected = 0;
                }
                Screen::Workout => self.edit_set(false)?,
                Screen::Picker | Screen::Form | Screen::Confirm | Screen::Latest | Screen::Help => {
                    unreachable!()
                }
            },
            Action::Tab => {
                if self.screen == Screen::Workout {
                    self.intention = !self.intention;
                    self.selected = 0;
                    self.scroll = 0;
                } else if self.screen == Screen::Home {
                    self.field = (self.field + 1) % 3;
                }
            }
            Action::Up => {
                if self.screen == Screen::Workout {
                    self.selected = self.selected.saturating_sub(1);
                    if self.intention {
                        self.scroll = self.scroll.saturating_sub(1);
                    } else {
                        self.scroll_to_actual_selection()?;
                    }
                } else {
                    self.selected = self.selected.saturating_sub(1);
                }
            }
            Action::Down => {
                if self.screen == Screen::Workout {
                    let count = if self.intention {
                        self.workout.as_ref().unwrap().intention.sets.len()
                    } else {
                        self.workout.as_ref().unwrap().sets.len()
                    };
                    self.selected = (self.selected + 1).min(count.saturating_sub(1));
                    if self.intention {
                        self.scroll = (self.scroll + 1).min(
                            wrap_lines(self.detail_lines()?, self.width.get())
                                .len()
                                .saturating_sub(1),
                        );
                    } else {
                        self.scroll_to_actual_selection()?;
                    }
                } else if self.screen == Screen::Home {
                    self.selected =
                        (self.selected + 1).min(self.store.routines()?.len().saturating_sub(1));
                } else {
                    self.selected = (self.selected + 1).min(self.entries.len().saturating_sub(1));
                }
            }
            Action::PageUp | Action::PageDown => {
                if self.screen == Screen::Workout {
                    self.scroll = if matches!(action, Action::PageUp) {
                        self.scroll.saturating_sub(10)
                    } else {
                        (self.scroll + 10).min(
                            wrap_lines(self.detail_lines()?, self.width.get())
                                .len()
                                .saturating_sub(1),
                        )
                    };
                }
            }
            Action::Text(c) => {
                if self.screen == Screen::Home {
                    self.input().push(c);
                }
            }
            Action::Backspace => {
                if self.screen == Screen::Home {
                    self.input().pop();
                }
            }
        }
        Ok(())
    }
    fn require_workout(&self) -> Result<()> {
        if self.screen != Screen::Workout
            || self.workout.as_ref().is_none_or(|w| w.state != "draft")
        {
            return Err("Select a draft first; finished activity is read-only".into());
        }
        Ok(())
    }
    fn search(&mut self) -> Result<()> {
        let query = self.query.case_fold().collect::<String>();
        self.exercises = self
            .store
            .list()?
            .into_iter()
            .filter(|e| e.name.case_fold().collect::<String>().contains(&query))
            .collect();
        self.selected = 0;
        Ok(())
    }
    fn edit_set(&mut self, duplicate: bool) -> Result<()> {
        self.require_workout()?;
        if self.conflicted {
            return Err(crate::workouts::RevisionConflict.into());
        }
        if self.intention {
            return Err("Preserved intention is read-only; use record structure".into());
        }
        let workout = self.workout.as_ref().unwrap();
        let set = workout
            .sets
            .get(self.selected)
            .ok_or("No saved set selected")?;
        if set.portions.len() != 1 {
            return Err(
                "This form supports ordinary single-portion sets; use CLI for complexes".into(),
            );
        }
        self.form = Some(SetForm::from_set(
            self.store.show(set.portions[0].exercise_id)?,
            set,
            &workout.notes,
            duplicate,
        ));
        self.editing = if duplicate { None } else { Some(self.selected) };
        self.field = 0;
        self.screen = Screen::Form;
        Ok(())
    }
    fn record_structure(&mut self) -> Result<()> {
        self.require_workout()?;
        if !self.intention {
            return Err(
                "Tab to preserved intention, select a set, then F7 records its structure".into(),
            );
        }
        let w = self.workout.as_ref().unwrap();
        let set = w
            .intention
            .sets
            .get(self.selected)
            .ok_or("No prescribed set selected")?;
        if set.portions.len() != 1 {
            return Err(
                "Ordinary recording requires a single-portion structure; use CLI for complexes"
                    .into(),
            );
        }
        let portion = &set.portions[0];
        let mut form = SetForm::new(self.store.show(portion.exercise_id)?, &w.notes);
        *form.input(Field::Kind) = set.kind.clone();
        *form.input(Field::Kilograms) = set
            .kilograms
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default();
        *form.input(Field::LoadDescription) = set.load_description.clone().unwrap_or_default();
        let (label, value) = match form.exercise.measurement {
            crate::exercise_types::Measurement::Repetitions => {
                ("minimum successful repetitions", &portion.repetitions)
            }
            crate::exercise_types::Measurement::Duration => ("minimum seconds", &portion.seconds),
            crate::exercise_types::Measurement::Distance => ("minimum metres", &portion.metres),
        };
        form.target = format!(
            "Targets only: {label} {}; kg >= {}; RPE <= {}",
            value
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or("unspecified".into()),
            form.value(Field::Kilograms),
            set.rpe
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or("unspecified".into())
        );
        self.form = Some(form);
        self.editing = None;
        self.field = 0;
        self.screen = Screen::Form;
        Ok(())
    }
    fn save_form(&mut self) -> Result<()> {
        if self.conflicted {
            return Err(crate::workouts::RevisionConflict.into());
        }
        let form = self.form.as_ref().unwrap();
        let set = form.parse()?;
        let original = self.workout.as_ref().unwrap();
        let mut updated: Workout = serde_json::from_value(serde_json::to_value(original)?)?;
        updated.notes = form::optional_text(form.value(Field::SessionNotes));
        if let Some(index) = self.editing {
            updated.sets[index] = set;
        } else {
            if !updated.sets.is_empty() {
                updated
                    .rest
                    .as_mut()
                    .unwrap()
                    .push(crate::organization::Rest(None));
            }
            updated.sets.push(set);
        }
        let saved = self.store.save_workout(original.id.unwrap(), updated)?;
        self.selected = self.editing.unwrap_or(saved.sets.len() - 1);
        self.workout = Some(saved);
        self.intention = false;
        self.form = None;
        self.screen = Screen::Workout;
        self.scroll_to_actual_selection()?;
        Ok(())
    }
    fn optional_start(&self) -> Option<String> {
        if self.start.is_empty() {
            None
        } else {
            Some(self.start.clone())
        }
    }
    fn input(&mut self) -> &mut String {
        match self.field {
            0 => &mut self.date,
            1 => &mut self.start,
            _ => &mut self.routine,
        }
    }
    fn scroll_to_actual_selection(&mut self) -> Result<()> {
        let w = self.workout.as_ref().unwrap();
        let lines = self.activity_lines(w)?;
        let header = lines
            .iter()
            .position(|line| line.starts_with("> Set "))
            .unwrap_or(0);
        self.scroll = wrap_lines(lines.into_iter().take(header).collect(), self.width.get()).len();
        Ok(())
    }
    fn activity_lines(&self, w: &Workout) -> Result<Vec<String>> {
        let mut lines = vec![
            format!("Date: {} Start: {:?} End: {:?}", w.date, w.start, w.end),
            format!(
                "Revision: {} | {} | Session notes: {}",
                w.revision.unwrap(),
                w.state,
                w.notes.as_deref().unwrap_or("none")
            ),
        ];
        for (i, set) in w.sets.iter().enumerate() {
            lines.push(format!(
                "{} Set {} ID {} | {} | kg {} | {}",
                if i == self.selected { ">" } else { " " },
                i + 1,
                set.id.unwrap(),
                set.kind,
                set.kilograms
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or("unknown".into()),
                set.load_description.as_deref().unwrap_or("")
            ));
            for p in &set.portions {
                let exercise = self.store.show(p.exercise_id)?;
                let quantity = p
                    .repetitions
                    .as_ref()
                    .or(p.seconds.as_ref())
                    .or(p.metres.as_ref())
                    .unwrap();
                lines.push(format!(
                    "  {}: {} {} ({})",
                    exercise.name,
                    quantity,
                    match exercise.measurement {
                        crate::exercise_types::Measurement::Repetitions => "attempted repetitions",
                        crate::exercise_types::Measurement::Duration => "seconds",
                        crate::exercise_types::Measurement::Distance => "metres",
                    },
                    exercise.load_convention
                ));
                if let Some(notes) = &p.notes {
                    lines.push(format!("  Portion notes: {notes}"));
                }
            }
            lines.push(format!(
                "  RPE {} | flags white {:?}, red {:?}",
                set.rpe
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or("unknown".into()),
                set.white_flags,
                set.red_flags
            ));
            if let Some(notes) = &set.notes {
                lines.push(format!("  Set notes: {notes}"));
            }
            if let Some(rest) = w.rest.as_ref().and_then(|r| r.get(i)) {
                lines.push(format!(
                    "  Rest after: {} seconds",
                    rest.0
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or("unknown".into())
                ));
            }
        }
        for group in &w.supersets {
            lines.push(format!(
                "Actual superset {:?}: {:?}",
                group.id, group.set_ids
            ));
        }
        Ok(lines)
    }
    fn detail_lines(&self) -> Result<Vec<String>> {
        let Some(w) = &self.workout else {
            return Ok(vec![]);
        };
        if !self.intention {
            return self.activity_lines(w);
        }
        let document = if self.intention {
            serde_json::to_value(&w.intention)?
        } else {
            serde_json::to_value(w)?
        };
        let mut lines = vec![
            format!("Date: {} Start: {:?} End: {:?}", w.date, w.start, w.end),
            format!(
                "Source: {:?} original: {:?} {:?}",
                w.source_routine_id, w.original_source_id, w.original_source_name
            ),
        ];
        // Full supported structures are scrollable; no target/actual correspondence.
        lines.extend(
            serde_json::to_string_pretty(&document)?
                .lines()
                .map(str::to_owned),
        );
        let sets: Vec<i64> = if self.intention {
            w.intention
                .sets
                .iter()
                .flat_map(|s| s.portions.iter().map(|p| p.exercise_id))
                .collect()
        } else {
            w.sets
                .iter()
                .flat_map(|s| s.portions.iter().map(|p| p.exercise_id))
                .collect()
        };
        for id in sets {
            let e = self.store.show(id)?;
            lines.push(format!(
                "Exercise {id}: {} ({}, {})",
                e.name, e.measurement, e.load_convention
            ));
        }
        Ok(lines)
    }
    pub fn render(&self, width: usize, height: usize) -> String {
        let width = width.max(1);
        self.width.set(width);
        let height = height.max(1);
        let mut lines = vec![
            format!("Precision | Database: {}", self.path.display()),
            format!(
                "Draft ID: {} | {}",
                self.workout
                    .as_ref()
                    .and_then(|w| w.id)
                    .map_or("none".into(), |id| id.to_string()),
                if self.form.is_some() || self.screen == Screen::Picker {
                    "Unsaved edit; saved activity retained"
                } else {
                    "Saved activity; startup fields unsaved"
                }
            ),
        ];
        let content = match self.screen {
            Screen::Home => {
                let mut v = vec!["Start a draft (optional time stays unknown when blank)".into()];
                for (i, (name, value)) in [
                    ("Date", &self.date),
                    ("Start RFC3339", &self.start),
                    ("Routine ID (blank = none)", &self.routine),
                ]
                .iter()
                .enumerate()
                {
                    v.push(format!(
                        "{} {name}: {value}",
                        if i == self.field { ">" } else { " " }
                    ));
                }
                match self.store.routines() {
                    Ok(r) => {
                        v.push("Available routines (Up/Down scroll; type ID above):".into());
                        let capacity = height.saturating_sub(11).max(1);
                        let offset = self.selected.saturating_sub(capacity - 1);
                        v.extend(
                            r.iter()
                                .skip(offset)
                                .map(|r| format!("{}: {}", r.id.unwrap(), r.name)),
                        );
                    }
                    Err(e) => v.push(e.to_string()),
                };
                v
            }
            Screen::Drafts | Screen::Reuse => {
                let mut v = vec![if self.screen == Screen::Drafts {
                    "Select a draft explicitly to resume".into()
                } else {
                    "Reuse current source; enter date/time on startup screen".into()
                }];
                let capacity = height.saturating_sub(8).max(1);
                let offset = self.selected.saturating_sub(capacity - 1);
                v.extend(self.entries.iter().enumerate().skip(offset).map(|(i, w)| {
                    format!(
                        "{} ID {} {} {} source {:?}",
                        if i == self.selected { ">" } else { " " },
                        w.id.unwrap(),
                        w.date,
                        w.state,
                        w.source_routine_id
                    )
                }));
                v
            }
            Screen::Help => vec![
                "F2 Start | F3 Resume draft | F4 Reuse current source".into(),
                "F5 Add: search name, arrows choose, Enter opens form".into(),
                "Enter edits actual set; Up/Down selects; PgUp/PgDown scrolls details".into(),
                "F6 Duplicate: quantities, observations and notes start blank".into(),
                "Tab switches actual/intention; F7 records prescribed structure".into(),
                "Targets remain separate; actual repetitions count attempts".into(),
                "Forms: Tab/Up/Down fields; Enter/F12 confirm and save".into(),
                "Blank optional values stay unknown; explicit zero stays zero".into(),
                "F8 Finish: confirm activity becomes read-only; optional end time".into(),
                "F9 Discard: separately confirm permanent draft removal".into(),
                "Errors retain input: Enter/F12 retry, Esc cancels current edit".into(),
                "F11 inspects latest saved activity; Esc returns to retained edit".into(),
                "Shift-F11 explicitly abandons edit and reopens latest activity".into(),
                "F10/Ctrl-C quit: confirmed saves survive; transient input is lost".into(),
                "Esc closes help; ordinary letters always enter text".into(),
            ],
            Screen::Latest => {
                let mut v = vec![
                    "Latest saved activity (read-only); rejected input remains retained".into(),
                    "Esc returns to edit; Shift-F11 explicitly abandons edit and reopens".into(),
                ];
                match self.activity_lines(self.latest.as_ref().unwrap()) {
                    Ok(lines) => v.extend(wrap_lines(lines, width).into_iter().skip(self.scroll)),
                    Err(e) => v.push(e.to_string()),
                }
                v
            }
            Screen::Confirm => {
                if self.discarding {
                    vec![
                        "Discard permanently removes this draft and all its saved activity.".into(),
                        "Enter confirms permanent removal; Esc cancels.".into(),
                    ]
                } else {
                    vec![
                        "Finish makes saved activity read-only. Routines remain independent."
                            .into(),
                        format!("End RFC3339 (blank retains saved end): {}", self.end),
                        "Enter confirms finish; Esc cancels.".into(),
                    ]
                }
            }
            Screen::Picker => {
                let mut v = vec![
                    format!("Exercise search: {}", self.query),
                    "Type name; arrows choose, Enter opens form, Esc cancels".into(),
                ];
                let equipment = self.store.catalog("equipment").unwrap_or_default();
                let capacity = (height.saturating_sub(9) / 2).max(1);
                let offset = self.selected.saturating_sub(capacity - 1);
                for (i, e) in self
                    .exercises
                    .iter()
                    .enumerate()
                    .skip(offset)
                    .take(capacity)
                {
                    let names = equipment
                        .iter()
                        .filter(|(id, _)| e.equipment.contains(id))
                        .map(|(_, n)| n.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    v.push(format!(
                        "{} {}",
                        if i == self.selected { ">" } else { " " },
                        e.name
                    ));
                    v.push(format!(
                        "  {} | {} | {}",
                        names, e.measurement, e.load_convention
                    ));
                }
                v
            }
            Screen::Form => {
                let form = self.form.as_ref().unwrap();
                let mut v = vec![format!(
                    "Unsaved: {} ({}, {})",
                    form.exercise.name, form.exercise.measurement, form.exercise.load_convention
                )];
                if !form.target.is_empty() {
                    v.extend(wrap_lines(vec![form.target.clone()], width));
                }
                for (i, field) in Field::ALL.iter().enumerate() {
                    let label = form.label(*field);
                    let prefix = format!("{} {label}: ", if i == self.field { ">" } else { " " });
                    let value = input_tail(
                        form.value(*field),
                        width
                            .saturating_sub(unicode_width::UnicodeWidthStr::width(prefix.as_str())),
                    );
                    v.push(format!("{prefix}{value}"));
                }
                v.push(
                    "Enter/F12 save; Tab/Up/Down fields; Esc cancel; blank = unspecified".into(),
                );
                v
            }
            Screen::Workout => {
                let mut v = vec![if self.intention {
                    "Preserved intention (read-only): minimum successful reps / maximum RPE".into()
                } else {
                    format!(
                        "Actual activity: repetitions count attempts | selected set {}",
                        self.selected + 1
                    )
                }];
                match self.detail_lines() {
                    Ok(details) => {
                        let wrapped = wrap_lines(details, width);
                        v.extend(wrapped.into_iter().skip(self.scroll));
                    }
                    Err(e) => v.push(e.to_string()),
                };
                v
            }
        };
        lines.extend(content.into_iter().take(height.saturating_sub(6)));
        while lines.len() < height.saturating_sub(4) {
            lines.push(String::new());
        }
        lines.push(if self.error.is_empty() {
            "Saved drafts survive quit. Enter/F12 retry; Esc cancels current edit.".into()
        } else {
            format!("Error: {}", self.error)
        });
        lines.push("F1 Help | F2 Start | F3 Drafts | F4 Reuse | F10 Quit".into());
        lines.push("F5 Add | F6 Duplicate | F7 Record | F8 Finish | F9 Discard | F12 Save".into());
        lines.push(
            "F11 latest | Shift-F11 abandon/reopen | Tab field/view | Enter | Esc cancel".into(),
        );
        lines
            .into_iter()
            .take(height)
            .map(|l| fit_line(&l, width))
            .collect::<Vec<_>>()
            .join("\r\n")
    }
}
fn input_tail(value: &str, width: usize) -> String {
    let mut columns = 0;
    let mut tail = Vec::new();
    for c in value.chars().rev() {
        columns += c.width().unwrap_or(0);
        if columns > width {
            break;
        }
        tail.push(c);
    }
    tail.into_iter().rev().collect()
}
fn fit_line(line: &str, width: usize) -> String {
    let mut columns = 0;
    line.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take_while(|c| {
            columns += c.width().unwrap_or(0);
            columns <= width
        })
        .collect()
}
fn wrap_lines(lines: Vec<String>, width: usize) -> Vec<String> {
    let mut wrapped = Vec::new();
    for line in lines {
        let mut current = String::new();
        let mut columns = 0;
        for c in line.chars().map(|c| if c.is_control() { ' ' } else { c }) {
            let size = c.width().unwrap_or(0);
            if columns + size > width && !current.is_empty() {
                wrapped.push(std::mem::take(&mut current));
                columns = 0;
            }
            current.push(c);
            columns += size;
        }
        wrapped.push(current);
    }
    wrapped
}
struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), cursor::Show, LeaveAlternateScreen);
    }
}
pub fn run(path: &Path) -> Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(
            "TUI requires interactive stdin and stdout terminals; use CLI commands instead".into(),
        );
    }
    let mut session = Session::open(path, chrono::Local::now().format("%Y-%m-%d").to_string())?;
    terminal::enable_raw_mode()?;
    let _guard = TerminalGuard;
    execute!(io::stdout(), EnterAlternateScreen, cursor::Hide)?;
    while !session.quit {
        let (w, h) = terminal::size()?;
        execute!(
            io::stdout(),
            cursor::MoveTo(0, 0),
            terminal::Clear(terminal::ClearType::All)
        )?;
        write!(io::stdout(), "{}", session.render(w as usize, h as usize))?;
        io::stdout().flush()?;
        if let Event::Key(k) = event::read()? {
            if k.kind == KeyEventKind::Release {
                continue;
            }
            let action = match k.code {
                KeyCode::F(1) => Action::Help,
                KeyCode::F(2) => Action::Start,
                KeyCode::F(3) => Action::Drafts,
                KeyCode::F(4) => Action::Reuse,
                KeyCode::F(5) => Action::Add,
                KeyCode::F(6) => Action::Duplicate,
                KeyCode::F(7) => Action::Record,
                KeyCode::F(8) => Action::Finish,
                KeyCode::F(9) => Action::Discard,
                KeyCode::F(11) if k.modifiers.contains(event::KeyModifiers::SHIFT) => {
                    Action::Reopen
                }
                KeyCode::F(11) => Action::InspectLatest,
                KeyCode::F(12) => Action::Save,
                KeyCode::F(10) => Action::Quit,
                KeyCode::Esc => Action::Escape,
                KeyCode::Tab => Action::Tab,
                KeyCode::Enter => Action::Select,
                KeyCode::Up => Action::Up,
                KeyCode::Down => Action::Down,
                KeyCode::PageUp => Action::PageUp,
                KeyCode::PageDown => Action::PageDown,
                KeyCode::Backspace => Action::Backspace,
                KeyCode::Char('c') if k.modifiers.contains(event::KeyModifiers::CONTROL) => {
                    Action::Quit
                }
                KeyCode::Char(c) => Action::Text(c),
                _ => continue,
            };
            let _ = session.act(action);
        }
    }
    Ok(())
}
