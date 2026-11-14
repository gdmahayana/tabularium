-- SPDX-License-Identifier: AGPL-3.0-only
-- 0006: Lembar Input SDI dan Input & Submission Registry (Tahap 3 irisan 6-7)

-- Setiap sel pada Lembar Input yang sudah ditandai pengguna.
CREATE TABLE sdi_cell_status (
    id              INTEGER PRIMARY KEY,
    profile_id      TEXT NOT NULL,
    period_value    TEXT NOT NULL,
    -- path kolom portal
    bidang          TEXT NOT NULL,
    sub_bidang      TEXT NOT NULL,
    kategori        TEXT NOT NULL,
    judul_kolom     TEXT NOT NULL,
    -- status penandaan oleh pengguna
    sudah_diinput   INTEGER NOT NULL DEFAULT 0 CHECK (sudah_diinput IN (0,1)),
    input_at        TEXT,
    input_oleh      TEXT,
    catatan         TEXT,
    -- deteksi perubahan: nilai sumber saat ditandai
    nilai_saat_input    REAL,
    nilai_terkini_hash  TEXT,   -- diisi saat nilai sumber berubah setelah ditandai
    perlu_diperbarui    INTEGER NOT NULL DEFAULT 0 CHECK (perlu_diperbarui IN (0,1)),
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    UNIQUE (profile_id, period_value, bidang, sub_bidang, kategori, judul_kolom)
) STRICT;
CREATE INDEX ix_sdi_cell_profile ON sdi_cell_status(profile_id, period_value);
CREATE INDEX ix_sdi_cell_perlu   ON sdi_cell_status(perlu_diperbarui) WHERE perlu_diperbarui = 1;

-- Riwayat pengajuan laporan ke portal (tingkat laporan, bukan sel).
CREATE TABLE sdi_submission (
    id              INTEGER PRIMARY KEY,
    profile_id      TEXT NOT NULL,
    period_value    TEXT NOT NULL,
    -- status sesuai dokumen induk bagian 7.10
    status          TEXT NOT NULL DEFAULT 'disiapkan'
                    CHECK (status IN ('disiapkan','diinput_draft','diajukan','diterima')),
    catatan         TEXT,
    bukti_ref       TEXT,   -- nomor tanda terima atau path tangkapan layar
    operator        TEXT NOT NULL DEFAULT 'pengguna',
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE INDEX ix_sdi_sub_profile ON sdi_submission(profile_id, period_value);