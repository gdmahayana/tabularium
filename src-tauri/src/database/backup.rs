// SPDX-License-Identifier: AGPL-3.0-only
//! Backup dan restore database TABULARIUM.
//!
//! Strategi (dokumen induk 8.6):
//!   * Backup memakai SQLite Online Backup API (aman saat WAL aktif).
//!   * File backup: backups/tabularium_YYYYMMDD_HHMMSS.sqlite
//!   * Restore: periksa integritas backup, tutup koneksi aktif,
//!     cadangkan database aktif, lalu ganti dengan file backup.
//!   * Backup otomatis dipanggil sebelum operasi berisiko (impor massal, dll).

use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Nama file backup berdasarkan waktu sekarang (UTC).
/// Format: tabularium_YYYYMMDD_HHMMSS.sqlite
fn nama_file_backup() -> String {
    // Hitung detik sejak epoch lalu konversi manual ke YYYYMMDD_HHMMSS
    let detik = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Konversi sederhana tanpa dependensi chrono
    let menit_total = detik / 60;
    let ss = detik % 60;
    let jam_total  = menit_total / 60;
    let mm         = menit_total % 60;
    let hari_total = jam_total / 24;
    let hh         = jam_total % 24;

    // Kalender Gregorian sederhana mulai 1970-01-01
    let mut tahun = 1970u32;
    let mut hari_sisa = hari_total as u32;
    loop {
        let d = hari_dalam_tahun(tahun);
        if hari_sisa < d { break; }
        hari_sisa -= d;
        tahun += 1;
    }
    let mut bulan = 1u32;
    loop {
        let d = hari_dalam_bulan(bulan, tahun);
        if hari_sisa < d { break; }
        hari_sisa -= d;
        bulan += 1;
    }
    let tanggal = hari_sisa + 1;

    format!(
        "tabularium_{:04}{:02}{:02}_{:02}{:02}{:02}.sqlite",
        tahun, bulan, tanggal, hh, mm, ss
    )
}

fn adalah_kabisat(tahun: u32) -> bool {
    (tahun % 4 == 0 && tahun % 100 != 0) || tahun % 400 == 0
}

fn hari_dalam_tahun(tahun: u32) -> u32 {
    if adalah_kabisat(tahun) { 366 } else { 365 }
}

fn hari_dalam_bulan(bulan: u32, tahun: u32) -> u32 {
    match bulan {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11               => 30,
        2 => if adalah_kabisat(tahun) { 29 } else { 28 },
        _ => 30,
    }
}

/// Buat backup database aktif ke folder `backup_dir`.
/// Mengembalikan path file backup yang dibuat.
pub fn buat_backup(conn: &Connection, backup_dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(backup_dir)
        .map_err(|e| format!("Gagal membuat folder backup: {e}"))?;

    let nama = nama_file_backup();
    let tujuan = backup_dir.join(&nama);

    // SQLite Online Backup API — aman saat WAL aktif dan ada koneksi baca.
    let mut dst = Connection::open(&tujuan)
        .map_err(|e| format!("Gagal membuat file backup '{}': {e}", tujuan.display()))?;

    let backup = rusqlite::backup::Backup::new(conn, &mut dst)
        .map_err(|e| format!("Gagal memulai proses backup: {e}"))?;

    backup.run_to_completion(100, std::time::Duration::from_millis(5), None)
        .map_err(|e| format!("Backup gagal di tengah jalan: {e}"))?;

    // Pastikan file backup bisa dibuka dan integritasnya baik.
    periksa_integritas(&tujuan)?;

    Ok(tujuan)
}

/// Periksa integritas file database di path tertentu.
pub fn periksa_integritas(path: &Path) -> Result<(), String> {
    let c = Connection::open(path)
        .map_err(|e| format!("Gagal membuka '{}' untuk pemeriksaan: {e}", path.display()))?;
    let hasil: String = c.query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|e| format!("Gagal menjalankan integrity_check: {e}"))?;
    if hasil != "ok" {
        return Err(format!(
            "File database '{}' rusak: {hasil}", path.display()
        ));
    }
    Ok(())
}

/// Daftar file backup di folder, diurutkan dari terbaru.
pub fn daftar_backup(backup_dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !backup_dir.exists() {
        return Ok(Vec::new());
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(backup_dir)
        .map_err(|e| format!("Gagal membaca folder backup: {e}"))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("sqlite"))
        .collect();
    // Nama file sudah mengandung timestamp, sort descending = terbaru duluan.
    files.sort_by(|a, b| b.cmp(a));
    Ok(files)
}

/// Restore database dari file backup ke path database aktif.
///
/// Langkah:
/// 1. Periksa integritas file backup.
/// 2. Buat cadangan pengaman dari database aktif (backup_dari_backup).
/// 3. Ganti file database aktif dengan isi backup menggunakan Backup API.
///
/// Pemanggil wajib menutup koneksi aktif sebelum memanggil fungsi ini,
/// lalu membuka koneksi baru setelah restore selesai.
pub fn restore_dari_backup(
    backup_path: &Path,
    db_path: &Path,
    backup_dir: &Path,
) -> Result<(), String> {
    // 1. Periksa integritas backup terlebih dahulu.
    periksa_integritas(backup_path)?;

    // 2. Cadangan pengaman dari database aktif (jika ada).
    if db_path.exists() {
        let cadangan = backup_dir.join(format!(
            "sebelum_restore_{}", nama_file_backup()
        ));
        std::fs::copy(db_path, &cadangan)
            .map_err(|e| format!("Gagal membuat cadangan pengaman: {e}"))?;
    }

    // 3. Salin isi backup ke database aktif menggunakan Backup API.
    //    Ini lebih aman daripada std::fs::copy karena menangani WAL.
    let src = Connection::open(backup_path)
        .map_err(|e| format!("Gagal membuka file backup untuk restore: {e}"))?;
    let mut dst = Connection::open(db_path)
        .map_err(|e| format!("Gagal membuka database tujuan restore: {e}"))?;

    let backup = rusqlite::backup::Backup::new(&src, &mut dst)
        .map_err(|e| format!("Gagal memulai proses restore: {e}"))?;

    backup.run_to_completion(100, std::time::Duration::from_millis(5), None)
        .map_err(|e| format!("Restore gagal di tengah jalan: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migrations;

    fn db_sementara() -> (Connection, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.sqlite");
        let mut c = Connection::open(&path).unwrap();
        migrations::configure(&c).unwrap();
        migrations::run_migrations(&mut c).unwrap();
        // Isi sedikit data
        c.execute(
            "INSERT INTO dataset (name, period_type) VALUES ('PKK','tahunan')", [],
        ).unwrap();
        (c, dir)
    }

    #[test]
    fn backup_membuat_file() {
        let (conn, dir) = db_sementara();
        let backup_dir = dir.path().join("backups");
        let path = buat_backup(&conn, &backup_dir).unwrap();
        assert!(path.exists());
        assert!(path.to_str().unwrap().contains("tabularium_"));
        // Verifikasi isi backup: harus ada tabel dataset
        let c2 = Connection::open(&path).unwrap();
        let n: i64 = c2.query_row(
            "SELECT count(*) FROM dataset", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn daftar_backup_urut_terbaru() {
        let (conn, dir) = db_sementara();
        let backup_dir = dir.path().join("backups");
        let p1 = buat_backup(&conn, &backup_dir).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        let p2 = buat_backup(&conn, &backup_dir).unwrap();
        let list = daftar_backup(&backup_dir).unwrap();
        assert_eq!(list.len(), 2);
        // Terbaru duluan
        assert_eq!(list[0], p2);
        assert_eq!(list[1], p1);
    }

    #[test]
    fn restore_menghasilkan_database_yang_sama() {
        let (conn, dir) = db_sementara();
        let db_path = dir.path().join("test.sqlite");
        let backup_dir = dir.path().join("backups");

        let backup_path = buat_backup(&conn, &backup_dir).unwrap();
        drop(conn); // tutup koneksi sebelum restore

        // Ubah database aktif
        {
            let c = Connection::open(&db_path).unwrap();
            migrations::configure(&c).unwrap();
            c.execute("INSERT INTO dataset (name,period_type) VALUES ('Tambahan','tahunan')", []).unwrap();
        }

        // Restore dari backup (sebelum penambahan)
        restore_dari_backup(&backup_path, &db_path, &backup_dir).unwrap();

        // Verifikasi: hanya 1 dataset (data tambahan hilang)
        let c3 = Connection::open(&db_path).unwrap();
        let n: i64 = c3.query_row("SELECT count(*) FROM dataset", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn periksa_integritas_file_valid() {
        let (conn, dir) = db_sementara();
        let backup_dir = dir.path().join("backups");
        let path = buat_backup(&conn, &backup_dir).unwrap();
        assert!(periksa_integritas(&path).is_ok());
    }

    #[test]
    fn periksa_integritas_file_rusak_ditolak() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rusak.sqlite");
        std::fs::write(&path, b"ini bukan sqlite").unwrap();
        assert!(periksa_integritas(&path).is_err());
    }
}