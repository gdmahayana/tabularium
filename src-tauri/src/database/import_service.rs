// SPDX-License-Identifier: AGPL-3.0-only
//! Service impor — staging, commit, dan undo batch impor.

use rusqlite::{params, Connection};
use serde_json::Value;
use crate::database::audit::{buat_batch, catat_audit, EntriAudit};

// ============================================================
// STAGING
// ============================================================

pub fn buat_sesi_impor(
    conn: &Connection,
    dataset_id: i64,
    source_name: &str,
    source_sheet: Option<&str>,
    period_value: &str,
    oleh: &str,
) -> Result<i64, String> {
    let now = chrono_now();
    conn.execute(
        "INSERT INTO import_session (dataset_id,source_name,source_sheet,period_value,created_by,created_at) VALUES (?1,?2,?3,?4,?5,?6)",
        params![dataset_id, source_name, source_sheet, period_value, oleh, now],
    ).map_err(|e| format!("Gagal membuat sesi impor: {e}"))?;
    Ok(conn.last_insert_rowid())
}

pub fn simpan_staging_rows(
    conn: &Connection,
    session_id: i64,
    rows: &[Value],
) -> Result<usize, String> {
    for (i, row) in rows.iter().enumerate() {
        let raw = serde_json::to_string(row)
            .map_err(|e| format!("Gagal serialisasi baris {i}: {e}"))?;
        conn.execute(
            "INSERT INTO import_staging_row (session_id,row_no,raw_json) VALUES (?1,?2,?3)",
            params![session_id, i as i64, raw],
        ).map_err(|e| format!("Gagal menyimpan baris staging {i}: {e}"))?;
    }
    Ok(rows.len())
}

pub fn simpan_unit_match(
    conn: &Connection,
    session_id: i64,
    matches: &[Value],
) -> Result<(), String> {
    for m in matches {
        let source_name = m["source_name"].as_str().unwrap_or("");
        let unit_id = m["unit_id"].as_i64();
        let match_kind = m["match_kind"].as_str().unwrap_or("tidak_cocok");
        conn.execute(
            "INSERT OR REPLACE INTO import_unit_match (session_id,source_name,unit_id,match_kind) VALUES (?1,?2,?3,?4)",
            params![session_id, source_name, unit_id, match_kind],
        ).map_err(|e| format!("Gagal menyimpan unit match: {e}"))?;
    }
    Ok(())
}

// ============================================================
// COMMIT
// ============================================================

pub struct KomitImporInput {
    pub session_id: i64,
    pub dataset_id: i64,
    pub period_value: String,
    pub rows: Vec<Value>,
    pub col_variable_map: Vec<(String, i64, i64)>, // (kolom, variable_id, unit_id)
    pub col_unit_wilayah: String,
}

pub fn komit_impor(
    conn: &mut Connection,
    input: KomitImporInput,
    oleh: &str,
) -> Result<KomitHasil, String> {
    let tx = conn.transaction()
        .map_err(|e| format!("Gagal memulai transaksi impor: {e}"))?;
    let batch_id = buat_batch(&tx, "impor",
        Some(&format!("Impor sesi {}", input.session_id)), oleh)?;
    let mut inserted = 0i64;
    let mut skipped = 0i64;
    let now = chrono_now();
    for row in &input.rows {
        let unit_name = row[&input.col_unit_wilayah].as_str().unwrap_or("");
        let unit_id: i64 = tx.query_row(
            "SELECT unit_id FROM import_unit_match WHERE session_id=?1 AND source_name=?2",
            params![input.session_id, unit_name],
            |r| r.get(0),
        ).map_err(|_| format!("Wilayah '{unit_name}' belum dipetakan ke master."))?;
        for (col, variable_id, uom_id) in &input.col_variable_map {
            let val = &row[col];
            if val.is_null() { continue; }
            let (value_status, value_num, value_text): (&str, Option<f64>, Option<String>) =
                if let Some(n) = val.as_f64() { ("terisi", Some(n), None) }
                else { ("terisi", None, Some(val.to_string())) };
            let existing: Option<i64> = tx.query_row(
                "SELECT id FROM observation WHERE dataset_id=?1 AND variable_id=?2 AND unit_id=?3 AND period_value=?4 AND dimension_value_id=0",
                params![input.dataset_id, variable_id, unit_id, input.period_value],
                |r| r.get(0),
            ).ok();
            if existing.is_some() {
                skipped += 1;
                continue;
            }
            tx.execute(
                "INSERT INTO observation (dataset_id,variable_id,unit_id,period_value,value_status,value_num,value_text,batch_id,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?9)",
                params![input.dataset_id, variable_id, unit_id, input.period_value,
                        value_status, value_num, value_text, batch_id, now],
            ).map_err(|e| format!("Gagal menyimpan observasi: {e}"))?;
            let obs_id = tx.last_insert_rowid();
            catat_audit(&tx, &EntriAudit {
                entity_type: "observation", entity_id: obs_id,
                dataset_id: Some(input.dataset_id),
                action: "create", field: None, old_value: None,
                new_value: value_num.map(|n| n.to_string()).as_deref()
                    .or(value_text.as_deref()),
                reason: None, batch_id: Some(batch_id), changed_by: oleh,
            })?;
            inserted += 1;
        }
    }
    tx.execute(
        "UPDATE import_session SET status='committed', batch_id=?1 WHERE id=?2",
        params![batch_id, input.session_id],
    ).map_err(|e| format!("Gagal update status sesi: {e}"))?;
    tx.commit().map_err(|e| format!("Gagal commit impor: {e}"))?;
    Ok(KomitHasil { batch_id, inserted, skipped })
}

pub struct KomitHasil {
    pub batch_id: i64,
    pub inserted: i64,
    pub skipped:  i64,
}

// ============================================================
// UNDO
// ============================================================

pub fn undo_batch(
    conn: &mut Connection,
    batch_id: i64,
    oleh: &str,
) -> Result<i64, String> {
    let tx = conn.transaction()
        .map_err(|e| format!("Gagal memulai transaksi undo: {e}"))?;
    let undo_batch_id = buat_batch(&tx, "koreksi",
        Some(&format!("Undo batch {batch_id}")), oleh)?;
    // Ambil semua observasi yang dibuat oleh batch ini
    let mut stmt = tx.prepare(
        "SELECT entity_id FROM audit_log WHERE batch_id=?1 AND entity_type='observation' AND action='create'"
    ).map_err(|e| format!("Gagal menyiapkan undo: {e}"))?;
    let ids: Vec<i64> = stmt.query_map([batch_id], |r| r.get(0))
        .map_err(|e| format!("Gagal membaca audit untuk undo: {e}"))?
        .filter_map(|r| r.ok()).collect();
    drop(stmt);
    let n = ids.len() as i64;
    for obs_id in &ids {
        catat_audit(&tx, &EntriAudit {
            entity_type: "observation", entity_id: *obs_id,
            dataset_id: None,
            action: "delete", field: None, old_value: None, new_value: None,
            reason: Some(&format!("undo batch {batch_id}")),
            batch_id: Some(undo_batch_id), changed_by: oleh,
        })?;
        tx.execute("DELETE FROM observation WHERE id=?1", [obs_id])
            .map_err(|e| format!("Gagal hapus observasi {obs_id}: {e}"))?;
    }
    tx.execute(
        "UPDATE batch SET undone_at=?1 WHERE id=?2",
        params![chrono_now(), batch_id],
    ).map_err(|e| format!("Gagal tandai batch sebagai undo: {e}"))?;
    tx.commit().map_err(|e| format!("Gagal commit undo: {e}"))?;
    Ok(n)
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = d.as_secs();
    let millis = d.subsec_millis();
    let s = secs % 60; let m = (secs / 60) % 60; let h = (secs / 3600) % 24;
    let days = secs / 86400;
    let (y, mo, dy) = days_to_ymd(days);
    format!("{y:04}-{mo:02}-{dy:02}T{h:02}:{m:02}:{s:02}.{millis:03}Z")
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut y = 1970u64;
    loop {
        let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
        let dy = if leap { 366 } else { 365 };
        if days < dy { break; }
        days -= dy; y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let months = [31u64,if leap{29}else{28},31,30,31,30,31,31,30,31,30,31];
    let mut mo = 1u64;
    for &dm in &months {
        if days < dm { break; }
        days -= dm; mo += 1;
    }
    (y, mo, days + 1)
}
