
# SPDX-License-Identifier: AGPL-3.0-only
"""
Spike A: validasi model observasi format panjang di SQLite.
Kriteria lulus:
1. Dua dataset berbeda struktur (PKK dan Aset Desa) termodelkan tanpa DDL dinamis
2. Agregasi (jumlah per kecamatan/tahun) selesai cepat
3. Audit per sel berjalan tanpa penalti besar
4. Query untuk Tahap 3 (indikator, lembar input) tidak rumit berlebihan
"""
import sqlite3, csv, time, pathlib

DB = pathlib.Path("tests/spike_a.sqlite")
DB.unlink(missing_ok=True)
con = sqlite3.connect(DB)
con.execute("PRAGMA foreign_keys = ON")
con.execute("PRAGMA journal_mode = WAL")

# ── Skema ──────────────────────────────────────────────────────────────────
con.executescript("""
CREATE TABLE dataset (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    grain_desc  TEXT
);

CREATE TABLE variable (
    id          TEXT PRIMARY KEY,
    dataset_id  TEXT NOT NULL REFERENCES dataset(id),
    name        TEXT NOT NULL,
    data_type   TEXT NOT NULL CHECK(data_type IN ('numeric','text','date')),
    unit        TEXT
);

CREATE TABLE ref_unit (
    id      TEXT PRIMARY KEY,
    name    TEXT NOT NULL,
    level   INTEGER NOT NULL,
    parent  TEXT REFERENCES ref_unit(id)
);

CREATE TABLE observation (
    id          INTEGER PRIMARY KEY,
    dataset_id  TEXT NOT NULL REFERENCES dataset(id),
    variable_id TEXT NOT NULL REFERENCES variable(id),
    unit_id     TEXT NOT NULL REFERENCES ref_unit(id),
    period      TEXT,
    dim1_key    TEXT,
    dim1_val    TEXT,
    value_num   REAL,
    value_text  TEXT,
    is_null     INTEGER NOT NULL DEFAULT 0,
    null_reason TEXT,
    created_at  TEXT NOT NULL DEFAULT(datetime('now')),
    batch_id    TEXT
);

CREATE INDEX idx_obs_dataset  ON observation(dataset_id);
CREATE INDEX idx_obs_variable ON observation(variable_id, period);
CREATE INDEX idx_obs_unit     ON observation(unit_id, period);

CREATE TABLE audit_log (
    id          INTEGER PRIMARY KEY,
    obs_id      INTEGER REFERENCES observation(id),
    changed_at  TEXT NOT NULL DEFAULT(datetime('now')),
    old_num     REAL,
    new_num     REAL,
    old_text    TEXT,
    new_text    TEXT,
    reason      TEXT,
    batch_id    TEXT
);
""")

# ── Master wilayah ─────────────────────────────────────────────────────────
wilayah = list(csv.DictReader(open("examples/wilayah_dummy.csv")))
kecamatan = {}
con.execute("INSERT INTO ref_unit VALUES(?,?,?,?)", ("KAB01","Kabupaten Contoh",1,None))
for w in wilayah:
    if w["kode_kecamatan"] not in kecamatan:
        kecamatan[w["kode_kecamatan"]] = w["kecamatan"]
        con.execute("INSERT INTO ref_unit VALUES(?,?,?,?)",
                    (w["kode_kecamatan"], w["kecamatan"], 2, "KAB01"))
for w in wilayah:
    con.execute("INSERT INTO ref_unit VALUES(?,?,?,?)",
                (w["kode_desa"], w["desa"], 3, w["kode_kecamatan"]))
con.commit()

# ── Dataset PKK ────────────────────────────────────────────────────────────
con.execute("INSERT INTO dataset VALUES(?,?,?)",
            ("pkk","Data Pembinaan PKK","1 desa x 1 tahun"))
for var_id, name, unit in [
    ("pkk_kelompok","jumlah_kelompok_pkk","kelompok"),
    ("pkk_binaan",  "jumlah_kelompok_binaan","kelompok"),
    ("pkk_kader",   "jumlah_kader","orang"),
]:
    con.execute("INSERT INTO variable VALUES(?,?,?,?,?)",
                (var_id,"pkk",name,"numeric",unit))

pkk_rows = list(csv.DictReader(open("examples/pkk_dummy_long.csv")))
batch = "batch_pkk_import_01"
obs = []
for row in pkk_rows:
    for var_id, col in [("pkk_kelompok","jumlah_kelompok_pkk"),
                        ("pkk_binaan",  "jumlah_kelompok_binaan"),
                        ("pkk_kader",   "jumlah_kader")]:
        val = row[col].strip() if row[col] else ""
        is_null = 1 if val == "" else 0
        obs.append(("pkk", var_id, row["kode_desa"], row["tahun"],
                    None, None,
                    float(val) if val else None, None,
                    is_null, "kosong dari sumber" if is_null else None,
                    batch))
con.executemany(
    "INSERT INTO observation(dataset_id,variable_id,unit_id,period,"
    "dim1_key,dim1_val,value_num,value_text,is_null,null_reason,batch_id) "
    "VALUES(?,?,?,?,?,?,?,?,?,?,?)", obs)
con.commit()
print(f"PKK: {len(obs)} observasi dimasukkan")

# ── Dataset Aset Desa (struktur berbeda: tanpa periode, ada dim kondisi) ────
con.execute("INSERT INTO dataset VALUES(?,?,?)",
            ("aset","Data Aset Desa","1 aset"))
for var_id, name, unit in [
    ("aset_jenis",  "jenis_aset",   None),
    ("aset_kondisi","kondisi",       None),
    ("aset_nilai",  "nilai_perolehan","rupiah"),
    ("aset_tahun",  "tahun_perolehan","tahun"),
]:
    con.execute("INSERT INTO variable VALUES(?,?,?,?,?)",
                (var_id,"aset",name,
                 "numeric" if var_id in ("aset_nilai","aset_tahun") else "text",
                 unit))
aset_rows = list(csv.DictReader(open("examples/aset_desa_dummy.csv")))
obs2 = []
for row in aset_rows:
    for var_id, col, is_num in [
        ("aset_jenis",  "jenis_aset",   False),
        ("aset_kondisi","kondisi",       False),
        ("aset_nilai",  "nilai_perolehan_rp", True),
        ("aset_tahun",  "tahun_perolehan",    True),
    ]:
        val = row[col].strip()
        obs2.append(("aset", var_id, row["kode_desa"], None,
                     "id_aset", row["id_aset"],
                     float(val) if is_num else None,
                     None if is_num else val,
                     0, None, "batch_aset_01"))
con.executemany(
    "INSERT INTO observation(dataset_id,variable_id,unit_id,period,"
    "dim1_key,dim1_val,value_num,value_text,is_null,null_reason,batch_id) "
    "VALUES(?,?,?,?,?,?,?,?,?,?,?)", obs2)
con.commit()
print(f"Aset: {len(obs2)} observasi dimasukkan")

# ── Query agregasi ─────────────────────────────────────────────────────────
print()
print("=== AGREGASI: Jumlah Kelompok PKK per Kecamatan per Tahun ===")
t0 = time.perf_counter()
rows = con.execute("""
    SELECT ru_kec.name AS kecamatan, o.period AS tahun,
           SUM(o.value_num) AS total_kelompok
    FROM observation o
    JOIN ref_unit ru_desa ON ru_desa.id = o.unit_id
    JOIN ref_unit ru_kec  ON ru_kec.id  = ru_desa.parent
    WHERE o.variable_id = 'pkk_kelompok' AND o.is_null = 0
    GROUP BY ru_kec.id, o.period
    ORDER BY ru_kec.name, o.period
""").fetchall()
t1 = time.perf_counter()
for r in rows: print(f"  {r[0]:<25} {r[1]}  →  {int(r[2])} kelompok")
print(f"  Waktu: {(t1-t0)*1000:.2f} ms")

print()
print("=== CEK ANOMALI: binaan > kelompok ===")
anom = con.execute("""
    SELECT o_bin.unit_id, o_bin.period, o_bin.value_num AS binaan, o_kel.value_num AS kelompok
    FROM observation o_bin
    JOIN observation o_kel ON o_kel.unit_id=o_bin.unit_id
        AND o_kel.period=o_bin.period AND o_kel.variable_id='pkk_kelompok'
    WHERE o_bin.variable_id='pkk_binaan' AND o_bin.value_num > o_kel.value_num
""").fetchall()
for a in anom: print(f"  ANOMALI desa {a[0]} tahun {a[1]}: binaan={a[2]} > kelompok={a[3]}")
if not anom: print("  Tidak ada anomali terdeteksi")

print()
print("=== SEL KOSONG ===")
nulls = con.execute("SELECT unit_id, period, variable_id FROM observation WHERE is_null=1").fetchall()
for n in nulls: print(f"  {n[0]} / {n[1]} / {n[2]}")

print()
total = con.execute("SELECT COUNT(*) FROM observation").fetchone()[0]
print(f"Total observasi di DB: {total}")
print(f"Database: {DB} ({DB.stat().st_size/1024:.1f} KB)")
print()
print("SPIKE A: LULUS" if rows and anom else "SPIKE A: perlu ditinjau")
con.close()
