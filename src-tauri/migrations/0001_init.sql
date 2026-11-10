-- 0001_init.sql — Skema dasar TABULARIUM (Tahap 1)
--
-- Aturan file ini:
--   * Jangan diubah setelah dirilis. Perubahan skema = file migrasi baru (0003, ...).
--     Checksum tiap migrasi dicatat; file yang diubah diam-diam akan ditolak.
--   * PRAGMA foreign_keys diatur oleh pembuka koneksi (Rust), bukan di sini,
--     karena PRAGMA tersebut tidak berlaku di dalam transaksi.
--   * Tabel `migration` dibuat oleh runner migrasi, bukan oleh file ini.
--   * Waktu disimpan sebagai teks ISO 8601 UTC, mis. 2026-11-10T08:30:00.000Z.

-- ============================================================
-- SISTEM
-- ============================================================

CREATE TABLE app_config (
    key        TEXT PRIMARY KEY NOT NULL,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;

-- ============================================================
-- REFERENSI DAN MASTER DATA
-- ============================================================

CREATE TABLE ref_period_type (
    code       TEXT PRIMARY KEY NOT NULL,   -- kode teknis
    name       TEXT NOT NULL,               -- nama tampilan (Indonesia)
    sort_order INTEGER NOT NULL
) STRICT;

INSERT INTO ref_period_type (code, name, sort_order) VALUES
    ('tahunan',      'Tahunan',        1),
    ('semesteran',   'Semesteran',     2),
    ('triwulanan',   'Triwulanan',     3),
    ('bulanan',      'Bulanan',        4),
    ('tanggal',      'Tanggal',        5),
    ('tanpa_periode','Tanpa periode',  6);

-- Wilayah hierarkis. Satu baris = satu objek wilayah dengan ID stabil.
-- valid_from/valid_to menandai masa berlaku (pemekaran, penggabungan, perubahan).
-- Riwayat nama/kode yang lebih rinci menyusul di Tahap 7 bila diperlukan.
CREATE TABLE ref_unit (
    id         INTEGER PRIMARY KEY,
    code       TEXT,                         -- kode resmi (boleh kosong untuk data dummy)
    name       TEXT NOT NULL,
    level      TEXT NOT NULL CHECK (level IN ('provinsi','kabupaten','kecamatan','desa')),
    parent_id  INTEGER REFERENCES ref_unit(id),
    valid_from TEXT,
    valid_to   TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    CHECK (valid_to IS NULL OR valid_from IS NULL OR valid_to >= valid_from),
    CHECK (parent_id IS NULL OR parent_id <> id)
) STRICT;

CREATE INDEX ix_ref_unit_parent ON ref_unit(parent_id);
CREATE INDEX ix_ref_unit_level  ON ref_unit(level);
-- Kode unik per masa berlaku (kode yang sama boleh dipakai ulang di masa berbeda).
CREATE UNIQUE INDEX ux_ref_unit_code
    ON ref_unit(code, COALESCE(valid_from, ''))
    WHERE code IS NOT NULL;

-- ============================================================
-- BATCH (induk dari setiap perubahan massal / impor / koreksi)
-- ============================================================

CREATE TABLE batch (
    id         INTEGER PRIMARY KEY,
    source     TEXT NOT NULL CHECK (source IN ('manual','impor','koreksi','sistem')),
    note       TEXT,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    undone_at  TEXT                          -- diisi saat batch dibatalkan (Tahap 2)
) STRICT;

-- ============================================================
-- KATALOG
-- ============================================================

CREATE TABLE dataset (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    domain      TEXT,
    grain_desc  TEXT,                        -- apa yang diwakili satu baris
    period_type TEXT NOT NULL REFERENCES ref_period_type(code),
    owner       TEXT,
    source      TEXT,
    status      TEXT NOT NULL DEFAULT 'draft'
                CHECK (status IN ('draft','diperiksa','terverifikasi','dipublikasikan')),
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;

CREATE TABLE variable (
    id          INTEGER PRIMARY KEY,
    dataset_id  INTEGER NOT NULL REFERENCES dataset(id),
    name        TEXT NOT NULL,
    code        TEXT NOT NULL,               -- kode teknis, stabil
    concept     TEXT,
    definition  TEXT,
    data_type   TEXT NOT NULL
                CHECK (data_type IN ('integer','decimal','text','date','boolean','category')),
    unit        TEXT,                        -- satuan (kelompok, orang, rupiah, ...)
    is_required INTEGER NOT NULL DEFAULT 0 CHECK (is_required IN (0,1)),
    sort_order  INTEGER NOT NULL DEFAULT 0,
    archived_at TEXT,                        -- variabel berobservasi diarsipkan, bukan dihapus
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    UNIQUE (dataset_id, code),
    UNIQUE (id, dataset_id)                  -- target FK gabungan dari observation
) STRICT;

CREATE INDEX ix_variable_dataset ON variable(dataset_id, sort_order);

CREATE TABLE label (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;

-- Label awal (bagian 5.2 dokumen induk).
INSERT INTO label (name) VALUES
    ('Internal'), ('Statistik Sektoral'), ('Kandidat SDI'), ('Bahan Perencanaan'),
    ('Bahan Evaluasi'), ('Publikasi'), ('Referensi Wilayah'), ('Research'),
    ('Operational'), ('Reporting');

CREATE TABLE dataset_label (
    dataset_id INTEGER NOT NULL REFERENCES dataset(id) ON DELETE CASCADE,
    label_id   INTEGER NOT NULL REFERENCES label(id),
    PRIMARY KEY (dataset_id, label_id)
) STRICT;

CREATE TABLE variable_label (
    variable_id INTEGER NOT NULL REFERENCES variable(id) ON DELETE CASCADE,
    label_id    INTEGER NOT NULL REFERENCES label(id),
    PRIMARY KEY (variable_id, label_id)
) STRICT;
-- indicator_label ditambahkan bersama tabel indicator (Tahap 3).

-- Dimensi disagregasi (jenis kelamin, kelompok umur, ...).
-- Baris id = 0 adalah penanda "tanpa dimensi" agar kolom pada observation
-- bisa NOT NULL (NULL membuat UNIQUE di SQLite tidak mencegah duplikat).
CREATE TABLE dimension (
    id          INTEGER PRIMARY KEY,
    code        TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    description TEXT
) STRICT;

CREATE TABLE dimension_value (
    id           INTEGER PRIMARY KEY,
    dimension_id INTEGER NOT NULL REFERENCES dimension(id),
    code         TEXT NOT NULL,
    name         TEXT NOT NULL,
    sort_order   INTEGER NOT NULL DEFAULT 0,
    UNIQUE (dimension_id, code)
) STRICT;

INSERT INTO dimension (id, code, name)            VALUES (0, '_none', 'Tanpa dimensi');
INSERT INTO dimension_value (id, dimension_id, code, name) VALUES (0, 0, '_none', 'Tanpa dimensi');

-- ============================================================
-- DATA: OBSERVASI (format panjang, keputusan K-08)
-- ============================================================

CREATE TABLE observation (
    id                 INTEGER PRIMARY KEY,
    dataset_id         INTEGER NOT NULL,
    variable_id        INTEGER NOT NULL,
    unit_id            INTEGER NOT NULL REFERENCES ref_unit(id),
    -- Periode referensi: '2026', '2026-S1', '2026-Q1', '2026-03', '2026-03-15'.
    -- Dataset tanpa periode memakai teks kosong ''.
    period_value       TEXT NOT NULL DEFAULT '',
    dimension_value_id INTEGER NOT NULL DEFAULT 0 REFERENCES dimension_value(id),
    -- Bagian 4.7: kosong, tidak berlaku, tidak diketahui, dan nol sebenarnya dibedakan.
    -- Nol sebenarnya = value_status 'terisi' dengan value_num = 0.
    value_status       TEXT NOT NULL DEFAULT 'terisi'
                       CHECK (value_status IN ('terisi','kosong','tidak_berlaku','tidak_diketahui')),
    value_num          REAL,
    value_text         TEXT,
    value_date         TEXT,
    batch_id           INTEGER REFERENCES batch(id),
    created_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    -- Variabel harus milik dataset yang sama.
    FOREIGN KEY (variable_id, dataset_id) REFERENCES variable(id, dataset_id),
    FOREIGN KEY (dataset_id) REFERENCES dataset(id),
    -- Status selain 'terisi' tidak boleh membawa nilai.
    CHECK (value_status = 'terisi'
           OR (value_num IS NULL AND value_text IS NULL AND value_date IS NULL)),
    -- Status 'terisi' harus membawa tepat satu jenis nilai.
    CHECK (value_status <> 'terisi'
           OR ((value_num IS NOT NULL) + (value_text IS NOT NULL) + (value_date IS NOT NULL)) = 1),
    UNIQUE (dataset_id, variable_id, unit_id, period_value, dimension_value_id)
) STRICT;

CREATE INDEX ix_obs_dataset_period ON observation(dataset_id, period_value);
CREATE INDEX ix_obs_variable       ON observation(variable_id);
CREATE INDEX ix_obs_unit           ON observation(unit_id);
CREATE INDEX ix_obs_batch          ON observation(batch_id);

-- ============================================================
-- AUDIT (generik; append-only dijaga oleh trigger di 0002)
-- ============================================================

CREATE TABLE audit_log (
    id          INTEGER PRIMARY KEY,
    entity_type TEXT NOT NULL,               -- 'dataset','variable','observation','ref_unit','label',...
    entity_id   INTEGER NOT NULL,
    dataset_id  INTEGER,                     -- denormalisasi untuk riwayat per dataset (tanpa FK:
                                             -- catatan audit harus tetap ada walau dataset dihapus)
    action      TEXT NOT NULL CHECK (action IN ('create','update','delete','archive')),
    field       TEXT,                        -- nama kolom yang berubah; NULL untuk create/delete utuh
    old_value   TEXT,
    new_value   TEXT,
    reason      TEXT,
    batch_id    INTEGER REFERENCES batch(id),
    changed_by  TEXT NOT NULL,
    changed_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;

CREATE INDEX ix_audit_entity  ON audit_log(entity_type, entity_id);
CREATE INDEX ix_audit_dataset ON audit_log(dataset_id, changed_at);
CREATE INDEX ix_audit_batch   ON audit_log(batch_id);