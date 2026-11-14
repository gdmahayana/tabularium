//! Pembuka koneksi SQLite dan sistem migrasi TABULARIUM.
//!
//! Aturan:
//! * Migrasi = file SQL bernomor di `src-tauri/migrations/`, disematkan ke biner
//!   lewat `include_str!` (pemasang tidak perlu membawa folder terpisah).
//! * Migrasi yang sudah dijalankan tidak boleh diubah: checksum SHA-256 dicatat
//!   dan dibandingkan setiap kali aplikasi dibuka.
//! * Tiap migrasi berjalan dalam satu transaksi; gagal berarti tidak ada yang berubah.
//! * Hanya Rust yang menulis ke database (keputusan K-02).

use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

/// Mode WAL diuji dulu sebelum diterapkan permanen (dokumen induk 8.3).
/// Ubah ke `false` bila uji menunjukkan masalah.
const GUNAKAN_WAL: bool = true;

pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

/// Daftar migrasi, berurutan. Tambahkan entri baru di akhir; jangan mengubah yang lama.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "0001_init",
        sql: include_str!("../../migrations/0001_init.sql"),
    },
    Migration {
        version: 2,
        name: "0002_audit_triggers",
        sql: include_str!("../../migrations/0002_audit_triggers.sql"),
    },
    Migration {
        version: 3,
        name: "0003_import_quality",
        sql: include_str!("../../migrations/0003_import_quality.sql"),
    },
    Migration {
        version: 4,
        name: "0004_indicator",
        sql: include_str!("../../migrations/0004_indicator.sql"),
    },
    Migration {
        version: 5,
        name: "0005_portal_mapping",
        sql: include_str!("../../migrations/0005_portal_mapping.sql"),
    },
];

/// SHA-256 dari isi SQL. Akhir baris dinormalisasi agar hasilnya sama di Windows dan Linux.
pub fn checksum(sql: &str) -> String {
    let normal = sql.replace("\r\n", "\n");
    Sha256::digest(normal.as_bytes())
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

/// Pengaturan wajib untuk setiap koneksi. `foreign_keys` tidak tersimpan di berkas
/// database sehingga harus diaktifkan pada setiap koneksi baru.
pub fn configure(conn: &Connection) -> Result<(), String> {
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| format!("Gagal mengaktifkan pemeriksaan relasi data: {e}"))?;
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(|e| format!("Gagal mengatur batas tunggu database: {e}"))?;
    if GUNAKAN_WAL {
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |r| r.get::<_, String>(0))
            .map_err(|e| format!("Gagal mengaktifkan mode WAL: {e}"))?;
    }
    Ok(())
}

/// Buka (atau buat) database di `path`, atur koneksi, lalu jalankan migrasi tertunda.
pub fn open_and_migrate(path: &Path) -> Result<Connection, String> {
    let mut conn = Connection::open(path)
        .map_err(|e| format!("Tidak dapat membuka database di {}: {e}", path.display()))?;
    configure(&conn)?;
    run_migrations(&mut conn)?;
    Ok(conn)
}

fn pastikan_tabel_migrasi(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS migration (
            version    INTEGER PRIMARY KEY NOT NULL,
            name       TEXT NOT NULL,
            checksum   TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        ) STRICT;",
    )
    .map_err(|e| format!("Gagal menyiapkan tabel riwayat migrasi: {e}"))
}

/// version -> checksum yang tercatat.
fn migrasi_terpasang(conn: &Connection) -> Result<HashMap<i64, String>, String> {
    let mut stmt = conn
        .prepare("SELECT version, checksum FROM migration")
        .map_err(|e| format!("Gagal membaca riwayat migrasi: {e}"))?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| format!("Gagal membaca riwayat migrasi: {e}"))?;
    let mut hasil = HashMap::new();
    for baris in rows {
        let (v, c) = baris.map_err(|e| format!("Gagal membaca riwayat migrasi: {e}"))?;
        hasil.insert(v, c);
    }
    Ok(hasil)
}

/// Nama migrasi yang belum dijalankan. Dipakai pemanggil untuk membuat backup
/// otomatis sebelum migrasi (dokumen induk 8.6).
pub fn migrasi_tertunda(conn: &Connection) -> Result<Vec<&'static str>, String> {
    pastikan_tabel_migrasi(conn)?;
    let terpasang = migrasi_terpasang(conn)?;
    Ok(MIGRATIONS
        .iter()
        .filter(|m| !terpasang.contains_key(&m.version))
        .map(|m| m.name)
        .collect())
}

/// Jalankan semua migrasi yang belum terpasang. Mengembalikan nama migrasi yang baru dijalankan.
pub fn run_migrations(conn: &mut Connection) -> Result<Vec<String>, String> {
    pastikan_tabel_migrasi(conn)?;
    let terpasang = migrasi_terpasang(conn)?;

    // Database dibuat oleh versi aplikasi yang lebih baru.
    let versi_tertinggi = MIGRATIONS.iter().map(|m| m.version).max().unwrap_or(0);
    if let Some(v) = terpasang.keys().filter(|v| **v > versi_tertinggi).max() {
        return Err(format!(
            "Database ini dibuat oleh versi TABULARIUM yang lebih baru (migrasi {v}). \
             Perbarui aplikasi sebelum membukanya."
        ));
    }

    // Migrasi lama tidak boleh berubah.
    for m in MIGRATIONS {
        if let Some(tercatat) = terpasang.get(&m.version) {
            if *tercatat != checksum(m.sql) {
                return Err(format!(
                    "Berkas migrasi {} berbeda dari yang pernah dijalankan pada database ini. \
                     Migrasi lama tidak boleh diubah; buat migrasi baru.",
                    m.name
                ));
            }
        }
    }

    let mut dijalankan = Vec::new();
    for m in MIGRATIONS.iter().filter(|m| !terpasang.contains_key(&m.version)) {
        let tx = conn
            .transaction()
            .map_err(|e| format!("Gagal memulai migrasi {}: {e}", m.name))?;
        tx.execute_batch(m.sql)
            .map_err(|e| format!("Migrasi {} gagal dan dibatalkan: {e}", m.name))?;
        tx.execute(
            "INSERT INTO migration (version, name, checksum) VALUES (?1, ?2, ?3)",
            params![m.version, m.name, checksum(m.sql)],
        )
        .map_err(|e| format!("Gagal mencatat migrasi {}: {e}", m.name))?;
        tx.commit()
            .map_err(|e| format!("Gagal menyelesaikan migrasi {}: {e}", m.name))?;
        dijalankan.push(m.name.to_string());
    }
    Ok(dijalankan)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baru() -> Connection {
        let mut c = Connection::open_in_memory().unwrap();
        configure(&c).unwrap();
        run_migrations(&mut c).unwrap();
        c
    }

    fn seed(c: &Connection) {
        c.execute_batch(
            "INSERT INTO ref_unit (id, name, level) VALUES (1, 'Karangasem', 'kabupaten');
             INSERT INTO ref_unit (id, name, level, parent_id) VALUES (2, 'Desa A', 'desa', 1);
             INSERT INTO dataset (id, name, period_type) VALUES (1, 'PKK', 'tahunan');
             INSERT INTO dataset (id, name, period_type) VALUES (2, 'Aset', 'tahunan');
             INSERT INTO variable (id, dataset_id, name, code, data_type)
                 VALUES (1, 1, 'Jumlah kelompok PKK', 'jumlah_kelompok_pkk', 'integer');",
        )
        .unwrap();
    }

    fn obs(c: &Connection, dataset: i64, periode: &str, status: &str, nilai: Option<f64>) -> rusqlite::Result<usize> {
        c.execute(
            "INSERT INTO observation (dataset_id, variable_id, unit_id, period_value, value_status, value_num)
             VALUES (?1, 1, 2, ?2, ?3, ?4)",
            params![dataset, periode, status, nilai],
        )
    }

    #[test]
    fn migrasi_bersih_membuat_semua_tabel() {
        let c = baru();
        for tabel in [
            "app_config", "ref_unit", "ref_period_type", "batch", "dataset", "variable",
            "label", "dataset_label", "variable_label", "dimension", "dimension_value",
            "observation", "audit_log", "migration",
        ] {
            let n: i64 = c
                .query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?1", [tabel], |r| r.get(0))
                .unwrap();
            assert_eq!(n, 1, "tabel {tabel} tidak ada");
        }
    }

    #[test]
    fn migrasi_kedua_kali_tidak_menjalankan_apa_apa() {
        let mut c = baru();
        assert!(run_migrations(&mut c).unwrap().is_empty());
        assert!(migrasi_tertunda(&c).unwrap().is_empty());
    }

    #[test]
    fn perubahan_berkas_migrasi_lama_terdeteksi() {
        let mut c = baru();
        c.execute("UPDATE migration SET checksum = 'palsu' WHERE version = 1", []).unwrap();
        let err = run_migrations(&mut c).unwrap_err();
        assert!(err.contains("0001_init"), "{err}");
    }

    #[test]
    fn database_dari_versi_lebih_baru_ditolak() {
        let mut c = baru();
        c.execute("INSERT INTO migration (version, name, checksum) VALUES (999, 'masa_depan', 'x')", []).unwrap();
        assert!(run_migrations(&mut c).unwrap_err().contains("lebih baru"));
    }

    #[test]
    fn catatan_audit_tidak_bisa_diubah_atau_dihapus() {
        let c = baru();
        c.execute(
            "INSERT INTO audit_log (entity_type, entity_id, action, changed_by) VALUES ('dataset', 1, 'create', 'uji')",
            [],
        ).unwrap();
        assert!(c.execute("UPDATE audit_log SET reason = 'x'", []).is_err());
        assert!(c.execute("DELETE FROM audit_log", []).is_err());
    }

    #[test]
    fn nol_sebenarnya_berbeda_dari_kosong() {
        let c = baru();
        seed(&c);
        obs(&c, 1, "2026", "terisi", Some(0.0)).unwrap(); // nol sebenarnya: sah
        assert!(obs(&c, 1, "2027", "kosong", Some(0.0)).is_err()); // kosong tidak boleh membawa nilai
        assert!(obs(&c, 1, "2028", "terisi", None).is_err()); // terisi wajib bernilai
        obs(&c, 1, "2029", "tidak_berlaku", None).unwrap();
    }

    #[test]
    fn observasi_ganda_ditolak() {
        let c = baru();
        seed(&c);
        obs(&c, 1, "2026", "terisi", Some(1.0)).unwrap();
        assert!(obs(&c, 1, "2026", "terisi", Some(2.0)).is_err());
    }

    #[test]
    fn variabel_harus_milik_dataset_yang_sama() {
        let c = baru();
        seed(&c);
        assert!(obs(&c, 2, "2026", "terisi", Some(1.0)).is_err()); // variabel 1 milik dataset 1
    }

    #[test]
    fn dataset_berobservasi_tidak_bisa_dihapus() {
        let c = baru();
        seed(&c);
        obs(&c, 1, "2026", "terisi", Some(1.0)).unwrap();
        assert!(c.execute("DELETE FROM dataset WHERE id = 1", []).is_err());
        assert!(c.execute("DELETE FROM dataset WHERE id = 2", []).is_ok()); // dataset kosong boleh
    }
}