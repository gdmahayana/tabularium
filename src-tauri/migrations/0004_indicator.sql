-- SPDX-License-Identifier: AGPL-3.0-only
-- 0004: Indicator Registry, hasil indikator, dan penandaan usang (Tahap 3)

CREATE TABLE indicator (
    id               INTEGER PRIMARY KEY,
    code             TEXT NOT NULL UNIQUE,
    name             TEXT NOT NULL,
    definition       TEXT,
    interpretation   TEXT,
    unit             TEXT,
    formula          TEXT NOT NULL,
    aggregation_rule TEXT NOT NULL DEFAULT 'sum',
    method_version   INTEGER NOT NULL DEFAULT 1,
    archived_at      TEXT,
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL
) STRICT;

CREATE TABLE indicator_variable (
    indicator_id INTEGER NOT NULL REFERENCES indicator(id),
    variable_id  INTEGER NOT NULL REFERENCES variable(id),
    alias        TEXT NOT NULL,
    PRIMARY KEY (indicator_id, alias)
) STRICT;
CREATE INDEX ix_indvar_variable ON indicator_variable(variable_id);

CREATE TABLE indicator_result (
    id             INTEGER PRIMARY KEY,
    indicator_id   INTEGER NOT NULL REFERENCES indicator(id),
    method_version INTEGER NOT NULL,
    region_id      INTEGER NOT NULL REFERENCES ref_unit(id),
    period_value   TEXT NOT NULL,
    value_num      REAL,
    value_status   TEXT NOT NULL
                   CHECK (value_status IN ('terisi','kosong','tidak_berlaku')),
    is_stale       INTEGER NOT NULL DEFAULT 0 CHECK (is_stale IN (0,1)),
    computed_at    TEXT NOT NULL,
    CHECK (value_status = 'terisi' OR value_num IS NULL),
    CHECK (value_status <> 'terisi' OR value_num IS NOT NULL),
    UNIQUE (indicator_id, region_id, period_value)
) STRICT;
CREATE INDEX ix_indres_period ON indicator_result(indicator_id, period_value);
CREATE INDEX ix_indres_stale  ON indicator_result(is_stale) WHERE is_stale = 1;

CREATE TRIGGER trg_obs_stale_ins AFTER INSERT ON observation
BEGIN
    UPDATE indicator_result SET is_stale = 1
    WHERE period_value = NEW.period_value
      AND indicator_id IN (SELECT indicator_id FROM indicator_variable
                           WHERE variable_id = NEW.variable_id);
END;

CREATE TRIGGER trg_obs_stale_upd AFTER UPDATE ON observation
BEGIN
    UPDATE indicator_result SET is_stale = 1
    WHERE period_value = NEW.period_value
      AND indicator_id IN (SELECT indicator_id FROM indicator_variable
                           WHERE variable_id = NEW.variable_id);
    UPDATE indicator_result SET is_stale = 1
    WHERE period_value = OLD.period_value
      AND indicator_id IN (SELECT indicator_id FROM indicator_variable
                           WHERE variable_id = OLD.variable_id);
END;

CREATE TRIGGER trg_obs_stale_del AFTER DELETE ON observation
BEGIN
    UPDATE indicator_result SET is_stale = 1
    WHERE period_value = OLD.period_value
      AND indicator_id IN (SELECT indicator_id FROM indicator_variable
                           WHERE variable_id = OLD.variable_id);
END;