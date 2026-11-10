// SPDX-License-Identifier: AGPL-3.0-only
//! Service wilayah (ref_unit) — impor, query, validasi hierarki.

use rusqlite::{params, Connection};
use crate::database::{
    audit::{buat_batch, catat_audit, EntriAudit},
    models::RefUnit,
};

fn baris_ke_unit(r: &rusqlite::Row) -> rusqlite::Result<RefUnit> {
    Ok(RefUnit {
        id:        r.get(0)?,
        code:      r.get(1)?,
        name:      r.get(2)?,
        level:     r.get(3)?,
        parent_id: r.get(4)?,
    })
}

pub fn ambil_unit(conn: &Connection, id: i64) -> Result<RefUnit, String> {
    conn.query_row(
        "SELECT id,code,name,level,parent_id FROM ref_unit WHERE id=?1",
        [id], baris_ke_unit,
    ).map_err(|e| format!("Wilayah {id} tidak ditemukan: {e}"))
}

pub fn daftar_unit(conn: &Connection, level: &str) -> Result<Vec<RefUnit>, String> {
    let mut stmt = conn.prepare(
        "SELECT id,code,name,level,parent_id FROM ref_unit WHERE level=?1 ORDER BY name",
    ).map_err(|e| format!("Gagal menyiapkan kueri wilayah: {e}"))?;
    let rows = stmt.query_map([level], baris_ke_unit)
        .map_err(|e| format!("Gagal membaca daftar wilayah: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Gagal membaca wilayah: {e}"))
}

pub fn daftar_anak(conn: &Connection, parent_id: i64) -> Result<Vec<RefUnit>, String> {
    let mut stmt = conn.prepare(
        "SELECT id,code,name,level,parent_id FROM ref_unit WHERE parent_id=?1 ORDER BY name",
    ).map_err(|e| format!("Gagal menyiapkan kueri anak wilayah: {e}"))?;
    let rows = stmt.query_map([parent_id], baris_ke_unit)
        .map_err(|e| format!("Gagal membaca anak wilayah: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Gagal membaca anak wilayah: {e}"))
}

fn level_anak(level_induk: &str) -> Option<&'static str> {
    match level_induk {
        "provinsi"  => Some("kabupaten"),
        "kabupaten" => Some("kecamatan"),
        "kecamatan" => Some("desa"),
        _           => None,
    }
}

#[derive(Debug)]
pub struct BarisWilayah {
    pub code:       Option<String>,
    pub name:       String,
    pub level:      String,
    pub parent_id:  Option<i64>,
    pub valid_from: Option<String>,
    pub valid_to:   Option<String>,
}

pub fn impor_wilayah(
    conn: &mut Connection,
    baris_list: Vec<BarisWilayah>,
    oleh: &str,
) -> Result<Vec<i64>, String> {
    let level_dikenal = ["provinsi", "kabupaten", "kecamatan", "desa"];
    for (i, b) in baris_list.iter().enumerate() {
        if !level_dikenal.contains(&b.level.as_str()) {
            return Err(format!("Baris {}: level '{}' tidak dikenal.", i + 1, b.level));
        }
        if b.name.trim().is_empty() {
            return Err(format!("Baris {}: nama wilayah tidak boleh kosong.", i + 1));
        }
    }

    let tx = conn.transaction().map_err(|e| format!("Gagal memulai transaksi: {e}"))?;
    let batch_id = buat_batch(&tx, "impor", Some("Impor wilayah CSV"), oleh)?;
    let mut id_baru = Vec::new();

    for (i, b) in baris_list.iter().enumerate() {
        if let Some(pid) = b.parent_id {
            let induk: Option<String> = tx.query_row(
                "SELECT level FROM ref_unit WHERE id=?1", [pid], |r| r.get(0),
            ).ok();
            match induk {
                None => return Err(format!("Baris {}: parent_id {pid} tidak ditemukan.", i + 1)),
                Some(lv) => {
                    if let Some(diharapkan) = level_anak(&lv) {
                        if diharapkan != b.level {
                            return Err(format!(
                                "Baris {}: level '{}' tidak sesuai untuk anak dari level '{lv}' (diharapkan '{diharapkan}').",
                                i + 1, b.level
                            ));
                        }
                    }
                }
            }
        } else if b.level != "provinsi" && b.level != "kabupaten" {
            return Err(format!("Baris {}: level '{}' harus memiliki parent_id.", i + 1, b.level));
        }

        tx.execute(
            "INSERT INTO ref_unit (code,name,level,parent_id,valid_from,valid_to)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![b.code, b.name, b.level, b.parent_id, b.valid_from, b.valid_to],
        ).map_err(|e| format!("Baris {}: gagal menyimpan '{}': {e}", i + 1, b.name))?;

        let id = tx.last_insert_rowid();
        id_baru.push(id);

        catat_audit(&tx, &EntriAudit {
            entity_type: "ref_unit", entity_id: id, dataset_id: None,
            action: "create", field: None, old_value: None,
            new_value: Some(&b.name), reason: None,
            batch_id: Some(batch_id), changed_by: oleh,
        })?;
    }

    tx.commit().map_err(|e| format!("Gagal menyelesaikan transaksi: {e}"))?;
    Ok(id_baru)
}

pub fn parse_csv_wilayah(isi_csv: &str) -> Result<Vec<BarisWilayah>, String> {
    let mut hasil = Vec::new();
    let mut baris_iter = isi_csv.lines();

    let header = baris_iter.next()
        .ok_or("CSV kosong")?
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .collect::<Vec<_>>();

    let idx = |nama: &str| -> Result<usize, String> {
        header.iter().position(|h| h == nama)
            .ok_or_else(|| format!("Kolom '{nama}' tidak ditemukan di header CSV."))
    };

    let i_code      = idx("code")?;
    let i_name      = idx("name")?;
    let i_level     = idx("level")?;
    let i_parent_id = idx("parent_id")?;
    let i_vfrom     = idx("valid_from").ok();
    let i_vto       = idx("valid_to").ok();

    for (n, baris) in baris_iter.enumerate() {
        let baris = baris.trim();
        if baris.is_empty() { continue; }
        let kolom: Vec<&str> = baris.split(',').collect();
        let ambil = |i: usize| kolom.get(i).copied().unwrap_or("").trim();

        let name = ambil(i_name).to_string();
        if name.is_empty() {
            return Err(format!("Baris {}: kolom 'name' kosong.", n + 2));
        }
        let str_opt = |s: &str| if s.is_empty() { None } else { Some(s.to_string()) };

        let parent_id = match ambil(i_parent_id) {
            "" => None,
            s  => Some(s.parse::<i64>().map_err(|_|
                format!("Baris {}: parent_id '{}' bukan angka.", n + 2, s))?),
        };

        hasil.push(BarisWilayah {
            code:       str_opt(ambil(i_code)),
            name,
            level:      ambil(i_level).to_string(),
            parent_id,
            valid_from: i_vfrom.and_then(|i| str_opt(ambil(i))),
            valid_to:   i_vto.and_then(|i| str_opt(ambil(i))),
        });
    }
    Ok(hasil)
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

    fn seed(c: &mut Connection) -> (i64, i64, i64) {
        let prov = impor_wilayah(c, vec![
            BarisWilayah { code: Some("51".into()), name: "Bali".into(),
                level: "provinsi".into(), parent_id: None, valid_from: None, valid_to: None },
        ], "uji").unwrap();
        let kab = impor_wilayah(c, vec![
            BarisWilayah { code: Some("5107".into()), name: "Karangasem".into(),
                level: "kabupaten".into(), parent_id: Some(prov[0]), valid_from: None, valid_to: None },
        ], "uji").unwrap();
        let kec = impor_wilayah(c, vec![
            BarisWilayah { code: Some("510701".into()), name: "Rendang".into(),
                level: "kecamatan".into(), parent_id: Some(kab[0]), valid_from: None, valid_to: None },
        ], "uji").unwrap();
        (prov[0], kab[0], kec[0])
    }

    #[test]
    fn impor_hierarki_berhasil() {
        let mut c = baru();
        let (_, id_kab, id_kec) = seed(&mut c);
        let ids = impor_wilayah(&mut c, vec![
            BarisWilayah { code: None, name: "Desa Pempatan".into(),
                level: "desa".into(), parent_id: Some(id_kec), valid_from: None, valid_to: None },
        ], "uji").unwrap();
        let desa = ambil_unit(&c, ids[0]).unwrap();
        assert_eq!(desa.name, "Desa Pempatan");
        let anak = daftar_anak(&c, id_kab).unwrap();
        assert_eq!(anak.len(), 1);
        assert_eq!(anak[0].name, "Rendang");
    }

    #[test]
    fn level_tidak_sesuai_ditolak() {
        let mut c = baru();
        let (_, id_kab, _) = seed(&mut c);
        let err = impor_wilayah(&mut c, vec![
            BarisWilayah { code: None, name: "Salah".into(),
                level: "desa".into(), parent_id: Some(id_kab), valid_from: None, valid_to: None },
        ], "uji").unwrap_err();
        assert!(err.contains("tidak sesuai"), "{err}");
    }

    #[test]
    fn parent_tidak_ada_ditolak() {
        let mut c = baru();
        let err = impor_wilayah(&mut c, vec![
            BarisWilayah { code: None, name: "Hantu".into(),
                level: "kecamatan".into(), parent_id: Some(9999), valid_from: None, valid_to: None },
        ], "uji").unwrap_err();
        assert!(err.contains("tidak ditemukan"), "{err}");
    }

    #[test]
    fn kecamatan_tanpa_parent_ditolak() {
        let mut c = baru();
        let err = impor_wilayah(&mut c, vec![
            BarisWilayah { code: None, name: "Tanpa Induk".into(),
                level: "kecamatan".into(), parent_id: None, valid_from: None, valid_to: None },
        ], "uji").unwrap_err();
        assert!(err.contains("harus memiliki parent_id"), "{err}");
    }

    #[test]
    fn daftar_unit_per_level_urut_nama() {
        let mut c = baru();
        let (_, id_kab, _) = seed(&mut c);
        impor_wilayah(&mut c, vec![
            BarisWilayah { code: None, name: "Sidemen".into(),
                level: "kecamatan".into(), parent_id: Some(id_kab), valid_from: None, valid_to: None },
        ], "uji").unwrap();
        let list = daftar_unit(&c, "kecamatan").unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "Rendang");
        assert_eq!(list[1].name, "Sidemen");
    }

    #[test]
    fn parse_csv_valid() {
        let csv = "code,name,level,parent_id,valid_from,valid_to\n\
                   51,Bali,provinsi,,,\n\
                   5107,Karangasem,kabupaten,,,\n\
                   510701,Rendang,kecamatan,,,";
        let baris = parse_csv_wilayah(csv).unwrap();
        assert_eq!(baris.len(), 3);
        assert_eq!(baris[2].name, "Rendang");
        assert_eq!(baris[2].level, "kecamatan");
    }

    #[test]
    fn parse_csv_kolom_hilang_ditolak() {
        let err = parse_csv_wilayah("code,name\n51,Bali").unwrap_err();
        assert!(err.contains("level"), "{err}");
    }
}