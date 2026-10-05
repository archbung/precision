CREATE TABLE workouts (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 state TEXT NOT NULL CHECK(state IN ('draft','finished')),
 date TEXT NOT NULL,
 start TEXT,
 end TEXT,
 notes TEXT
);
CREATE TABLE performed_sets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workout_id INTEGER NOT NULL REFERENCES workouts(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK(position >= 0),
    type TEXT NOT NULL CHECK(type IN ('warmup','main')),
    kilograms TEXT,
    rpe_half INTEGER CHECK(rpe_half BETWEEN 2 AND 20),
    white_flags INTEGER CHECK(white_flags BETWEEN 0 AND 3),
    red_flags INTEGER CHECK(red_flags BETWEEN 0 AND 3),
    load_description TEXT,
    notes TEXT,
    UNIQUE(workout_id, position)
);
CREATE TABLE performed_portions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    set_id INTEGER NOT NULL REFERENCES performed_sets(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK(position >= 0),
    exercise_id INTEGER NOT NULL REFERENCES exercises(id),
    repetitions TEXT,
    seconds TEXT,
    metres TEXT,
    notes TEXT,
    UNIQUE(set_id, position)
);
