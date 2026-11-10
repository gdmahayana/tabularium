// SPDX-License-Identifier: AGPL-3.0-only
//! Service variable — semua operasi tulis variable lewat fungsi ini.

use rusqlite::{params, Connection};
use crate::database::{
    audit::{buat_batch, catat_audit, EntriAudit},
    models::{BuatVariableInput, Variable, UbahVariableInput},
};

fn baris_ke_variable(r: &rusqlite::Row) -> rusqlite::Result<Variable> {
    Ok(Variable {
        id:          r.get(0)?,
        dataset_id:  r.get(1)?,
        name:        r.get(2)?,
        code:        r.get(3)?,
        concept:     r.get(4)?,
        definition:  r.get(5)?,
        data_type:   r.get(6)?,
        unit:        r.get(7)?,
        is_required: r.get::<_,i64>(8)? != 0,
        sort_order:  r.get(9)?,
        archived_at: r.get(10)?,
        created_at:  r.get(11)?,
        updated_at:  r.get(12)?,
    })
}

pub fn buat_variable(conn: &mut Connection, input: BuatVariableInput, oleh: &str) -> Result<Variable, String> {
    let tx = conn.transaction().map_err(|e| format!("Gagal memulai transaksi: {e}"))?;
    let batch_id = buat_batch(&tx, "manual", Some("Buat variable baru"), oleh)?;
    let sort_order: i64 = tx.query_row(
        "SELECT COALESCE(MAX(sort_order),0)+1 FROM variable WHERE dataset_id=?1",
        [input.dataset_id], |r| r.get(0),
    ).unwrap_or(1);
    let is_required = input.is_required.unwrap_or(false) as i64;
    tx.execute(
        "INSERT INTO variable (dataset_id,name,code,concept,definition,data_type,unit,is_required,sort_order)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![input.dataset_id, input.name, input.code, input.concept,
                input.definition, input.data_type, input.unit, is_required, sort_order],
    ).map_err(|e| format!("Gagal menyimpan variable: {e}"))?;
    let id = tx.last_insert_rowid();
    catat_audit(&tx, &EntriAudit {
        entity_type: "variable", entity_id: id, dataset_id: Some(input.dataset_id),
        action: "create", field: None, old_value: None,
        new_value: Some(&input.name), reason: None,
        batch_id: Some(batch_id), changed_by: oleh,
    })?;
    let variable = tx.query_row(
        "SELECT id,dataset_id,name,code,concept,definition,data_type,unit,is_required,sort_order,archived_at,created_at,updated_at
         FROM variable WHERE id=?1",
        [id], baris_ke_variable,
    ).map_err(|e| format!("Gagal membaca variable setelah dibuat: {e}"))?;
    tx.commit().map_err(|e| format!("Gagal menyelesaikan transaksi: {e}"))?;
    Ok(variable)
}

pub fn daftar_variable(conn: &Connection, dataset_id: i64) -> Result<Vec<Variable>, String> {
    let mut stmt = conn.prepare(
        "SELECT id,dataset_id,name,code,concept,definition,data_type,unit,is_required,sort_order,archived_at,created_at,updated_at
         FROM variable WHERE dataset_id=?1 AND archived_at IS NULL ORDER BY sort_order,id",
    ).map_err(|e| format!("Gagal menyiapkan kueri variable: {e}"))?;
    let rows = stmt.query_map([dataset_id], baris_ke_variable)
        .map_err(|e| format!("Gagal membaca daftar variable: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Gagal membaca variable: {e}"))
}

pub fn ubah_variable(conn: &mut Connection, id: i64, input: UbahVariableInput, oleh: &str) -> Result<Variable, String> {
    let lama: Variable = conn.query_row(
        "SELECT id,dataset_id,name,code,concept,definition,data_type,unit,is_required,sort_order,archived_at,created_at,updated_at
         FROM variable WHERE id=?1",
        [id], baris_ke_variable,
    ).map_err(|e| format!("Variable {id} tidak ditemukan: {e}"))?;
    let tx = conn.transaction().map_err(|e| format!("Gagal memulai transaksi: {e}"))?;
    let batch_id = buat_batch(&tx, "manual", input.reason.as_deref(), oleh)?;

    macro_rules! ubah_str {
        ($field:ident, $col:expr, $lama:expr) => {
            if let Some(ref baru) = input.$field {
                let baru_s = baru.as_str();
                let _lama_owned = $lama;
                let lama_s = _lama_owned.as_deref().unwrap_or("");
                if baru_s != lama_s {
                    tx.execute(
                        concat!("UPDATE variable SET ", $col, "=?1, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?2"),
                        params![baru_s, id],
                    ).map_err(|e| format!("Gagal mengubah {}: {e}", $col))?;
                    catat_audit(&tx, &EntriAudit {
                        entity_type: "variable", entity_id: id, dataset_id: Some(lama.dataset_id),
                        action: "update", field: Some($col),
                        old_value: if lama_s.is_empty() { None } else { Some(lama_s) },
                        new_value: Some(baru_s),
                        reason: input.reason.as_deref(),
                        batch_id: Some(batch_id), changed_by: oleh,
                    })?;
                }
            }
        };
    }

    ubah_str!(name,       "name",       Some(lama.name.clone()));
    ubah_str!(concept,    "concept",    lama.concept.clone());
    ubah_str!(definition, "definition", lama.definition.clone());
    ubah_str!(unit,       "unit",       lama.unit.clone());

    if let Some(req) = input.is_required {
        let req_i = req as i64;
        let lama_i = lama.is_required as i64;
        if req_i != lama_i {
            tx.execute(
                "UPDATE variable SET is_required=?1, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?2",
                params![req_i, id],
            ).map_err(|e| format!("Gagal mengubah is_required: {e}"))?;
            catat_audit(&tx, &EntriAudit {
                entity_type: "variable", entity_id: id, dataset_id: Some(lama.dataset_id),
                action: "update", field: Some("is_required"),
                old_value: Some(if lama.is_required { "true" } else { "false" }),
                new_value: Some(if req { "true" } else { "false" }),
                reason: input.reason.as_deref(),
                batch_id: Some(batch_id), changed_by: oleh,
            })?;
        }
    }

    let variable = tx.query_row(
        "SELECT id,dataset_id,name,code,concept,definition,data_type,unit,is_required,sort_order,archived_at,created_at,updated_at
         FROM variable WHERE id=?1",
        [id], baris_ke_variable,
    ).map_err(|e| format!("Gagal membaca variable setelah diubah: {e}"))?;
    tx.commit().map_err(|e| format!("Gagal menyelesaikan transaksi: {e}"))?;
    Ok(variable)
}

pub fn arsipkan_variable(conn: &mut Connection, id: i64, oleh: &str) -> Result<(), String> {
    let lama: Variable = conn.query_row(
        "SELECT id,dataset_id,name,code,concept,definition,data_type,unit,is_required,sort_order,archived_at,created_at,updated_at
         FROM variable WHERE id=?1",
        [id], baris_ke_variable,
    ).map_err(|e| format!("Variable {id} tidak ditemukan: {e}"))?;
    if lama.archived_at.is_some() {
        return Err(format!("Variable {id} sudah diarsipkan."));
    }
    let tx = conn.transaction().map_err(|e| format!("Gagal memulai transaksi: {e}"))?;
    let batch_id = buat_batch(&tx, "manual", Some("Arsipkan variable"), oleh)?;
    tx.execute(
        "UPDATE variable SET archived_at=strftime('%Y-%m-%dT%H:%M:%fZ','now'), updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",
        [id],
    ).map_err(|e| format!("Gagal mengarsipkan variable: {e}"))?;
    catat_audit(&tx, &EntriAudit {
        entity_type: "variable", entity_id: id, dataset_id: Some(lama.dataset_id),
        action: "archive", field: None,
        old_value: Some(&lama.name), new_value: None,
        reason: None, batch_id: Some(batch_id), changed_by: oleh,
    })?;
    tx.commit().map_err(|e| format!("Gagal menyelesaikan transaksi: {e}"))?;
    Ok(())
}

pub fn atur_urutan_variable(conn: &mut Connection, dataset_id: i64, urutan_id: Vec<i64>, oleh: &str) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| format!("Gagal memulai transaksi: {e}"))?;
    let batch_id = buat_batch(&tx, "manual", Some("Atur urutan variable"), oleh)?;
    for (i, var_id) in urutan_id.iter().enumerate() {
        tx.execute(
            "UPDATE variable SET sort_order=?1 WHERE id=?2 AND dataset_id=?3",
            params![i as i64 + 1, var_id, dataset_id],
        ).map_err(|e| format!("Gagal mengatur urutan variable {var_id}: {e}"))?;
    }
    catat_audit(&tx, &EntriAudit {
        entity_type: "dataset", entity_id: dataset_id, dataset_id: Some(dataset_id),
        action: "update", field: Some("variable_order"),
        old_value: None, new_value: None,
        reason: None, batch_id: Some(batch_id), changed_by: oleh,
    })?;
    tx.commit().map_err(|e| format!("Gagal menyelesaikan transaksi: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{migrations, dataset_service::buat_dataset, models::BuatDatasetInput};

    fn baru() -> Connection {
        let mut c = Connection::open_in_memory().unwrap();
        migrations::configure(&c).unwrap();
        migrations::run_migrations(&mut c).unwrap();
        c
    }

    fn dataset(c: &mut Connection) -> i64 {
        buat_dataset(c, BuatDatasetInput {
            name: "PKK".into(), period_type: "tahunan".into(),
            description: None, domain: None, grain_desc: None, owner: None, source: None,
        }, "uji").unwrap().id
    }

    fn var_input(ds: i64, kode: &str) -> BuatVariableInput {
        BuatVariableInput {
            dataset_id: ds, name: format!("Var {kode}"), code: kode.into(),
            concept: None, definition: None, data_type: "integer".into(),
            unit: Some("kelompok".into()), is_required: Some(true),
        }
    }

    #[test]
    fn buat_dan_daftar_variable() {
        let mut c = baru();
        let ds = dataset(&mut c);
        buat_variable(&mut c, var_input(ds, "a"), "uji").unwrap();
        buat_variable(&mut c, var_input(ds, "b"), "uji").unwrap();
        let list = daftar_variable(&c, ds).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].sort_order, 1);
        assert_eq!(list[1].sort_order, 2);
    }

    #[test]
    fn kode_duplikat_dalam_dataset_ditolak() {
        let mut c = baru();
        let ds = dataset(&mut c);
        buat_variable(&mut c, var_input(ds, "sama"), "uji").unwrap();
        assert!(buat_variable(&mut c, var_input(ds, "sama"), "uji").is_err());
    }

    #[test]
    fn arsipkan_menyembunyikan_dari_daftar() {
        let mut c = baru();
        let ds = dataset(&mut c);
        let v = buat_variable(&mut c, var_input(ds, "x"), "uji").unwrap();
        arsipkan_variable(&mut c, v.id, "uji").unwrap();
        assert!(daftar_variable(&c, ds).unwrap().is_empty());
    }

    #[test]
    fn ubah_variable_tercatat_di_audit() {
        let mut c = baru();
        let ds = dataset(&mut c);
        let v = buat_variable(&mut c, var_input(ds, "y"), "uji").unwrap();
        ubah_variable(&mut c, v.id, UbahVariableInput {
            name: Some("Nama Baru".into()), concept: None, definition: None,
            unit: None, is_required: None, reason: Some("uji ubah".into()),
        }, "uji").unwrap();
        let n: i64 = c.query_row(
            "SELECT count(*) FROM audit_log WHERE entity_type='variable' AND action='update'",
            [], |r| r.get(0),
        ).unwrap();
        assert!(n > 0);
    }
}
