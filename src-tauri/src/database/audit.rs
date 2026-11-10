// SPDX-License-Identifier: AGPL-3.0-only
//! Layanan audit TABULARIUM.
//!
//! Setiap penulisan ke database harus lewat fungsi di modul ini atau lewat
//! transaksi yang memanggil [catat_audit] secara eksplisit. Dengan begitu
//! audit trail tidak bisa dilewati, bukan hanya lewat konvensi.

use rusqlite::{params, Transaction};

/// Satu entri catatan audit yang akan ditulis.
pub struct EntriAudit<'a> {
    pub entity_type: &'a str,
    pub entity_id:   i64,
    pub dataset_id:  Option<i64>,
    pub action:      &'a str,   // create | update | delete | archive
    pub field:       Option<&'a str>,
    pub old_value:   Option<&'a str>,
    pub new_value:   Option<&'a str>,
    pub reason:      Option<&'a str>,
    pub batch_id:    Option<i64>,
    pub changed_by:  &'a str,
}

/// Tulis satu entri ke audit_log dalam transaksi yang sudah berjalan.
/// Pemanggil bertanggung jawab atas commit atau rollback transaksi.
pub fn catat_audit(tx: &Transaction, e: &EntriAudit) -> Result<(), String> {
    tx.execute(
        "INSERT INTO audit_log
            (entity_type, entity_id, dataset_id, action, field,
             old_value, new_value, reason, batch_id, changed_by)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            e.entity_type, e.entity_id, e.dataset_id, e.action, e.field,
            e.old_value, e.new_value, e.reason, e.batch_id, e.changed_by,
        ],
    )
    .map_err(|err| format!("Gagal mencatat audit ({} {}): {err}", e.entity_type, e.entity_id))?;
    Ok(())
}

/// Buat batch baru dan kembalikan id-nya.
/// Setiap impor atau sesi perubahan massal mendapat satu batch.
pub fn buat_batch(
    tx: &Transaction,
    source: &str,
    note: Option<&str>,
    changed_by: &str,
) -> Result<i64, String> {
    tx.execute(
        "INSERT INTO batch (source, note, created_by) VALUES (?1,?2,?3)",
        params![source, note, changed_by],
    )
    .map_err(|e| format!("Gagal membuat batch: {e}"))?;
    Ok(tx.last_insert_rowid())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migrations;

    fn baru() -> rusqlite::Connection {
        let mut c = rusqlite::Connection::open_in_memory().unwrap();
        migrations::configure(&c).unwrap();
        migrations::run_migrations(&mut c).unwrap();
        c
    }

    #[test]
    fn catat_audit_berhasil() {
        let mut c = baru();
        let tx = c.transaction().unwrap();
        catat_audit(&tx, &EntriAudit {
            entity_type: "dataset",
            entity_id:   1,
            dataset_id:  Some(1),
            action:      "create",
            field:       None,
            old_value:   None,
            new_value:   Some("PKK"),
            reason:      None,
            batch_id:    None,
            changed_by:  "uji",
        }).unwrap();
        tx.commit().unwrap();
        let n: i64 = c.query_row(
            "SELECT count(*) FROM audit_log WHERE entity_type='dataset'",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn audit_tidak_bisa_diubah_lewat_transaksi() {
        let mut c = baru();
        let tx = c.transaction().unwrap();
        catat_audit(&tx, &EntriAudit {
            entity_type: "dataset", entity_id: 1, dataset_id: None,
            action: "create", field: None, old_value: None,
            new_value: None, reason: None, batch_id: None, changed_by: "uji",
        }).unwrap();
        tx.commit().unwrap();
        assert!(c.execute("UPDATE audit_log SET reason='x'", []).is_err());
    }

    #[test]
    fn buat_batch_mengembalikan_id() {
        let mut c = baru();
        let tx = c.transaction().unwrap();
        let id = buat_batch(&tx, "manual", Some("uji batch"), "uji").unwrap();
        tx.commit().unwrap();
        assert!(id > 0);
    }
}
