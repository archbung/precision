CREATE TABLE routines (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    notes TEXT
);
CREATE TABLE prescribed_sets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    routine_id INTEGER NOT NULL REFERENCES routines(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK(position >= 0),
    type TEXT NOT NULL CHECK(type IN ('warmup','main')),
    kilograms TEXT,
    rpe_half INTEGER CHECK(rpe_half BETWEEN 2 AND 20),
    load_description TEXT,
    notes TEXT,
    UNIQUE(routine_id, position)
);
CREATE TABLE prescribed_portions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    set_id INTEGER NOT NULL REFERENCES prescribed_sets(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK(position >= 0),
    exercise_id INTEGER NOT NULL REFERENCES exercises(id),
    repetitions TEXT,
    seconds TEXT,
    metres TEXT,
    notes TEXT,
    UNIQUE(set_id, position)
);
