// SPDX-License-Identifier: AGPL-3.0-only
//! Service dataset — semua operasi tulis dataset lewat fungsi ini.
//! Rust satu-satunya penulis database (K-02); setiap perubahan dicatat audit.

use rusqlite::{params, Connection};
use crate::database::{
    audit::{buat_batch, catat_audit, EntriAudit},
    models::{BuatDatasetInput, Dataset, UbahDatasetInput},
};

fn baris_ke_dataset(r: &rusqlite::Row) -> rusqlite::Result<Dataset> {
    Ok(Dataset {
        id:          r.get(0)?,
        name:        r.get(1)?,
        description: r.get(2)?,
        domain:      r.get(3)?,
        grain_desc:  r.get(4)?,
        period_type: r.get(5)?,
        owner:       r.get(6)?,
        source:      r.get(7)?,
        status:      r.get(8)?,
        created_at:  r.get(9)?,
        updated_at:  r.get(10)?,
    })
}

pub fn buat_dataset(conn: &mut Connection, input: BuatDatasetInput, oleh: &str) -> Result<Dataset, String> {
    let tx = conn.transaction().map_err(|e| format!("Gagal memulai transaksi: {e}"))?;
    let batch_id = buat_batch(&tx, "manual", Some("Buat dataset baru"), oleh)?;
    tx.execute(
        "INSERT INTO dataset (name, description, domain, grain_desc, period_type, owner, source)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![input.name, input.description, input.domain, input.grain_desc,
                input.period_type, input.owner, input.source],
    ).map_err(|e| format!("Gagal menyimpan dataset: {e}"))?;
    let id = tx.last_insert_rowid();
    catat_audit(&tx, &EntriAudit {
        entity_type: "dataset", entity_id: id, dataset_id: Some(id),
        action: "create", field: None, old_value: None,
        new_value: Some(&input.name), reason: None,
        batch_id: Some(batch_id), changed_by: oleh,
    })?;
    let dataset = tx.query_row(
        "SELECT id,name,description,domain,grain_desc,period_type,owner,source,status,created_at,updated_at
         FROM dataset WHERE id=?1",
        [id], baris_ke_dataset,
    ).map_err(|e| format!("Gagal membaca dataset setelah dibuat: {e}"))?;
    tx.commit().map_err(|e| format!("Gagal menyelesaikan transaksi: {e}"))?;
    Ok(dataset)
}

pub fn ambil_dataset(conn: &Connection, id: i64) -> Result<Dataset, String> {
    conn.query_row(
        "SELECT id,name,description,domain,grain_desc,period_type,owner,source,status,created_at,updated_at
         FROM dataset WHERE id=?1",
        [id], baris_ke_dataset,
    ).map_err(|e| format!("Dataset dengan id {id} tidak ditemukan: {e}"))
}

pub fn daftar_dataset(conn: &Connection) -> Result<Vec<Dataset>, String> {
    let mut stmt = conn.prepare(
        "SELECT id,name,description,domain,grain_desc,period_type,owner,source,status,created_at,updated_at
         FROM dataset ORDER BY name",
    ).map_err(|e| format!("Gagal menyiapkan kueri dataset: {e}"))?;
    let rows = stmt.query_map([], baris_ke_dataset)
        .map_err(|e| format!("Gagal membaca daftar dataset: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Gagal membaca dataset: {e}"))
}

pub fn ubah_dataset(conn: &mut Connection, id: i64, input: UbahDatasetInput, oleh: &str) -> Result<Dataset, String> {
    let lama = ambil_dataset(conn, id)?;
    let tx = conn.transaction().map_err(|e| format!("Gagal memulai transaksi: {e}"))?;
    let batch_id = buat_batch(&tx, "manual", input.reason.as_deref(), oleh)?;

    macro_rules! ubah_field {
        ($field:ident, $col:expr, $lama:expr) => {
            if let Some(ref baru) = input.$field {
                let baru_str = baru.as_str();
                let _lama_owned = $lama;
                let lama_val = _lama_owned.as_deref().unwrap_or("");
                if baru_str != lama_val {
                    tx.execute(
                        concat!("UPDATE dataset SET ", $col, "=?1, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?2"),
                        params![baru_str, id],
                    ).map_err(|e| format!("Gagal mengubah {}: {e}", $col))?;
                    catat_audit(&tx, &EntriAudit {
                        entity_type: "dataset", entity_id: id, dataset_id: Some(id),
                        action: "update", field: Some($col),
                        old_value: if lama_val.is_empty() { None } else { Some(lama_val) },
                        new_value: Some(baru_str),
                        reason: input.reason.as_deref(),
                        batch_id: Some(batch_id), changed_by: oleh,
                    })?;
                }
            }
        };
    }

    ubah_field!(name,        "name",        Some(lama.name.clone()));
    ubah_field!(description, "description", lama.description.clone());
    ubah_field!(domain,      "domain",      lama.domain.clone());
    ubah_field!(grain_desc,  "grain_desc",  lama.grain_desc.clone());
    ubah_field!(period_type, "period_type", Some(lama.period_type.clone()));
    ubah_field!(owner,       "owner",       lama.owner.clone());
    ubah_field!(source,      "source",      lama.source.clone());

    let dataset = tx.query_row(
        "SELECT id,name,description,domain,grain_desc,period_type,owner,source,status,created_at,updated_at
         FROM dataset WHERE id=?1",
        [id], baris_ke_dataset,
    ).map_err(|e| format!("Gagal membaca dataset setelah diubah: {e}"))?;
    tx.commit().map_err(|e| format!("Gagal menyelesaikan transaksi: {e}"))?;
    Ok(dataset)
}

pub fn hapus_dataset(conn: &mut Connection, id: i64, oleh: &str) -> Result<(), String> {
    let lama = ambil_dataset(conn, id)?;
    let tx = conn.transaction().map_err(|e| format!("Gagal memulai transaksi: {e}"))?;
    let batch_id = buat_batch(&tx, "manual", Some("Hapus dataset"), oleh)?;
    tx.execute("DELETE FROM dataset WHERE id=?1", [id])
        .map_err(|e| format!("Gagal menghapus dataset (mungkin masih memiliki data): {e}"))?;
    catat_audit(&tx, &EntriAudit {
        entity_type: "dataset", entity_id: id, dataset_id: Some(id),
        action: "delete", field: None,
        old_value: Some(&lama.name), new_value: None,
        reason: None, batch_id: Some(batch_id), changed_by: oleh,
    })?;
    tx.commit().map_err(|e| format!("Gagal menyelesaikan transaksi: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migrations;

    fn baru() -> Connection {
        let mut c = Connection::open_in_memory().unwrap();
        migrations::configure(&c).unwrap();
        migrations::run_migrations(&mut c).unwrap();
        c
    }

    fn input_pkk() -> BuatDatasetInput {
        BuatDatasetInput {
            name: "Data Pembinaan PKK".into(),
            description: Some("Data pembinaan PKK per desa".into()),
            domain: Some("pemberdayaan".into()),
            grain_desc: Some("Satu desa dalam satu tahun".into()),
            period_type: "tahunan".into(),
            owner: Some("DPMD".into()),
            source: None,
        }
    }

    #[test]
    fn buat_dan_ambil_dataset() {
        let mut c = baru();
        let d = buat_dataset(&mut c, input_pkk(), "uji").unwrap();
        assert_eq!(d.name, "Data Pembinaan PKK");
        assert_eq!(d.status, "draft");
        let d2 = ambil_dataset(&c, d.id).unwrap();
        assert_eq!(d.id, d2.id);
    }

    #[test]
    fn nama_duplikat_ditolak() {
        let mut c = baru();
        buat_dataset(&mut c, input_pkk(), "uji").unwrap();
        assert!(buat_dataset(&mut c, input_pkk(), "uji").is_err());
    }

    #[test]
    fn ubah_dataset_tercatat_di_audit() {
        let mut c = baru();
        let d = buat_dataset(&mut c, input_pkk(), "uji").unwrap();
        ubah_dataset(&mut c, d.id, UbahDatasetInput {
            name: Some("PKK Diubah".into()),
            description: None, domain: None, grain_desc: None,
            period_type: None, owner: None, source: None,
            reason: Some("uji ubah".into()),
        }, "uji").unwrap();
        let n: i64 = c.query_row(
            "SELECT count(*) FROM audit_log WHERE entity_id=?1 AND action='update'",
            [d.id], |r| r.get(0),
        ).unwrap();
        assert!(n > 0);
        let nama: String = c.query_row(
            "SELECT name FROM dataset WHERE id=?1", [d.id], |r| r.get(0),
        ).unwrap();
        assert_eq!(nama, "PKK Diubah");
    }

    #[test]
    fn hapus_dataset_kosong_berhasil() {
        let mut c = baru();
        let d = buat_dataset(&mut c, input_pkk(), "uji").unwrap();
        hapus_dataset(&mut c, d.id, "uji").unwrap();
        assert!(ambil_dataset(&c, d.id).is_err());
    }

    #[test]
    fn daftar_dataset_urut_nama() {
        let mut c = baru();
        buat_dataset(&mut c, BuatDatasetInput { name: "Zebra".into(), period_type: "tahunan".into(),
            description: None, domain: None, grain_desc: None, owner: None, source: None }, "uji").unwrap();
        buat_dataset(&mut c, BuatDatasetInput { name: "Apel".into(), period_type: "tahunan".into(),
            description: None, domain: None, grain_desc: None, owner: None, source: None }, "uji").unwrap();
        let list = daftar_dataset(&c).unwrap();
        assert_eq!(list[0].name, "Apel");
        assert_eq!(list[1].name, "Zebra");
    }
}
