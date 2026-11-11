-- 0003_import_quality.sql
CREATE TABLE import_session (
    id INTEGER PRIMARY KEY,
    dataset_id INTEGER NOT NULL REFERENCES dataset(id),
    source_name TEXT NOT NULL,
    source_sheet TEXT,
    period_value TEXT NOT NULL DEFAULT "",
    status TEXT NOT NULL DEFAULT "staged"
           CHECK (status IN ("staged","mapped","previewed","committed","undone","failed")),
    batch_id INTEGER REFERENCES batch(id),
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL
) STRICT;
CREATE TABLE import_staging_row (
    id INTEGER PRIMARY KEY,
    session_id INTEGER NOT NULL REFERENCES import_session(id) ON DELETE CASCADE,
    row_no INTEGER NOT NULL,
    raw_json TEXT NOT NULL
) STRICT;
CREATE INDEX ix_staging_session ON import_staging_row(session_id, row_no);
CREATE TABLE import_unit_match (
    id INTEGER PRIMARY KEY,
    session_id INTEGER NOT NULL REFERENCES import_session(id) ON DELETE CASCADE,
    source_name TEXT NOT NULL,
    unit_id INTEGER REFERENCES ref_unit(id),
    match_kind TEXT NOT NULL CHECK (match_kind IN ("persis","normalisasi","saran","manual","tidak_cocok")),
    UNIQUE (session_id, source_name)
) STRICT;
CREATE TABLE column_mapping_template (
    id INTEGER PRIMARY KEY,
    dataset_id INTEGER NOT NULL REFERENCES dataset(id),
    name TEXT NOT NULL,
    mapping_json TEXT NOT NULL,
    created_at TEXT NOT NULL
) STRICT;
CREATE UNIQUE INDEX ix_mapping_template ON column_mapping_template(dataset_id, name);
CREATE TABLE validation_rule (
    id INTEGER PRIMARY KEY,
    dataset_id INTEGER REFERENCES dataset(id),
    kind TEXT NOT NULL CHECK (kind IN ("completeness","validity","uniqueness","referential")),
    variable_id INTEGER REFERENCES variable(id),
    params_json TEXT NOT NULL DEFAULT "{}",
    severity TEXT NOT NULL DEFAULT "warning" CHECK (severity IN ("warning","fail")),
    version INTEGER NOT NULL DEFAULT 1,
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1))
) STRICT;
CREATE TABLE validation_run (
    id INTEGER PRIMARY KEY,
    dataset_id INTEGER NOT NULL REFERENCES dataset(id),
    session_id INTEGER REFERENCES import_session(id),
    result TEXT NOT NULL CHECK (result IN ("lolos","peringatan","gagal")),
    ran_at TEXT NOT NULL
) STRICT;
CREATE TABLE validation_issue (
    id INTEGER PRIMARY KEY,
    run_id INTEGER NOT NULL REFERENCES validation_run(id) ON DELETE CASCADE,
    rule_id INTEGER REFERENCES validation_rule(id),
    row_no INTEGER,
    unit_id INTEGER REFERENCES ref_unit(id),
    severity TEXT NOT NULL CHECK (severity IN ("warning","fail")),
    message TEXT NOT NULL
) STRICT;
CREATE INDEX ix_issue_run ON validation_issue(run_id);
