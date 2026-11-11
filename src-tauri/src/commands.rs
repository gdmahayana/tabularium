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
