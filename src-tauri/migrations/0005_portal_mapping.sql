-- SPDX-License-Identifier: AGPL-3.0-only
-- 0005: Pemetaan variabel/indikator ke kolom profil portal (Tahap 3 irisan 5)

CREATE TABLE portal_mapping (
    id           INTEGER PRIMARY KEY,
    profile_id   TEXT NOT NULL,
    -- path dalam hierarki portal
    bidang       TEXT NOT NULL,
    sub_bidang   TEXT NOT NULL,
    kategori     TEXT NOT NULL,
    judul_kolom  TEXT NOT NULL,
    satuan_unit  TEXT NOT NULL DEFAULT '',
    -- sumber: 'variable' atau 'indicator'
    source_type  TEXT NOT NULL CHECK (source_type IN ('variable','indicator')),
    -- code dari variable atau indicator (bukan id, agar portabel)
    source_ref   TEXT NOT NULL,
    -- aturan agregasi: 'sum_all_villages', 'ratio_of_sums', atau lainnya
    aggregation  TEXT NOT NULL DEFAULT 'sum_all_villages',
    -- konversi satuan (opsional, misal 'x1000' untuk ribuan)
    unit_factor  REAL NOT NULL DEFAULT 1.0,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    UNIQUE (profile_id, bidang, sub_bidang, kategori, judul_kolom)
) STRICT;
CREATE INDEX ix_mapping_profile ON portal_mapping(profile_id);
CREATE INDEX ix_mapping_source  ON portal_mapping(source_type, source_ref);