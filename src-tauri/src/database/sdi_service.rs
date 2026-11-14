// SPDX-License-Identifier: AGPL-3.0-only
//! Lembar Input SDI v0 dan Input & Submission Registry.
//! Alur: susun lembar → ambil nilai → tandai sel → deteksi perubahan → catat pengajuan.
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::database::{
    indicator_service,
    mapping_service,
};

fn err(e: rusqlite::Error) -> String {
    format!("Kesalahan database: {e}")
}

// ── Sel Lembar ───────────────────────────────────────────────────────────────

/// Satu sel pada Lembar Input SDI, siap ditampilkan di layar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelLembar {
    pub profile_id: String,
    pub period_value: String,
    pub bidang: String,
    pub sub_bidang: String,
    pub kategori: String,
    pub judul_kolom: String,
    pub satuan_unit: String,
    pub source_type: String,
    pub source_ref: String,
    pub aggregation: String,
    pub unit_factor: f64,
    // nilai hasil perhitungan
    pub nilai: Option<f64>,
    pub nilai_tampil: Option<String>,   // nilai * unit_factor, diformat
    pub value_status: String,           // terisi / kosong / tidak_berlaku
    pub is_stale: bool,
    // status penandaan pengguna
    pub sudah_diinput: bool,
    pub perlu_diperbarui: bool,
    pub input_at: Option<String>,
    pub catatan: Option<String>,
}

/// Susun Lembar Input SDI untuk satu profil dan satu periode.
/// Nilai diambil dari indicator_result (untuk indikator) atau observation (untuk variabel).
pub fn susun_lembar(
    conn: &Connection,
    profile_id: &str,
    period_value: &str,
    region_id: i64,
) -> Result<Vec<SelLembar>, String> {
    let mappings = mapping_service::daftar_profil(conn, profile_id)?;
    let mut lembar = vec![];

    for m in &mappings {
        let (nilai, value_status, is_stale) = match m.source_type.as_str() {
            "indicator" => nilai_indikator(conn, &m.source_ref, period_value, region_id)?,
            "variable"  => nilai_variabel(conn, &m.source_ref, period_value, region_id)?,
            _           => (None, "kosong".to_string(), false),
        };

        let nilai_tampil = nilai.map(|v| {
            let hasil = v * m.unit_factor;
            if (hasil - hasil.round()).abs() < 1e-6 {
                format!("{}", hasil as i64)
            } else {
                format!("{:.4}", hasil)
            }
        });

        // Status penandaan pengguna
        let (sudah_diinput, perlu_diperbarui, input_at, catatan) =
            ambil_status_sel(conn, profile_id, period_value, &m.bidang,
                             &m.sub_bidang, &m.kategori, &m.judul_kolom)?;

        lembar.push(SelLembar {
            profile_id: profile_id.to_string(),
            period_value: period_value.to_string(),
            bidang: m.bidang.clone(),
            sub_bidang: m.sub_bidang.clone(),
            kategori: m.kategori.clone(),
            judul_kolom: m.judul_kolom.clone(),
            satuan_unit: m.satuan_unit.clone(),
            source_type: m.source_type.clone(),
            source_ref: m.source_ref.clone(),
            aggregation: m.aggregation.clone(),
            unit_factor: m.unit_factor,
            nilai,
            nilai_tampil,
            value_status,
            is_stale,
            sudah_diinput,
            perlu_diperbarui,
            input_at,
            catatan,
        });
    }
    Ok(lembar)
}

fn nilai_indikator(
    conn: &Connection,
    source_ref: &str,
    period_value: &str,
    region_id: i64,
) -> Result<(Option<f64>, String, bool), String> {
    // Cari indicator_id dari code
    let ind_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM indicator WHERE code = ?1 AND archived_at IS NULL",
            params![source_ref],
            |r| r.get(0),
        )
        .optional()
        .map_err(err)?;

    let Some(ind_id) = ind_id else {
        return Ok((None, "kosong".to_string(), false));
    };

    let row: Option<(Option<f64>, String, i64)> = conn
        .query_row(
            "SELECT value_num, value_status, is_stale FROM indicator_result
             WHERE indicator_id = ?1 AND region_id = ?2 AND period_value = ?3",
            params![ind_id, region_id, period_value],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(err)?;

    match row {
        Some((v, s, stale)) => Ok((v, s, stale != 0)),
        None => Ok((None, "kosong".to_string(), false)),
    }
}

fn nilai_variabel(
    conn: &Connection,
    source_ref: &str,
    period_value: &str,
    region_id: i64,
) -> Result<(Option<f64>, String, bool), String> {
    // Agregasi rekursif: jumlah seluruh sub-wilayah
    let total: Option<f64> = conn
        .query_row(
            "WITH RECURSIVE sub(id) AS (
                 SELECT ?1
                 UNION ALL
                 SELECT u.id FROM ref_unit u JOIN sub s ON u.parent_id = s.id
             )
             SELECT SUM(o.value_num) FROM observation o
             JOIN variable v ON v.id = o.variable_id
             WHERE v.code = ?2
               AND o.period_value = ?3
               AND o.value_status = 'terisi'
               AND o.value_num IS NOT NULL
               AND o.unit_id IN (SELECT id FROM sub)",
            params![region_id, source_ref, period_value],
            |r| r.get(0),
        )
        .map_err(err)?;

    match total {
        Some(v) => Ok((Some(v), "terisi".to_string(), false)),
        None    => Ok((None, "kosong".to_string(), false)),
    }
}

fn ambil_status_sel(
    conn: &Connection,
    profile_id: &str,
    period_value: &str,
    bidang: &str,
    sub_bidang: &str,
    kategori: &str,
    judul_kolom: &str,
) -> Result<(bool, bool, Option<String>, Option<String>), String> {
    let row: Option<(i64, i64, Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT sudah_diinput, perlu_diperbarui, input_at, catatan
             FROM sdi_cell_status
             WHERE profile_id=?1 AND period_value=?2 AND bidang=?3
               AND sub_bidang=?4 AND kategori=?5 AND judul_kolom=?6",
            params![profile_id, period_value, bidang, sub_bidang, kategori, judul_kolom],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()
        .map_err(err)?;

    match row {
        Some((s, p, at, cat)) => Ok((s != 0, p != 0, at, cat)),
        None => Ok((false, false, None, None)),
    }
}

// ── Tandai Sel ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TandaiSelInput {
    pub profile_id: String,
    pub period_value: String,
    pub bidang: String,
    pub sub_bidang: String,
    pub kategori: String,
    pub judul_kolom: String,
    pub nilai_saat_input: Option<f64>,
    pub catatan: Option<String>,
    pub operator: String,
}

pub fn tandai_sel(conn: &Connection, input: &TandaiSelInput) -> Result<(), String> {
    conn.execute(
        "INSERT INTO sdi_cell_status
             (profile_id, period_value, bidang, sub_bidang, kategori, judul_kolom,
              sudah_diinput, input_at, input_oleh, catatan, nilai_saat_input,
              perlu_diperbarui)
         VALUES (?1,?2,?3,?4,?5,?6, 1, strftime('%Y-%m-%dT%H:%M:%fZ','now'),
                 ?7, ?8, ?9, 0)
         ON CONFLICT(profile_id,period_value,bidang,sub_bidang,kategori,judul_kolom)
         DO UPDATE SET
             sudah_diinput    = 1,
             input_at         = strftime('%Y-%m-%dT%H:%M:%fZ','now'),
             input_oleh       = excluded.input_oleh,
             catatan          = excluded.catatan,
             nilai_saat_input = excluded.nilai_saat_input,
             perlu_diperbarui = 0,
             updated_at       = strftime('%Y-%m-%dT%H:%M:%fZ','now')",
        params![
            input.profile_id, input.period_value,
            input.bidang, input.sub_bidang, input.kategori, input.judul_kolom,
            input.operator, input.catatan, input.nilai_saat_input,
        ],
    )
    .map_err(err)?;
    Ok(())
}

pub fn batal_tandai_sel(
    conn: &Connection,
    profile_id: &str,
    period_value: &str,
    bidang: &str,
    sub_bidang: &str,
    kategori: &str,
    judul_kolom: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE sdi_cell_status SET sudah_diinput=0, perlu_diperbarui=0,
             updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE profile_id=?1 AND period_value=?2 AND bidang=?3
           AND sub_bidang=?4 AND kategori=?5 AND judul_kolom=?6",
        params![profile_id, period_value, bidang, sub_bidang, kategori, judul_kolom],
    )
    .map_err(err)?;
    Ok(())
}

/// Tandai sel perlu diperbarui di portal (dipanggil saat nilai sumber berubah).
pub fn tandai_perlu_diperbarui(
    conn: &Connection,
    profile_id: &str,
    period_value: &str,
    source_type: &str,
    source_ref: &str,
) -> Result<usize, String> {
    let n = conn
        .execute(
            "UPDATE sdi_cell_status SET perlu_diperbarui=1,
                 updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE sudah_diinput=1 AND perlu_diperbarui=0
               AND profile_id=?1 AND period_value=?2
               AND (bidang, sub_bidang, kategori, judul_kolom) IN (
                   SELECT bidang, sub_bidang, kategori, judul_kolom
                   FROM portal_mapping
                   WHERE source_type=?3 AND source_ref=?4 AND profile_id=?1
               )",
            params![profile_id, period_value, source_type, source_ref],
        )
        .map_err(err)?;
    Ok(n)
}

// ── Ringkasan Lembar ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RingkasanLembar {
    pub total_sel: usize,
    pub sudah_diinput: usize,
    pub perlu_diperbarui: usize,
    pub belum_diinput: usize,
    pub nilai_kosong: usize,
}

pub fn ringkasan(lembar: &[SelLembar]) -> RingkasanLembar {
    RingkasanLembar {
        total_sel: lembar.len(),
        sudah_diinput: lembar.iter().filter(|s| s.sudah_diinput).count(),
        perlu_diperbarui: lembar.iter().filter(|s| s.perlu_diperbarui).count(),
        belum_diinput: lembar.iter().filter(|s| !s.sudah_diinput).count(),
        nilai_kosong: lembar.iter().filter(|s| s.value_status == "kosong").count(),
    }
}

// ── Submission Registry ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionInput {
    pub profile_id: String,
    pub period_value: String,
    pub status: String,
    pub catatan: Option<String>,
    pub bukti_ref: Option<String>,
    pub operator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Submission {
    pub id: i64,
    pub profile_id: String,
    pub period_value: String,
    pub status: String,
    pub catatan: Option<String>,
    pub bukti_ref: Option<String>,
    pub operator: String,
    pub created_at: String,
    pub updated_at: String,
}

const STATUS_VALID: [&str; 4] =
    ["disiapkan", "diinput_draft", "diajukan", "diterima"];

pub fn catat_submission(
    conn: &Connection,
    input: &SubmissionInput,
) -> Result<i64, String> {
    if !STATUS_VALID.contains(&input.status.as_str()) {
        return Err(format!(
            "Status '{}' tidak valid. Pilihan: {}.",
            input.status,
            STATUS_VALID.join(", ")
        ));
    }
    conn.execute(
        "INSERT INTO sdi_submission
             (profile_id, period_value, status, catatan, bukti_ref, operator)
         VALUES (?1,?2,?3,?4,?5,?6)",
        params![
            input.profile_id, input.period_value, input.status,
            input.catatan, input.bukti_ref, input.operator,
        ],
    )
    .map_err(err)?;
    Ok(conn.last_insert_rowid())
}

pub fn daftar_submission(
    conn: &Connection,
    profile_id: &str,
    period_value: &str,
) -> Result<Vec<Submission>, String> {
    let mut st = conn
        .prepare(
            "SELECT id, profile_id, period_value, status, catatan, bukti_ref,
                    operator, created_at, updated_at
             FROM sdi_submission
             WHERE profile_id=?1 AND period_value=?2
             ORDER BY created_at DESC",
        )
        .map_err(err)?;
    let rows = st
        .query_map(params![profile_id, period_value], |r| {
            Ok(Submission {
                id: r.get(0)?,
                profile_id: r.get(1)?,
                period_value: r.get(2)?,
                status: r.get(3)?,
                catatan: r.get(4)?,
                bukti_ref: r.get(5)?,
                operator: r.get(6)?,
                created_at: r.get(7)?,
                updated_at: r.get(8)?,
            })
        })
        .map_err(err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::mapping_service::{self, MappingInput};
    use crate::database::indicator_service::{self, IndicatorInput, IndicatorVariabel};

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../../migrations/0001_init.sql")).unwrap();
        conn.execute_batch(include_str!("../../migrations/0004_indicator.sql")).unwrap();
        conn.execute_batch(include_str!("../../migrations/0005_portal_mapping.sql")).unwrap();
        conn.execute_batch(include_str!("../../migrations/0006_sdi_workbench.sql")).unwrap();
        conn.execute_batch(
            "INSERT INTO dataset (id, name, period_type)
             VALUES (1, 'Data PKK', 'tahunan');
             INSERT INTO ref_unit (id, name, level, parent_id) VALUES
                 (1, 'Karangasem', 'kabupaten', NULL),
                 (2, 'Rendang',    'kecamatan', 1),
                 (3, 'Desa A',     'desa',      2),
                 (4, 'Desa B',     'desa',      2);
             INSERT INTO variable (id, dataset_id, name, code, data_type) VALUES
                 (1, 1, 'Jumlah kelompok', 'jumlah_kelompok', 'integer'),
                 (2, 1, 'Kelompok binaan', 'jumlah_binaan',   'integer');"
        ).unwrap();
        conn
    }

    fn mapping_kelompok(conn: &Connection) {
        mapping_service::buat(conn, &MappingInput {
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
        }).unwrap();
    }

    fn mapping_indikator(conn: &Connection) {
        mapping_service::buat(conn, &MappingInput {
            profile_id: "karangasem-sdi-sektoral".into(),
            bidang: "Pemberdayaan".into(),
            sub_bidang: "PKK".into(),
            kategori: "Pembinaan PKK".into(),
            judul_kolom: "Persen Binaan".into(),
            satuan_unit: "Persen".into(),
            source_type: "indicator".into(),
            source_ref: "pkk_binaan_persen".into(),
            aggregation: "ratio_of_sums".into(),
            unit_factor: None,
        }).unwrap();
    }

    fn isi_obs(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO observation (dataset_id, variable_id, unit_id, period_value, value_num)
             VALUES (1,1,3,'2026',10),(1,1,4,'2026',20),(1,2,3,'2026',5),(1,2,4,'2026',15);"
        ).unwrap();
    }

    fn buat_indikator(conn: &Connection) -> i64 {
        indicator_service::buat(conn, &IndicatorInput {
            code: "pkk_binaan_persen".into(),
            name: "Persen PKK Binaan".into(),
            definition: None, interpretation: None,
            unit: Some("persen".into()),
            formula: "ratio(binaan, kelompok) * 100".into(),
            aggregation_rule: None,
            variables: vec![
                IndicatorVariabel { variable_id: 2, alias: "binaan".into() },
                IndicatorVariabel { variable_id: 1, alias: "kelompok".into() },
            ],
        }).unwrap()
    }

    #[test]
    fn lembar_variabel_tanpa_data() {
        let conn = db();
        mapping_kelompok(&conn);
        let lembar = susun_lembar(&conn, "karangasem-sdi-sektoral", "2026", 1).unwrap();
        assert_eq!(lembar.len(), 1);
        assert_eq!(lembar[0].value_status, "kosong");
        assert!(!lembar[0].sudah_diinput);
    }

    #[test]
    fn lembar_variabel_dengan_data() {
        let conn = db();
        mapping_kelompok(&conn);
        isi_obs(&conn);
        let lembar = susun_lembar(&conn, "karangasem-sdi-sektoral", "2026", 1).unwrap();
        assert_eq!(lembar[0].nilai, Some(30.0));
        assert_eq!(lembar[0].nilai_tampil.as_deref(), Some("30"));
        assert_eq!(lembar[0].value_status, "terisi");
    }

    #[test]
    fn lembar_indikator_dengan_hasil() {
        let conn = db();
        mapping_indikator(&conn);
        isi_obs(&conn);
        let ind_id = buat_indikator(&conn);
        indicator_service::hitung(&conn, ind_id, "2026").unwrap();
        let lembar = susun_lembar(&conn, "karangasem-sdi-sektoral", "2026", 1).unwrap();
        assert_eq!(lembar.len(), 1);
        let kec = 20.0 / 30.0 * 100.0;
        assert!((lembar[0].nilai.unwrap() - kec).abs() < 1e-6);
        assert!(!lembar[0].is_stale);
    }

    #[test]
    fn tandai_dan_batal_tandai() {
        let conn = db();
        mapping_kelompok(&conn);
        isi_obs(&conn);
        let lembar = susun_lembar(&conn, "karangasem-sdi-sektoral", "2026", 1).unwrap();
        assert!(!lembar[0].sudah_diinput);

        tandai_sel(&conn, &TandaiSelInput {
            profile_id: "karangasem-sdi-sektoral".into(),
            period_value: "2026".into(),
            bidang: "Pemberdayaan".into(),
            sub_bidang: "PKK".into(),
            kategori: "Pembinaan PKK".into(),
            judul_kolom: "Jumlah Kelompok PKK".into(),
            nilai_saat_input: Some(30.0),
            catatan: Some("sudah diinput manual".into()),
            operator: "pengguna".into(),
        }).unwrap();

        let lembar2 = susun_lembar(&conn, "karangasem-sdi-sektoral", "2026", 1).unwrap();
        assert!(lembar2[0].sudah_diinput);
        assert!(!lembar2[0].perlu_diperbarui);

        batal_tandai_sel(&conn, "karangasem-sdi-sektoral", "2026",
            "Pemberdayaan", "PKK", "Pembinaan PKK", "Jumlah Kelompok PKK").unwrap();
        let lembar3 = susun_lembar(&conn, "karangasem-sdi-sektoral", "2026", 1).unwrap();
        assert!(!lembar3[0].sudah_diinput);
    }

    #[test]
    fn perlu_diperbarui_setelah_sumber_berubah() {
        let conn = db();
        mapping_kelompok(&conn);
        isi_obs(&conn);
        tandai_sel(&conn, &TandaiSelInput {
            profile_id: "karangasem-sdi-sektoral".into(),
            period_value: "2026".into(),
            bidang: "Pemberdayaan".into(),
            sub_bidang: "PKK".into(),
            kategori: "Pembinaan PKK".into(),
            judul_kolom: "Jumlah Kelompok PKK".into(),
            nilai_saat_input: Some(30.0),
            catatan: None,
            operator: "pengguna".into(),
        }).unwrap();

        let n = tandai_perlu_diperbarui(
            &conn, "karangasem-sdi-sektoral", "2026", "variable", "jumlah_kelompok"
        ).unwrap();
        assert_eq!(n, 1);

        let lembar = susun_lembar(&conn, "karangasem-sdi-sektoral", "2026", 1).unwrap();
        assert!(lembar[0].perlu_diperbarui);
    }

    #[test]
    fn ringkasan_lembar() {
        let conn = db();
        mapping_kelompok(&conn);
        isi_obs(&conn);
        let lembar = susun_lembar(&conn, "karangasem-sdi-sektoral", "2026", 1).unwrap();
        let r = ringkasan(&lembar);
        assert_eq!(r.total_sel, 1);
        assert_eq!(r.belum_diinput, 1);
        assert_eq!(r.sudah_diinput, 0);
    }

    #[test]
    fn submission_registry() {
        let conn = db();
        let id = catat_submission(&conn, &SubmissionInput {
            profile_id: "karangasem-sdi-sektoral".into(),
            period_value: "2026".into(),
            status: "diinput_draft".into(),
            catatan: Some("laporan PKK 2026".into()),
            bukti_ref: None,
            operator: "pengguna".into(),
        }).unwrap();
        assert!(id > 0);

        let list = daftar_submission(&conn, "karangasem-sdi-sektoral", "2026").unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].status, "diinput_draft");

        // status tidak valid ditolak
        assert!(catat_submission(&conn, &SubmissionInput {
            profile_id: "x".into(), period_value: "2026".into(),
            status: "selesai".into(), catatan: None, bukti_ref: None,
            operator: "pengguna".into(),
        }).unwrap_err().contains("tidak valid"));
    }
}