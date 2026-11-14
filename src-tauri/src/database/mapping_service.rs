// SPDX-License-Identifier: AGPL-3.0-only
//! Pemetaan elemen TABULARIUM (variabel/indikator) ke kolom profil portal.
//! Pemetaan dibuat sekali dan dipakai ulang setiap periode.
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingInput {
    pub profile_id: String,
    pub bidang: String,
    pub sub_bidang: String,
    pub kategori: String,
    pub judul_kolom: String,
    pub satuan_unit: String,
    pub source_type: String,
    pub source_ref: String,
    pub aggregation: String,
    pub unit_factor: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapping {
    pub id: i64,
    pub profile_id: String,
    pub bidang: String,
    pub sub_bidang: String,
    pub kategori: String,
    pub judul_kolom: String,
    pub satuan_unit: String,
    pub source_type: String,
    pub source_ref: String,
    pub aggregation: String,
    pub unit_factor: f64,
    pub created_at: String,
    pub updated_at: String,
}

fn err(e: rusqlite::Error) -> String {
    format!("Kesalahan database: {e}")
}

fn validasi(input: &MappingInput) -> Result<(), String> {
    if input.profile_id.trim().is_empty() {
        return Err("profile_id wajib diisi.".into());
    }
    if input.judul_kolom.trim().is_empty() {
        return Err("judul_kolom wajib diisi.".into());
    }
    if input.source_ref.trim().is_empty() {
        return Err("source_ref wajib diisi.".into());
    }
    if !["variable", "indicator"].contains(&input.source_type.as_str()) {
        return Err("source_type harus 'variable' atau 'indicator'.".into());
    }
    if let Some(f) = input.unit_factor {
        if f <= 0.0 {
            return Err("unit_factor harus lebih dari nol.".into());
        }
    }
    Ok(())
}

pub fn buat(conn: &Connection, input: &MappingInput) -> Result<i64, String> {
    validasi(input)?;
    conn.execute(
        "INSERT INTO portal_mapping
             (profile_id, bidang, sub_bidang, kategori, judul_kolom, satuan_unit,
              source_type, source_ref, aggregation, unit_factor)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            input.profile_id.trim(),
            input.bidang.trim(),
            input.sub_bidang.trim(),
            input.kategori.trim(),
            input.judul_kolom.trim(),
            input.satuan_unit.trim(),
            input.source_type.trim(),
            input.source_ref.trim(),
            input.aggregation.trim(),
            input.unit_factor.unwrap_or(1.0),
        ],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            format!(
                "Pemetaan untuk '{}' di profil '{}' sudah ada.",
                input.judul_kolom, input.profile_id
            )
        } else {
            err(e)
        }
    })?;
    Ok(conn.last_insert_rowid())
}

pub fn ubah(conn: &Connection, id: i64, input: &MappingInput) -> Result<(), String> {
    validasi(input)?;
    let n = conn
        .execute(
            "UPDATE portal_mapping SET
                 profile_id=?1, bidang=?2, sub_bidang=?3, kategori=?4, judul_kolom=?5,
                 satuan_unit=?6, source_type=?7, source_ref=?8, aggregation=?9,
                 unit_factor=?10,
                 updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id=?11",
            params![
                input.profile_id.trim(),
                input.bidang.trim(),
                input.sub_bidang.trim(),
                input.kategori.trim(),
                input.judul_kolom.trim(),
                input.satuan_unit.trim(),
                input.source_type.trim(),
                input.source_ref.trim(),
                input.aggregation.trim(),
                input.unit_factor.unwrap_or(1.0),
                id,
            ],
        )
        .map_err(err)?;
    if n == 0 {
        return Err(format!("Pemetaan dengan ID {id} tidak ditemukan."));
    }
    Ok(())
}

pub fn hapus(conn: &Connection, id: i64) -> Result<(), String> {
    let n = conn
        .execute("DELETE FROM portal_mapping WHERE id=?1", params![id])
        .map_err(err)?;
    if n == 0 {
        return Err(format!("Pemetaan dengan ID {id} tidak ditemukan."));
    }
    Ok(())
}

pub fn daftar_profil(conn: &Connection, profile_id: &str) -> Result<Vec<Mapping>, String> {
    let mut st = conn
        .prepare(
            "SELECT id, profile_id, bidang, sub_bidang, kategori, judul_kolom,
                    satuan_unit, source_type, source_ref, aggregation, unit_factor,
                    created_at, updated_at
             FROM portal_mapping WHERE profile_id=?1
             ORDER BY bidang, sub_bidang, kategori, judul_kolom",
        )
        .map_err(err)?;
    let rows = st
        .query_map(params![profile_id], |r| {
            Ok(Mapping {
                id: r.get(0)?,
                profile_id: r.get(1)?,
                bidang: r.get(2)?,
                sub_bidang: r.get(3)?,
                kategori: r.get(4)?,
                judul_kolom: r.get(5)?,
                satuan_unit: r.get(6)?,
                source_type: r.get(7)?,
                source_ref: r.get(8)?,
                aggregation: r.get(9)?,
                unit_factor: r.get(10)?,
                created_at: r.get(11)?,
                updated_at: r.get(12)?,
            })
        })
        .map_err(err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(err)
}

pub fn cari_source(
    conn: &Connection,
    source_type: &str,
    source_ref: &str,
) -> Result<Vec<Mapping>, String> {
    let mut st = conn
        .prepare(
            "SELECT id, profile_id, bidang, sub_bidang, kategori, judul_kolom,
                    satuan_unit, source_type, source_ref, aggregation, unit_factor,
                    created_at, updated_at
             FROM portal_mapping WHERE source_type=?1 AND source_ref=?2
             ORDER BY profile_id, bidang",
        )
        .map_err(err)?;
    let rows = st
        .query_map(params![source_type, source_ref], |r| {
            Ok(Mapping {
                id: r.get(0)?,
                profile_id: r.get(1)?,
                bidang: r.get(2)?,
                sub_bidang: r.get(3)?,
                kategori: r.get(4)?,
                judul_kolom: r.get(5)?,
                satuan_unit: r.get(6)?,
                source_type: r.get(7)?,
                source_ref: r.get(8)?,
                aggregation: r.get(9)?,
                unit_factor: r.get(10)?,
                created_at: r.get(11)?,
                updated_at: r.get(12)?,
            })
        })
        .map_err(err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            include_str!("../../migrations/0001_init.sql")
        ).unwrap();
        conn.execute_batch(
            include_str!("../../migrations/0005_portal_mapping.sql")
        ).unwrap();
        conn
    }

    fn mapping_pkk() -> MappingInput {
        MappingInput {
            profile_id: "karangasem-sdi-sektoral".into(),
            bidang: "Pemberdayaan".into(),
            sub_bidang: "PKK".into(),
            kategori: "Pembinaan PKK".into(),
            judul_kolom: "Jumlah Kelompok PKK".into(),
            satuan_unit: "Kelompok".into(),
            source_type: "variable".into(),
            source_ref: "jumlah_kelompok".into(),
            aggregation: "sum_all_villages".into(),
            unit_factor: None,
        }
    }

    #[test]
    fn buat_dan_daftar() {
        let conn = db();
        let id = buat(&conn, &mapping_pkk()).unwrap();
        assert!(id > 0);
        let list = daftar_profil(&conn, "karangasem-sdi-sektoral").unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].source_ref, "jumlah_kelompok");
        assert!((list[0].unit_factor - 1.0).abs() < 1e-9);
    }

    #[test]
    fn duplikat_ditolak() {
        let conn = db();
        buat(&conn, &mapping_pkk()).unwrap();
        assert!(buat(&conn, &mapping_pkk()).unwrap_err().contains("sudah ada"));
    }

    #[test]
    fn ubah_dan_hapus() {
        let conn = db();
        let id = buat(&conn, &mapping_pkk()).unwrap();
        let mut baru = mapping_pkk();
        baru.source_ref = "jumlah_kelompok_v2".into();
        baru.unit_factor = Some(0.001);
        ubah(&conn, id, &baru).unwrap();
        let list = daftar_profil(&conn, "karangasem-sdi-sektoral").unwrap();
        assert_eq!(list[0].source_ref, "jumlah_kelompok_v2");
        assert!((list[0].unit_factor - 0.001).abs() < 1e-9);
        hapus(&conn, id).unwrap();
        assert!(daftar_profil(&conn, "karangasem-sdi-sektoral").unwrap().is_empty());
    }

    #[test]
    fn cari_per_source() {
        let conn = db();
        buat(&conn, &mapping_pkk()).unwrap();
        let mut ind = mapping_pkk();
        ind.judul_kolom = "Persen Binaan".into();
        ind.source_type = "indicator".into();
        ind.source_ref = "pkk_binaan_persen".into();
        buat(&conn, &ind).unwrap();
        let r = cari_source(&conn, "indicator", "pkk_binaan_persen").unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].judul_kolom, "Persen Binaan");
    }

    #[test]
    fn validasi_source_type_salah() {
        let conn = db();
        let mut m = mapping_pkk();
        m.source_type = "dataset".into();
        assert!(buat(&conn, &m).unwrap_err().contains("source_type"));
    }
}