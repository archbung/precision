//! Keyboard adapter and headless draft session. Storage remains authoritative.
use crate::{exercises::Store, workouts::Workout};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
};
use unicode_width::UnicodeWidthChar;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub enum Action {
    Start,
    Drafts,
    Reuse,
    Select,
    Up,
    Down,
    Tab,
    Escape,
    Quit,
    Text(char),
    Backspace,
}
#[derive(PartialEq)]
enum Screen {
    Home,
    Drafts,
    Reuse,
    Workout,
}
pub struct Session {
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
        self.error.clear();
        let result = self.apply(action);
        if let Err(e) = &result {
            self.error = e.to_string();
        }
        result
    }
    fn apply(&mut self, action: Action) -> Result<()> {
        match action {
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
                }
                Screen::Workout => {}
            },
            Action::Tab => {
                if self.screen == Screen::Workout {
                    self.intention = !self.intention;
                    self.scroll = 0;
                } else if self.screen == Screen::Home {
                    self.field = (self.field + 1) % 3;
                }
            }
            Action::Up => {
                if self.screen == Screen::Workout {
                    self.scroll = self.scroll.saturating_sub(1);
                } else {
                    self.selected = self.selected.saturating_sub(1);
                }
            }
            Action::Down => {
                if self.screen == Screen::Workout {
                    self.scroll = (self.scroll + 1).min(
                        wrap_lines(self.detail_lines()?, self.width.get())
                            .len()
                            .saturating_sub(1),
                    );
                } else if self.screen == Screen::Home {
                    self.selected =
                        (self.selected + 1).min(self.store.routines()?.len().saturating_sub(1));
                } else {
                    self.selected = (self.selected + 1).min(self.entries.len().saturating_sub(1));
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
    fn detail_lines(&self) -> Result<Vec<String>> {
        let Some(w) = &self.workout else {
            return Ok(vec![]);
        };
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
                "Draft ID: {} | Saved activity; startup fields unsaved",
                self.workout
                    .as_ref()
                    .and_then(|w| w.id)
                    .map_or("none".into(), |id| id.to_string())
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
                        let capacity = height.saturating_sub(10).max(1);
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
                let capacity = height.saturating_sub(7).max(1);
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
            Screen::Workout => {
                let mut v = vec![if self.intention {
                    "Preserved intention (read-only): minimum successful reps / maximum RPE".into()
                } else {
                    "Actual activity (read-only): repetitions count attempts".into()
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
        lines.extend(content.into_iter().take(height.saturating_sub(5)));
        while lines.len() < height.saturating_sub(3) {
            lines.push(String::new());
        }
        lines.push(if self.error.is_empty() {
            "Saved drafts survive quit. Esc returns; input retained.".into()
        } else {
            format!("Error: {}", self.error)
        });
        lines.push("F1 Help | F2 Start | F3 Drafts | F4 Reuse | F10 Quit".into());
        lines.push("Tab field/actual/intention | Enter select | Up/Down scroll | Esc back".into());
        lines
            .into_iter()
            .take(height)
            .map(|l| fit_line(&l, width))
            .collect::<Vec<_>>()
            .join("\r\n")
    }
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
                KeyCode::F(1) => {
                    session.error = "Tab changes fields/views; F2 saves a new draft; F3 resumes; F4 reuses current source; F10 retains draft and quits. Up/Down scroll all details.".into();
                    continue;
                }
                KeyCode::F(2) => Action::Start,
                KeyCode::F(3) => Action::Drafts,
                KeyCode::F(4) => Action::Reuse,
                KeyCode::F(10) => Action::Quit,
                KeyCode::Esc => Action::Escape,
                KeyCode::Tab => Action::Tab,
                KeyCode::Enter => Action::Select,
                KeyCode::Up => Action::Up,
                KeyCode::Down => Action::Down,
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
