// SPDX-License-Identifier: AGPL-3.0-only
//! Mesin validasi P0: completeness, validity, uniqueness, referential.
//! Berjalan atas staging rows sebelum commit.

use rusqlite::{params, Connection};
use serde_json::Value;

pub struct HasilValidasi {
    pub result:  String,  // lolos | peringatan | gagal
    pub issues:  Vec<Isu>,
    pub run_id:  i64,
}

pub struct Isu {
    pub row_no:   Option<i64>,
    pub unit_ref: Option<String>,
    pub severity: String,
    pub message:  String,
}

pub fn jalankan_validasi(
    conn: &Connection,
    session_id: i64,
    dataset_id: i64,
    col_variable_map: &[(String, i64, String)], // (kolom, variable_id, tipe_data)
    col_unit_wilayah: &str,
    period_value: &str,
) -> Result<HasilValidasi, String> {
    let rows = ambil_staging(conn, session_id)?;
    let mut issues: Vec<Isu> = Vec::new();

    for (i, row) in rows.iter().enumerate() {
        let row_no = i as i64;
        let unit_ref = row[col_unit_wilayah].as_str().map(|s| s.to_string());

        // 1. COMPLETENESS: kolom wajib tidak boleh kosong
        for (col, _var_id, _tipe) in col_variable_map {
            let val = &row[col];
            if val.is_null() || val.as_str().map(|s| s.trim().is_empty()).unwrap_or(false) {
                issues.push(Isu {
                    row_no: Some(row_no),
                    unit_ref: unit_ref.clone(),
                    severity: "warning".into(),
                    message: format!(
                        "Baris {}: kolom '{}' kosong. Periksa apakah nilai memang tidak tersedia.",
                        row_no + 1, col
                    ),
                });
            }
        }

        // 2. VALIDITY: nilai numerik harus bisa diparse sebagai angka
        for (col, _var_id, tipe) in col_variable_map {
            let val = &row[col];
            if val.is_null() { continue; }
            if tipe == "integer" || tipe == "decimal" {
                if val.as_f64().is_none() {
                    if let Some(s) = val.as_str() {
                        if !s.trim().is_empty() {
                            issues.push(Isu {
                                row_no: Some(row_no),
                                unit_ref: unit_ref.clone(),
                                severity: "fail".into(),
                                message: format!(
                                    "Baris {}: kolom '{}' berisi '{}' yang bukan angka.",
                                    row_no + 1, col, s
                                ),
                            });
                        }
                    }
                }
                if let Some(n) = val.as_f64() {
                    if n < 0.0 {
                        issues.push(Isu {
                            row_no: Some(row_no),
                            unit_ref: unit_ref.clone(),
                            severity: "warning".into(),
                            message: format!(
                                "Baris {}: kolom '{}' bernilai negatif ({}). Periksa apakah ini benar.",
                                row_no + 1, col, n
                            ),
                        });
                    }
                }
            }
        }

        // 3. REFERENTIAL: nama wilayah harus cocok ke master
        let cocok: Option<i64> = conn.query_row(
            "SELECT unit_id FROM import_unit_match WHERE session_id=?1 AND source_name=?2",
            params![session_id, unit_ref.as_deref().unwrap_or("")],
            |r| r.get(0),
        ).ok().flatten();
        if cocok.is_none() {
            issues.push(Isu {
                row_no: Some(row_no),
                unit_ref: unit_ref.clone(),
                severity: "fail".into(),
                message: format!(
                    "Baris {}: wilayah '{}' tidak ditemukan di master. Periksa ejaan atau tambahkan ke master wilayah.",
                    row_no + 1, unit_ref.as_deref().unwrap_or("(kosong)")
                ),
            });
        }
    }

    // 4. UNIQUENESS: tidak boleh ada duplikat (unit + periode + variabel)
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (i, row) in rows.iter().enumerate() {
        let unit = row[col_unit_wilayah].as_str().unwrap_or("");
        for (col, var_id, _) in col_variable_map {
            let key = format!("{unit}|{period_value}|{var_id}");
            if seen.contains(&key) {
                issues.push(Isu {
                    row_no: Some(i as i64),
                    unit_ref: Some(unit.to_string()),
                    severity: "fail".into(),
                    message: format!(
                        "Baris {}: duplikat — wilayah '{}' periode '{}' kolom '{}' sudah ada di baris sebelumnya.",
                        i + 1, unit, period_value, col
                    ),
                });
            } else {
                seen.insert(key);
            }
        }
    }

    let result = if issues.iter().any(|x| x.severity == "fail") {
        "gagal"
    } else if !issues.is_empty() {
        "peringatan"
    } else {
        "lolos"
    }.to_string();

    let run_id = simpan_hasil(conn, dataset_id, Some(session_id), &result, &issues)?;
    Ok(HasilValidasi { result, issues, run_id })
}

fn ambil_staging(conn: &Connection, session_id: i64) -> Result<Vec<Value>, String> {
    let mut stmt = conn.prepare(
        "SELECT raw_json FROM import_staging_row WHERE session_id=?1 ORDER BY row_no"
    ).map_err(|e| format!("Gagal menyiapkan kueri staging: {e}"))?;
    let rows: Vec<Value> = stmt.query_map([session_id], |r| r.get::<_,String>(0))
        .map_err(|e| format!("Gagal membaca staging: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|s| serde_json::from_str(&s).ok())
        .collect();
    Ok(rows)
}

fn simpan_hasil(
    conn: &Connection,
    dataset_id: i64,
    session_id: Option<i64>,
    result: &str,
    issues: &[Isu],
) -> Result<i64, String> {
    conn.execute(
        "INSERT INTO validation_run (dataset_id, session_id, result, ran_at) VALUES (?1,?2,?3,strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
        params![dataset_id, session_id, result],
    ).map_err(|e| format!("Gagal menyimpan hasil validasi: {e}"))?;
    let run_id = conn.last_insert_rowid();
    for isu in issues {
        conn.execute(
            "INSERT INTO validation_issue (run_id, row_no, severity, message) VALUES (?1,?2,?3,?4)",
            params![run_id, isu.row_no, isu.severity, isu.message],
        ).map_err(|e| format!("Gagal menyimpan isu validasi: {e}"))?;
    }
    Ok(run_id)
}
