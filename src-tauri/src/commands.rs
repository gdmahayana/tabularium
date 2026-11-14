// SPDX-License-Identifier: AGPL-3.0-only
//! Tauri Commands — jembatan antara frontend Svelte dan database Rust.
//!
//! Aturan:
//!   * Semua fungsi bertipe `#[tauri::command]` dan mengembalikan `Result<T, String>`.
//!   * Pesan error dalam bahasa Indonesia, siap ditampilkan langsung di UI.
//!   * State koneksi database dikelola lewat `tauri::State<DbState>`.
//!   * Frontend tidak pernah mengakses database langsung.

use std::path::PathBuf;
use std::sync::Mutex;
use rusqlite::Connection;
use tauri::State;

use crate::sidecar::Sidecar;
use serde_json::{json, Value};
use crate::database::import_service::{self, KomitImporInput};
use crate::database::{
    backup,
    dataset_service,
    migrations,
    models::{
        BuatDatasetInput, Dataset, EntriAuditLog, Label,
        RefUnit, UbahDatasetInput, BuatVariableInput, Variable, UbahVariableInput,
    },
    unit_service,
    variable_service,
    indicator_service::{self, Indicator, IndicatorInput, HasilIndikator},
    label_service::{self, Label as LabelItem, EntitasSektoral},
    mapping_service::{self, Mapping, MappingInput},
    sdi_service::{self, SelLembar, TandaiSelInput, SubmissionInput, Submission, RingkasanLembar},
    export_service,
};

// ============================================================
// STATE
// ============================================================

pub struct DbState {
    pub conn: Mutex<Connection>,
    pub db_path: PathBuf,
    pub backup_dir: PathBuf,
    pub sidecar: Option<Sidecar>,
}

impl DbState {
    pub fn buka(db_path: PathBuf, backup_dir: PathBuf) -> Result<Self, String> {
        let mut conn = migrations::open_and_migrate(&db_path)?;
        let sidecar_exe = std::env::current_exe().ok()
            .and_then(|p| Some(p.parent()?.join("tabularium-sidecar")))
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "tabularium-sidecar".to_string());
        let sidecar = Sidecar::spawn(&sidecar_exe).ok();
        Ok(Self { conn: Mutex::new(conn), db_path, backup_dir, sidecar })
    }
}

// ============================================================
// HELPER
// ============================================================

/// Nama sesi pengguna untuk audit. Tahap awal: single-user, nama tetap.
/// Tahap multi-user (Tahap 8): diganti dengan identitas dari sesi login.
const PENGGUNA: &str = "pengguna";

macro_rules! kunci {
    ($state:expr) => {
        $state.conn.lock()
            .map_err(|_| "Database sedang digunakan, coba beberapa saat lagi.".to_string())?
    };
}

// ============================================================
// DATASET
// ============================================================

#[tauri::command]
pub fn cmd_buat_dataset(
    state: State<DbState>,
    input: BuatDatasetInput,
) -> Result<Dataset, String> {
    let mut conn = kunci!(state);
    dataset_service::buat_dataset(&mut conn, input, PENGGUNA)
}

#[tauri::command]
pub fn cmd_ambil_dataset(
    state: State<DbState>,
    id: i64,
) -> Result<Dataset, String> {
    let conn = kunci!(state);
    dataset_service::ambil_dataset(&conn, id)
}

#[tauri::command]
pub fn cmd_daftar_dataset(state: State<DbState>) -> Result<Vec<Dataset>, String> {
    let conn = kunci!(state);
    dataset_service::daftar_dataset(&conn)
}

#[tauri::command]
pub fn cmd_ubah_dataset(
    state: State<DbState>,
    id: i64,
    input: UbahDatasetInput,
) -> Result<Dataset, String> {
    let mut conn = kunci!(state);
    dataset_service::ubah_dataset(&mut conn, id, input, PENGGUNA)
}

#[tauri::command]
pub fn cmd_hapus_dataset(
    state: State<DbState>,
    id: i64,
) -> Result<(), String> {
    let mut conn = kunci!(state);
    dataset_service::hapus_dataset(&mut conn, id, PENGGUNA)
}

// ============================================================
// VARIABLE
// ============================================================

#[tauri::command]
pub fn cmd_buat_variable(
    state: State<DbState>,
    input: BuatVariableInput,
) -> Result<Variable, String> {
    let mut conn = kunci!(state);
    variable_service::buat_variable(&mut conn, input, PENGGUNA)
}

#[tauri::command]
pub fn cmd_daftar_variable(
    state: State<DbState>,
    dataset_id: i64,
) -> Result<Vec<Variable>, String> {
    let conn = kunci!(state);
    variable_service::daftar_variable(&conn, dataset_id)
}

#[tauri::command]
pub fn cmd_ubah_variable(
    state: State<DbState>,
    id: i64,
    input: UbahVariableInput,
) -> Result<Variable, String> {
    let mut conn = kunci!(state);
    variable_service::ubah_variable(&mut conn, id, input, PENGGUNA)
}

#[tauri::command]
pub fn cmd_arsipkan_variable(
    state: State<DbState>,
    id: i64,
) -> Result<(), String> {
    let mut conn = kunci!(state);
    variable_service::arsipkan_variable(&mut conn, id, PENGGUNA)
}

#[tauri::command]
pub fn cmd_atur_urutan_variable(
    state: State<DbState>,
    dataset_id: i64,
    urutan_id: Vec<i64>,
) -> Result<(), String> {
    let mut conn = kunci!(state);
    variable_service::atur_urutan_variable(&mut conn, dataset_id, urutan_id, PENGGUNA)
}

// ============================================================
// WILAYAH
// ============================================================

#[tauri::command]
pub fn cmd_daftar_unit(
    state: State<DbState>,
    level: String,
) -> Result<Vec<RefUnit>, String> {
    let conn = kunci!(state);
    unit_service::daftar_unit(&conn, &level)
}

#[tauri::command]
pub fn cmd_daftar_anak_unit(
    state: State<DbState>,
    parent_id: i64,
) -> Result<Vec<RefUnit>, String> {
    let conn = kunci!(state);
    unit_service::daftar_anak(&conn, parent_id)
}

#[tauri::command]
pub fn cmd_impor_wilayah_csv(
    state: State<DbState>,
    isi_csv: String,
) -> Result<usize, String> {
    let baris = unit_service::parse_csv_wilayah(&isi_csv)?;
    let jumlah = baris.len();
    let mut conn = kunci!(state);
    unit_service::impor_wilayah(&mut conn, baris, PENGGUNA)?;
    Ok(jumlah)
}

// ============================================================
// AUDIT
// ============================================================

#[tauri::command]
pub fn cmd_audit_log(
    state: State<DbState>,
    entity_type: String,
    entity_id: i64,
    limit: Option<i64>,
) -> Result<Vec<EntriAuditLog>, String> {
    let conn = kunci!(state);
    let limit = limit.unwrap_or(100);
    let mut stmt = conn.prepare(
        "SELECT id,entity_type,entity_id,action,field,old_value,new_value,reason,changed_by,changed_at
         FROM audit_log WHERE entity_type=?1 AND entity_id=?2
         ORDER BY changed_at DESC LIMIT ?3",
    ).map_err(|e| format!("Gagal menyiapkan kueri audit: {e}"))?;
    let rows = stmt.query_map(
        rusqlite::params![entity_type, entity_id, limit],
        |r| Ok(EntriAuditLog {
            id:          r.get(0)?,
            entity_type: r.get(1)?,
            entity_id:   r.get(2)?,
            action:      r.get(3)?,
            field:       r.get(4)?,
            old_value:   r.get(5)?,
            new_value:   r.get(6)?,
            reason:      r.get(7)?,
            changed_by:  r.get(8)?,
            changed_at:  r.get(9)?,
        }),
    ).map_err(|e| format!("Gagal membaca audit log: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Gagal membaca audit log: {e}"))
}

// ============================================================
// LABEL
// ============================================================

#[tauri::command]
pub fn cmd_daftar_label(state: State<DbState>) -> Result<Vec<Label>, String> {
    let conn = kunci!(state);
    let mut stmt = conn.prepare(
        "SELECT id,name,description FROM label ORDER BY name",
    ).map_err(|e| format!("Gagal membaca daftar label: {e}"))?;
    let rows = stmt.query_map([], |r| Ok(Label {
        id: r.get(0)?, name: r.get(1)?, description: r.get(2)?,
    })).map_err(|e| format!("Gagal membaca label: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Gagal membaca label: {e}"))
}

#[tauri::command]
pub fn cmd_tambah_label_dataset(
    state: State<DbState>,
    dataset_id: i64,
    label_id: i64,
) -> Result<(), String> {
    let conn = kunci!(state);
    conn.execute(
        "INSERT OR IGNORE INTO dataset_label (dataset_id,label_id) VALUES (?1,?2)",
        rusqlite::params![dataset_id, label_id],
    ).map_err(|e| format!("Gagal menambah label dataset: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn cmd_hapus_label_dataset(
    state: State<DbState>,
    dataset_id: i64,
    label_id: i64,
) -> Result<(), String> {
    let conn = kunci!(state);
    conn.execute(
        "DELETE FROM dataset_label WHERE dataset_id=?1 AND label_id=?2",
        rusqlite::params![dataset_id, label_id],
    ).map_err(|e| format!("Gagal menghapus label dataset: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn cmd_label_dataset(
    state: State<DbState>,
    dataset_id: i64,
) -> Result<Vec<Label>, String> {
    let conn = kunci!(state);
    let mut stmt = conn.prepare(
        "SELECT l.id,l.name,l.description FROM label l
         JOIN dataset_label dl ON dl.label_id=l.id
         WHERE dl.dataset_id=?1 ORDER BY l.name",
    ).map_err(|e| format!("Gagal membaca label dataset: {e}"))?;
    let rows = stmt.query_map([dataset_id], |r| Ok(Label {
        id: r.get(0)?, name: r.get(1)?, description: r.get(2)?,
    })).map_err(|e| format!("Gagal membaca label dataset: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Gagal membaca label dataset: {e}"))
}

// ============================================================
// BACKUP & RESTORE
// ============================================================

#[tauri::command]
pub fn cmd_buat_backup(state: State<DbState>) -> Result<String, String> {
    let conn = kunci!(state);
    let path = backup::buat_backup(&conn, &state.backup_dir)?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn cmd_daftar_backup(state: State<DbState>) -> Result<Vec<String>, String> {
    let list = backup::daftar_backup(&state.backup_dir)?;
    Ok(list.iter().map(|p| p.to_string_lossy().to_string()).collect())
}

#[tauri::command]
pub fn cmd_restore_backup(
    state: State<DbState>,
    backup_path: String,
) -> Result<(), String> {
    // Koneksi harus dilepas sementara — kunci mutex, lakukan restore,
    // lalu buka koneksi baru dan ganti isi Mutex.
    let backup_path = PathBuf::from(backup_path);
    if !backup_path.exists() {
        return Err(format!("File backup tidak ditemukan: {}", backup_path.display()));
    }
    let mut conn = kunci!(state);
    // Jalankan restore ke db_path; conn masih terbuka tapi backup API
    // membuka koneksi src/dst sendiri — aman karena WAL aktif.
    backup::restore_dari_backup(&backup_path, &state.db_path, &state.backup_dir)?;
    // Reload koneksi agar cache query Rust mengacu ke data yang baru.
    *conn = migrations::open_and_migrate(&state.db_path)?;
    Ok(())
}

// ============================================================
// DAFTARKAN SEMUA COMMANDS KE TAURI BUILDER
// ============================================================

/// Panggil fungsi ini di `lib.rs` untuk mendaftarkan semua commands.
pub fn daftarkan(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder.invoke_handler(tauri::generate_handler![
        cmd_buat_dataset,
        cmd_ambil_dataset,
        cmd_daftar_dataset,
        cmd_ubah_dataset,
        cmd_hapus_dataset,
        cmd_buat_variable,
        cmd_daftar_variable,
        cmd_ubah_variable,
        cmd_arsipkan_variable,
        cmd_atur_urutan_variable,
        cmd_daftar_unit,
        cmd_daftar_anak_unit,
        cmd_impor_wilayah_csv,
        cmd_audit_log,
        cmd_daftar_label,
        cmd_tambah_label_dataset,
        cmd_hapus_label_dataset,
        cmd_label_dataset,
        cmd_buat_backup,
        cmd_daftar_backup,
        cmd_restore_backup,
    ])
}
// ============================================================
// SIDECAR — IMPOR FILE
// ============================================================

#[tauri::command]
pub fn cmd_inspeksi_file(state: State<DbState>, path: String) -> Result<Value, String> {
    match &state.sidecar {
        None => Err("Sidecar tidak tersedia. Impor Excel tidak dapat dijalankan.".to_string()),
        Some(sc) => sc.call("inspect_file", json!({"path": path})),
    }
}

#[tauri::command]
pub fn cmd_baca_tabel(
    state: State<DbState>,
    path: String,
    sheet: String,
    out_dir: Option<String>,
) -> Result<Value, String> {
    match &state.sidecar {
        None => Err("Sidecar tidak tersedia. Impor Excel tidak dapat dijalankan.".to_string()),
        Some(sc) => sc.call("read_table", json!({
            "path": path,
            "sheet": sheet,
            "out_dir": out_dir,
        })),
    }
}

// ============================================================
// IMPOR — COMMIT DAN UNDO
// ============================================================

#[tauri::command]
pub fn cmd_buat_sesi_impor(
    state: State<DbState>,
    dataset_id: i64,
    source_name: String,
    source_sheet: Option<String>,
    period_value: String,
) -> Result<i64, String> {
    let conn = kunci!(state);
    import_service::buat_sesi_impor(
        &conn, dataset_id, &source_name,
        source_sheet.as_deref(), &period_value, PENGGUNA,
    )
}

#[tauri::command]
pub fn cmd_simpan_staging(
    state: State<DbState>,
    session_id: i64,
    rows: Vec<serde_json::Value>,
) -> Result<usize, String> {
    let conn = kunci!(state);
    import_service::simpan_staging_rows(&conn, session_id, &rows)
}

#[tauri::command]
pub fn cmd_simpan_unit_match(
    state: State<DbState>,
    session_id: i64,
    matches: Vec<serde_json::Value>,
) -> Result<(), String> {
    let conn = kunci!(state);
    import_service::simpan_unit_match(&conn, session_id, &matches)
}

#[tauri::command]
pub fn cmd_komit_impor(
    state: State<DbState>,
    session_id: i64,
    dataset_id: i64,
    period_value: String,
    rows: Vec<serde_json::Value>,
    col_variable_map: Vec<(String, i64, i64)>,
    col_unit_wilayah: String,
) -> Result<serde_json::Value, String> {
    let mut conn = kunci!(state);
    let hasil = import_service::komit_impor(&mut conn, KomitImporInput {
        session_id, dataset_id, period_value, rows,
        col_variable_map, col_unit_wilayah,
    }, PENGGUNA)?;
    Ok(serde_json::json!({
        "batch_id": hasil.batch_id,
        "inserted": hasil.inserted,
        "skipped":  hasil.skipped,
    }))
}

#[tauri::command]
pub fn cmd_undo_batch(
    state: State<DbState>,
    batch_id: i64,
) -> Result<i64, String> {
    let mut conn = kunci!(state);
    import_service::undo_batch(&mut conn, batch_id, PENGGUNA)
}

#[tauri::command]
pub fn cmd_validasi_impor(
    state: State<DbState>,
    session_id: i64,
    dataset_id: i64,
    col_variable_map: Vec<(String, i64, String)>,
    col_unit_wilayah: String,
    period_value: String,
) -> Result<serde_json::Value, String> {
    let conn = kunci!(state);
    use crate::database::validation_service;
    let hasil = validation_service::jalankan_validasi(
        &conn, session_id, dataset_id,
        &col_variable_map, &col_unit_wilayah, &period_value,
    )?;
    let issues: Vec<_> = hasil.issues.iter().map(|x| serde_json::json!({
        "row_no":   x.row_no,
        "unit_ref": x.unit_ref,
        "severity": x.severity,
        "message":  x.message,
    })).collect();
    Ok(serde_json::json!({
        "result":  hasil.result,
        "run_id":  hasil.run_id,
        "issues":  issues,
    }))
}

// ============================================================
// INPUT MANUAL

// ============================================================

#[tauri::command]
pub fn cmd_simpan_observasi(
    state: State<DbState>,
    dataset_id: i64,
    variable_id: i64,
    unit_id: i64,
    period_value: String,
    dimension_value_id: Option<i64>,
    value_status: String,
    value_num: Option<f64>,
    value_text: Option<String>,
    value_date: Option<String>,
    reason: Option<String>,
) -> Result<i64, String> {
    use rusqlite::params;
    use crate::database::audit::{buat_batch, catat_audit, EntriAudit};
    let mut conn = kunci!(state);
    let tx = conn.transaction()
        .map_err(|e| format!("Gagal memulai transaksi: {e}"))?;
    let batch_id = buat_batch(&tx, "manual", reason.as_deref(), PENGGUNA)?;
    let dim_id = dimension_value_id.unwrap_or(0);
    let now = crate::database::import_service::now_iso();
    tx.execute(
        "INSERT INTO observation (dataset_id,variable_id,unit_id,period_value,dimension_value_id,value_status,value_num,value_text,value_date,batch_id,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?11)",
        params![dataset_id, variable_id, unit_id, period_value, dim_id,
                value_status, value_num, value_text, value_date, batch_id, now],
    ).map_err(|e| format!("Gagal menyimpan observasi: {e}"))?;
    let obs_id = tx.last_insert_rowid();
    let new_val = value_num.map(|n| n.to_string())
        .or_else(|| value_text.clone())
        .or_else(|| value_date.clone())
        .unwrap_or_else(|| value_status.clone());
    catat_audit(&tx, &EntriAudit {
        entity_type: "observation", entity_id: obs_id,
        dataset_id: Some(dataset_id),
        action: "create", field: None, old_value: None,
        new_value: Some(&new_val),
        reason: reason.as_deref(),
        batch_id: Some(batch_id), changed_by: PENGGUNA,
    })?;
    tx.commit().map_err(|e| format!("Gagal commit input manual: {e}"))?;
    Ok(obs_id)
}

#[tauri::command]
pub fn cmd_ubah_observasi(
    state: State<DbState>,
    obs_id: i64,
    value_status: String,
    value_num: Option<f64>,
    value_text: Option<String>,
    value_date: Option<String>,
    reason: Option<String>,
) -> Result<(), String> {
    use rusqlite::params;
    use crate::database::audit::{buat_batch, catat_audit, EntriAudit};
    let mut conn = kunci!(state);
    let (old_status, old_num, old_text, old_date, dataset_id): (String, Option<f64>, Option<String>, Option<String>, i64) =
        conn.query_row(
            "SELECT value_status,value_num,value_text,value_date,dataset_id FROM observation WHERE id=?1",
            [obs_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)),
        ).map_err(|_| format!("Observasi id {obs_id} tidak ditemukan."))?;
    let tx = conn.transaction()
        .map_err(|e| format!("Gagal memulai transaksi: {e}"))?;
    let batch_id = buat_batch(&tx, "koreksi", reason.as_deref(), PENGGUNA)?;
    let now = crate::database::import_service::now_iso();
    tx.execute(
        "UPDATE observation SET value_status=?1,value_num=?2,value_text=?3,value_date=?4,updated_at=?5 WHERE id=?6",
        params![value_status, value_num, value_text, value_date, now, obs_id],
    ).map_err(|e| format!("Gagal mengubah observasi: {e}"))?;
    let old_val = old_num.map(|n| n.to_string())
        .or(old_text).or(old_date)
        .unwrap_or(old_status);
    let new_val = value_num.map(|n| n.to_string())
        .or_else(|| value_text.clone())
        .or_else(|| value_date.clone())
        .unwrap_or_else(|| value_status.clone());
    catat_audit(&tx, &EntriAudit {
        entity_type: "observation", entity_id: obs_id,
        dataset_id: Some(dataset_id),
        action: "update", field: Some("value"),
        old_value: Some(&old_val), new_value: Some(&new_val),
        reason: reason.as_deref(),
        batch_id: Some(batch_id), changed_by: PENGGUNA,
    })?;
    tx.commit().map_err(|e| format!("Gagal commit ubah observasi: {e}"))?;
    Ok(())
}

// ============================================================
// INDICATOR
// ============================================================

#[tauri::command]
pub fn cmd_buat_indicator(
    state: State<DbState>,
    input: IndicatorInput,
) -> Result<i64, String> {
    let conn = kunci!(state);
    indicator_service::buat(&conn, &input)
}

#[tauri::command]
pub fn cmd_ambil_indicator(
    state: State<DbState>,
    id: i64,
) -> Result<Indicator, String> {
    let conn = kunci!(state);
    indicator_service::ambil(&conn, id)
}

#[tauri::command]
pub fn cmd_daftar_indicator(state: State<DbState>) -> Result<Vec<Indicator>, String> {
    let conn = kunci!(state);
    indicator_service::daftar(&conn)
}

#[tauri::command]
pub fn cmd_ubah_indicator(
    state: State<DbState>,
    id: i64,
    input: IndicatorInput,
) -> Result<(), String> {
    let conn = kunci!(state);
    indicator_service::ubah(&conn, id, &input)
}

#[tauri::command]
pub fn cmd_arsipkan_indicator(
    state: State<DbState>,
    id: i64,
) -> Result<(), String> {
    let conn = kunci!(state);
    indicator_service::arsipkan(&conn, id)
}

#[tauri::command]
pub fn cmd_hitung_indicator(
    state: State<DbState>,
    indicator_id: i64,
    period: String,
) -> Result<usize, String> {
    let conn = kunci!(state);
    indicator_service::hitung(&conn, indicator_id, &period)
}

#[tauri::command]
pub fn cmd_hasil_indicator(
    state: State<DbState>,
    indicator_id: i64,
    period: String,
) -> Result<Vec<HasilIndikator>, String> {
    let conn = kunci!(state);
    indicator_service::hasil(&conn, indicator_id, &period)
}

#[tauri::command]
pub fn cmd_jumlah_usang(state: State<DbState>) -> Result<i64, String> {
    let conn = kunci!(state);
    indicator_service::jumlah_usang(&conn)
}

#[tauri::command]
pub fn cmd_hitung_ulang_usang(state: State<DbState>) -> Result<usize, String> {
    let conn = kunci!(state);
    indicator_service::hitung_ulang_usang(&conn)
}

// ============================================================
// LABEL (TAHAP 3: variabel dan indikator)
// ============================================================

#[tauri::command]
pub fn cmd_daftar_semua_label(state: State<DbState>) -> Result<Vec<LabelItem>, String> {
    let conn = kunci!(state);
    label_service::daftar_semua(&conn)
}

#[tauri::command]
pub fn cmd_beri_label_variabel(
    state: State<DbState>,
    variable_id: i64,
    label_name: String,
) -> Result<(), String> {
    let conn = kunci!(state);
    label_service::beri_label_variabel(&conn, variable_id, &label_name)
}

#[tauri::command]
pub fn cmd_cabut_label_variabel(
    state: State<DbState>,
    variable_id: i64,
    label_name: String,
) -> Result<(), String> {
    let conn = kunci!(state);
    label_service::cabut_label_variabel(&conn, variable_id, &label_name)
}

#[tauri::command]
pub fn cmd_label_variabel(
    state: State<DbState>,
    variable_id: i64,
) -> Result<Vec<LabelItem>, String> {
    let conn = kunci!(state);
    label_service::label_variabel(&conn, variable_id)
}

#[tauri::command]
pub fn cmd_beri_label_indikator(
    state: State<DbState>,
    indicator_id: i64,
    label_name: String,
) -> Result<(), String> {
    let conn = kunci!(state);
    label_service::beri_label_indikator(&conn, indicator_id, &label_name)
}

#[tauri::command]
pub fn cmd_cabut_label_indikator(
    state: State<DbState>,
    indicator_id: i64,
    label_name: String,
) -> Result<(), String> {
    let conn = kunci!(state);
    label_service::cabut_label_indikator(&conn, indicator_id, &label_name)
}

#[tauri::command]
pub fn cmd_label_indikator(
    state: State<DbState>,
    indicator_id: i64,
) -> Result<Vec<LabelItem>, String> {
    let conn = kunci!(state);
    label_service::label_indikator(&conn, indicator_id)
}

#[tauri::command]
pub fn cmd_entitas_sektoral(state: State<DbState>) -> Result<EntitasSektoral, String> {
    let conn = kunci!(state);
    label_service::entitas_sektoral(&conn)
}

// ============================================================
// PORTAL MAPPING
// ============================================================

#[tauri::command]
pub fn cmd_buat_mapping(
    state: State<DbState>,
    input: MappingInput,
) -> Result<i64, String> {
    let conn = kunci!(state);
    mapping_service::buat(&conn, &input)
}

#[tauri::command]
pub fn cmd_ubah_mapping(
    state: State<DbState>,
    id: i64,
    input: MappingInput,
) -> Result<(), String> {
    let conn = kunci!(state);
    mapping_service::ubah(&conn, id, &input)
}

#[tauri::command]
pub fn cmd_hapus_mapping(
    state: State<DbState>,
    id: i64,
) -> Result<(), String> {
    let conn = kunci!(state);
    mapping_service::hapus(&conn, id)
}

#[tauri::command]
pub fn cmd_daftar_mapping(
    state: State<DbState>,
    profile_id: String,
) -> Result<Vec<Mapping>, String> {
    let conn = kunci!(state);
    mapping_service::daftar_profil(&conn, &profile_id)
}

// ============================================================
// SDI WORKBENCH
// ============================================================

#[tauri::command]
pub fn cmd_susun_lembar(
    state: State<DbState>,
    profile_id: String,
    period_value: String,
    region_id: i64,
) -> Result<Vec<SelLembar>, String> {
    let conn = kunci!(state);
    sdi_service::susun_lembar(&conn, &profile_id, &period_value, region_id)
}

#[tauri::command]
pub fn cmd_tandai_sel(
    state: State<DbState>,
    input: TandaiSelInput,
) -> Result<(), String> {
    let conn = kunci!(state);
    sdi_service::tandai_sel(&conn, &input)
}

#[tauri::command]
pub fn cmd_batal_tandai_sel(
    state: State<DbState>,
    profile_id: String,
    period_value: String,
    bidang: String,
    sub_bidang: String,
    kategori: String,
    judul_kolom: String,
) -> Result<(), String> {
    let conn = kunci!(state);
    sdi_service::batal_tandai_sel(
        &conn, &profile_id, &period_value,
        &bidang, &sub_bidang, &kategori, &judul_kolom,
    )
}

#[tauri::command]
pub fn cmd_tandai_perlu_diperbarui(
    state: State<DbState>,
    profile_id: String,
    period_value: String,
    source_type: String,
    source_ref: String,
) -> Result<usize, String> {
    let conn = kunci!(state);
    sdi_service::tandai_perlu_diperbarui(
        &conn, &profile_id, &period_value, &source_type, &source_ref,
    )
}

#[tauri::command]
pub fn cmd_ringkasan_lembar(
    state: State<DbState>,
    profile_id: String,
    period_value: String,
    region_id: i64,
) -> Result<RingkasanLembar, String> {
    let conn = kunci!(state);
    let lembar = sdi_service::susun_lembar(&conn, &profile_id, &period_value, region_id)?;
    Ok(sdi_service::ringkasan(&lembar))
}

#[tauri::command]
pub fn cmd_catat_submission(
    state: State<DbState>,
    input: SubmissionInput,
) -> Result<i64, String> {
    let conn = kunci!(state);
    sdi_service::catat_submission(&conn, &input)
}

#[tauri::command]
pub fn cmd_daftar_submission(
    state: State<DbState>,
    profile_id: String,
    period_value: String,
) -> Result<Vec<Submission>, String> {
    let conn = kunci!(state);
    sdi_service::daftar_submission(&conn, &profile_id, &period_value)
}

// ============================================================
// EKSPOR
// ============================================================

#[tauri::command]
pub fn cmd_ekspor_csv(
    state: State<DbState>,
    profile_id: String,
    period_value: String,
    region_id: i64,
) -> Result<String, String> {
    let conn = kunci!(state);
    let lembar = sdi_service::susun_lembar(&conn, &profile_id, &period_value, region_id)?;
    Ok(export_service::ke_csv(&lembar))
}

#[tauri::command]
pub fn cmd_ekspor_json(
    state: State<DbState>,
    profile_id: String,
    period_value: String,
    region_id: i64,
) -> Result<serde_json::Value, String> {
    let conn = kunci!(state);
    let lembar = sdi_service::susun_lembar(&conn, &profile_id, &period_value, region_id)?;
    Ok(export_service::ke_json(&lembar))
}
