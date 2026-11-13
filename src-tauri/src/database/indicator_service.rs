// SPDX-License-Identifier: AGPL-3.0-only
//! Indicator Registry, penghitungan indikator, dan penandaan hasil usang.
use crate::expression::{self, Context, Expr};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const FUNGSI_DIKENAL: [&str; 5] = ["sum", "avg", "count", "ratio", "if"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorVariabel {
    pub variable_id: i64,
    pub alias: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorInput {
    pub code: String,
    pub name: String,
    pub definition: Option<String>,
    pub interpretation: Option<String>,
    pub unit: Option<String>,
    pub formula: String,
    pub aggregation_rule: Option<String>,
    pub variables: Vec<IndicatorVariabel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Indicator {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub definition: Option<String>,
    pub interpretation: Option<String>,
    pub unit: Option<String>,
    pub formula: String,
    pub aggregation_rule: String,
    pub method_version: i64,
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub variables: Vec<IndicatorVariabel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasilIndikator {
    pub indicator_id: i64,
    pub region_id: i64,
    pub region_name: String,
    pub region_level: String,
    pub period_value: String,
    pub value_num: Option<f64>,
    pub value_status: String,
    pub is_stale: bool,
    pub method_version: i64,
    pub computed_at: String,
}

fn err(e: rusqlite::Error) -> String {
    format!("Kesalahan database: {e}")
}

fn err_kode(e: rusqlite::Error) -> String {
    if e.to_string().contains("UNIQUE") {
        "Kode indikator sudah dipakai.".to_string()
    } else {
        err(e)
    }
}

fn kumpulkan(e: &Expr, vars: &mut HashSet<String>, fns: &mut HashSet<String>) {
    match e {
        Expr::Num(_) => {}
        Expr::Var(n) => {
            vars.insert(n.clone());
        }
        Expr::BinOp { lhs, rhs, .. } => {
            kumpulkan(lhs, vars, fns);
            kumpulkan(rhs, vars, fns);
        }
        Expr::Call { name, args } => {
            fns.insert(name.clone());
            for a in args {
                kumpulkan(a, vars, fns);
            }
        }
    }
}

fn validasi(conn: &Connection, input: &IndicatorInput) -> Result<(), String> {
    if input.code.trim().is_empty() {
        return Err("Kode indikator wajib diisi.".into());
    }
    if input.name.trim().is_empty() {
        return Err("Nama indikator wajib diisi.".into());
    }
    let rule = input.aggregation_rule.as_deref().unwrap_or("sum");
    if rule != "sum" {
        return Err(format!(
            "Aturan agregasi '{rule}' belum didukung. Saat ini hanya: sum."
        ));
    }
    let expr = expression::parse(&input.formula)
        .map_err(|e| format!("Formula tidak valid: {e}"))?;

    let mut aliases: HashSet<String> = HashSet::new();
    for v in &input.variables {
        if !aliases.insert(v.alias.clone()) {
            return Err(format!("Alias '{}' dipakai lebih dari sekali.", v.alias));
        }
        let ada: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM variable WHERE id = ?1",
                params![v.variable_id],
                |r| r.get(0),
            )
            .map_err(err)?;
        if ada == 0 {
            return Err(format!(
                "Variabel dengan ID {} tidak ditemukan.",
                v.variable_id
            ));
        }
    }

    let mut vars = HashSet::new();
    let mut fns = HashSet::new();
    kumpulkan(&expr, &mut vars, &mut fns);
    for f in &fns {
        if !FUNGSI_DIKENAL.contains(&f.as_str()) {
            return Err(format!(
                "Fungsi tidak dikenal: '{f}'. Yang tersedia: sum, avg, count, ratio, if."
            ));
        }
    }
    for n in &vars {
        if !aliases.contains(n) {
            return Err(format!(
                "Formula memakai '{n}' yang belum dipetakan ke variabel."
            ));
        }
    }
    let ctx: Context = aliases.iter().map(|a| (a.clone(), 1.0)).collect();
    expression::eval(&expr, &ctx).map_err(|e| format!("Formula tidak valid: {e}"))?;
    Ok(())
}

fn simpan_variabel(
    conn: &Connection,
    id: i64,
    vs: &[IndicatorVariabel],
) -> Result<(), String> {
    for v in vs {
        conn.execute(
            "INSERT INTO indicator_variable (indicator_id, variable_id, alias)
             VALUES (?1, ?2, ?3)",
            params![id, v.variable_id, v.alias],
        )
        .map_err(err)?;
    }
    Ok(())
}

pub fn buat(conn: &Connection, input: &IndicatorInput) -> Result<i64, String> {
    validasi(conn, input)?;
    let tx = conn.unchecked_transaction().map_err(err)?;
    tx.execute(
        "INSERT INTO indicator (code, name, definition, interpretation, unit, formula,
             aggregation_rule, method_version, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1,
             strftime('%Y-%m-%dT%H:%M:%fZ','now'),
             strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
        params![
            input.code.trim(),
            input.name.trim(),
            input.definition,
            input.interpretation,
            input.unit,
            input.formula.trim(),
            input.aggregation_rule.as_deref().unwrap_or("sum"),
        ],
    )
    .map_err(err_kode)?;
    let id = tx.last_insert_rowid();
    simpan_variabel(&tx, id, &input.variables)?;
    tx.commit().map_err(err)?;
    Ok(id)
}

fn kunci_variabel(vs: &[IndicatorVariabel]) -> Vec<(String, i64)> {
    let mut k: Vec<(String, i64)> = vs
        .iter()
        .map(|v| (v.alias.clone(), v.variable_id))
        .collect();
    k.sort();
    k
}

pub fn ubah(conn: &Connection, id: i64, input: &IndicatorInput) -> Result<(), String> {
    validasi(conn, input)?;
    let lama = ambil(conn, id)?;
    let aturan = input
        .aggregation_rule
        .clone()
        .unwrap_or_else(|| "sum".to_string());
    let metode_berubah = lama.formula != input.formula.trim()
        || lama.aggregation_rule != aturan
        || kunci_variabel(&lama.variables) != kunci_variabel(&input.variables);
    let tx = conn.unchecked_transaction().map_err(err)?;
    tx.execute(
        "UPDATE indicator
         SET code = ?1, name = ?2, definition = ?3, interpretation = ?4,
             unit = ?5, formula = ?6, aggregation_rule = ?7,
             method_version = method_version + ?8,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id = ?9",
        params![
            input.code.trim(),
            input.name.trim(),
            input.definition,
            input.interpretation,
            input.unit,
            input.formula.trim(),
            aturan,
            if metode_berubah { 1i64 } else { 0i64 },
            id,
        ],
    )
    .map_err(err_kode)?;
    if metode_berubah {
        tx.execute(
            "DELETE FROM indicator_variable WHERE indicator_id = ?1",
            params![id],
        )
        .map_err(err)?;
        simpan_variabel(&tx, id, &input.variables)?;
        tx.execute(
            "UPDATE indicator_result SET is_stale = 1 WHERE indicator_id = ?1",
            params![id],
        )
        .map_err(err)?;
    }
    tx.commit().map_err(err)?;
    Ok(())
}

pub fn arsipkan(conn: &Connection, id: i64) -> Result<(), String> {
    let n = conn
        .execute(
            "UPDATE indicator
             SET archived_at = strftime('%Y-%m-%dT%H:%M:%fZ','now'),
                 updated_at  = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?1 AND archived_at IS NULL",
            params![id],
        )
        .map_err(err)?;
    if n == 0 {
        return Err("Indikator tidak ditemukan atau sudah diarsipkan.".into());
    }
    Ok(())
}

pub fn ambil(conn: &Connection, id: i64) -> Result<Indicator, String> {
    let mut ind = conn
        .query_row(
            "SELECT id, code, name, definition, interpretation, unit, formula,
                    aggregation_rule, method_version, archived_at, created_at, updated_at
             FROM indicator WHERE id = ?1",
            params![id],
            |r| {
                Ok(Indicator {
                    id: r.get(0)?,
                    code: r.get(1)?,
                    name: r.get(2)?,
                    definition: r.get(3)?,
                    interpretation: r.get(4)?,
                    unit: r.get(5)?,
                    formula: r.get(6)?,
                    aggregation_rule: r.get(7)?,
                    method_version: r.get(8)?,
                    archived_at: r.get(9)?,
                    created_at: r.get(10)?,
                    updated_at: r.get(11)?,
                    variables: vec![],
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                format!("Indikator dengan ID {id} tidak ditemukan.")
            }
            other => err(other),
        })?;
    let mut st = conn
        .prepare(
            "SELECT variable_id, alias FROM indicator_variable
             WHERE indicator_id = ?1 ORDER BY alias",
        )
        .map_err(err)?;
    let rows = st
        .query_map(params![id], |r| {
            Ok(IndicatorVariabel {
                variable_id: r.get(0)?,
                alias: r.get(1)?,
            })
        })
        .map_err(err)?;
    for row in rows {
        ind.variables.push(row.map_err(err)?);
    }
    Ok(ind)
}

pub fn daftar(conn: &Connection) -> Result<Vec<Indicator>, String> {
    let mut st = conn
        .prepare(
            "SELECT id FROM indicator WHERE archived_at IS NULL ORDER BY code",
        )
        .map_err(err)?;
    let ids: Vec<i64> = st
        .query_map([], |r| r.get(0))
        .map_err(err)?
        .collect::<Result<Vec<i64>, _>>()
        .map_err(err)?;
    ids.into_iter().map(|id| ambil(conn, id)).collect()
}

fn hitung_wilayah(
    conn: &Connection,
    ind: &Indicator,
    region: i64,
    period: &str,
) -> Result<(Option<f64>, &'static str), String> {
    let mut ctx = Context::new();
    for v in &ind.variables {
        let total: Option<f64> = conn
            .query_row(
                "WITH RECURSIVE sub(id) AS (
                     SELECT ?1
                     UNION ALL
                     SELECT u.id FROM ref_unit u JOIN sub s ON u.parent_id = s.id
                 )
                 SELECT SUM(value_num) FROM observation
                 WHERE variable_id = ?2
                   AND period_value = ?3
                   AND dimension_value_id = 0
                   AND value_status = 'terisi'
                   AND value_num IS NOT NULL
                   AND unit_id IN (SELECT id FROM sub)",
                params![region, v.variable_id, period],
                |r| r.get(0),
            )
            .map_err(err)?;
        match total {
            Some(t) => {
                ctx.insert(v.alias.clone(), t);
            }
            None => return Ok((None, "kosong")),
        }
    }
    match expression::evaluate(&ind.formula, &ctx) {
        Ok(Some(v)) if v.is_finite() => Ok((Some(v), "terisi")),
        Ok(_) => Ok((None, "tidak_berlaku")),
        Err(e) => Err(format!("Gagal menghitung {}: {e}", ind.code)),
    }
}

pub fn hitung(
    conn: &Connection,
    indicator_id: i64,
    period: &str,
) -> Result<usize, String> {
    let ind = ambil(conn, indicator_id)?;
    if ind.archived_at.is_some() {
        return Err("Indikator sudah diarsipkan.".into());
    }
    let mut st = conn
        .prepare("SELECT id FROM ref_unit ORDER BY id")
        .map_err(err)?;
    let regions: Vec<i64> = st
        .query_map([], |r| r.get(0))
        .map_err(err)?
        .collect::<Result<Vec<i64>, _>>()
        .map_err(err)?;
    let tx = conn.unchecked_transaction().map_err(err)?;
    for region in &regions {
        let (nilai, status) = hitung_wilayah(&tx, &ind, *region, period)?;
        tx.execute(
            "INSERT INTO indicator_result
                 (indicator_id, method_version, region_id, period_value,
                  value_num, value_status, is_stale, computed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0,
                 strftime('%Y-%m-%dT%H:%M:%fZ','now'))
             ON CONFLICT(indicator_id, region_id, period_value) DO UPDATE SET
                 method_version = excluded.method_version,
                 value_num      = excluded.value_num,
                 value_status   = excluded.value_status,
                 is_stale       = 0,
                 computed_at    = excluded.computed_at",
            params![
                indicator_id,
                ind.method_version,
                region,
                period,
                nilai,
                status,
            ],
        )
        .map_err(err)?;
    }
    tx.commit().map_err(err)?;
    Ok(regions.len())
}

pub fn hasil(
    conn: &Connection,
    indicator_id: i64,
    period: &str,
) -> Result<Vec<HasilIndikator>, String> {
    let mut st = conn
        .prepare(
            "SELECT r.indicator_id, r.region_id, u.name, u.level,
                    r.period_value, r.value_num, r.value_status,
                    r.is_stale, r.method_version, r.computed_at
             FROM indicator_result r
             JOIN ref_unit u ON u.id = r.region_id
             WHERE r.indicator_id = ?1 AND r.period_value = ?2
             ORDER BY
                 CASE u.level
                     WHEN 'provinsi'   THEN 0
                     WHEN 'kabupaten'  THEN 1
                     WHEN 'kecamatan'  THEN 2
                     ELSE 3
                 END, u.name",
        )
        .map_err(err)?;
    let rows = st
        .query_map(params![indicator_id, period], |r| {
            Ok(HasilIndikator {
                indicator_id: r.get(0)?,
                region_id: r.get(1)?,
                region_name: r.get(2)?,
                region_level: r.get(3)?,
                period_value: r.get(4)?,
                value_num: r.get(5)?,
                value_status: r.get(6)?,
                is_stale: r.get::<_, i64>(7)? != 0,
                method_version: r.get(8)?,
                computed_at: r.get(9)?,
            })
        })
        .map_err(err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(err)
}

pub fn jumlah_usang(conn: &Connection) -> Result<i64, String> {
    conn.query_row(
        "SELECT COUNT(*) FROM indicator_result WHERE is_stale = 1",
        [],
        |r| r.get(0),
    )
    .map_err(err)
}

pub fn hitung_ulang_usang(conn: &Connection) -> Result<usize, String> {
    let mut st = conn
        .prepare(
            "SELECT DISTINCT r.indicator_id, r.period_value
             FROM indicator_result r
             JOIN indicator i ON i.id = r.indicator_id
             WHERE r.is_stale = 1 AND i.archived_at IS NULL",
        )
        .map_err(err)?;
    let pasangan: Vec<(i64, String)> = st
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err)?;
    for (id, periode) in &pasangan {
        hitung(conn, *id, periode)?;
    }
    Ok(pasangan.len())
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
            include_str!("../../migrations/0004_indicator.sql")
        ).unwrap();
        conn.execute_batch(
            "INSERT INTO dataset (id, name, period_type) VALUES (1, 'Data PKK', 'tahunan');
             INSERT INTO ref_unit (id, name, level, parent_id) VALUES
                 (1, 'Karangasem', 'kabupaten', NULL),
                 (2, 'Rendang',    'kecamatan', 1),
                 (3, 'Desa A',     'desa',      2),
                 (4, 'Desa B',     'desa',      2);
             INSERT INTO variable (id, dataset_id, name, code, data_type) VALUES
                 (1, 1, 'Jumlah kelompok', 'jumlah_kelompok', 'integer'),
                 (2, 1, 'Kelompok binaan', 'jumlah_binaan',   'integer'),
                 (3, 1, 'Jumlah kader',   'jumlah_kader',    'integer');"
        ).unwrap();
        conn
    }

    fn obs(conn: &Connection, var: i64, unit: i64, period: &str, v: f64) {
        conn.execute(
            "INSERT INTO observation
                 (dataset_id, variable_id, unit_id, period_value, value_num)
             VALUES (1, ?1, ?2, ?3, ?4)",
            params![var, unit, period, v],
        )
        .unwrap();
    }

    fn isi_penuh(conn: &Connection) {
        obs(conn, 1, 3, "2026", 10.0);
        obs(conn, 1, 4, "2026", 20.0);
        obs(conn, 2, 3, "2026",  5.0);
        obs(conn, 2, 4, "2026", 15.0);
    }

    fn pkk() -> IndicatorInput {
        IndicatorInput {
            code: "pkk_binaan_persen".into(),
            name: "Persentase kelompok PKK binaan".into(),
            definition: None,
            interpretation: None,
            unit: Some("persen".into()),
            formula: "ratio(binaan, kelompok) * 100".into(),
            aggregation_rule: None,
            variables: vec![
                IndicatorVariabel {
                    variable_id: 2,
                    alias: "binaan".into(),
                },
                IndicatorVariabel {
                    variable_id: 1,
                    alias: "kelompok".into(),
                },
            ],
        }
    }

    fn nilai(
        conn: &Connection,
        ind: i64,
        wilayah: &str,
    ) -> (Option<f64>, String) {
        let h = hasil(conn, ind, "2026").unwrap();
        let r = h.iter().find(|x| x.region_name == wilayah).unwrap();
        (r.value_num, r.value_status.clone())
    }

    #[test]
    fn buat_dan_validasi() {
        let conn = db();
        let id = buat(&conn, &pkk()).unwrap();
        assert_eq!(ambil(&conn, id).unwrap().variables.len(), 2);
        assert!(buat(&conn, &pkk()).unwrap_err().contains("sudah dipakai"));

        let mut a = pkk();
        a.code = "x1".into();
        a.formula = "foo(binaan)".into();
        assert!(buat(&conn, &a).unwrap_err().contains("Fungsi tidak dikenal"));

        let mut b = pkk();
        b.code = "x2".into();
        b.formula = "binaan / tak_ada".into();
        assert!(buat(&conn, &b).unwrap_err().contains("belum dipetakan"));
    }

    #[test]
    fn hitung_dan_agregasi_naik_tingkat() {
        let conn = db();
        isi_penuh(&conn);
        let id = buat(&conn, &pkk()).unwrap();
        assert_eq!(hitung(&conn, id, "2026").unwrap(), 4);
        assert!((nilai(&conn, id, "Desa A").0.unwrap()    - 50.0).abs() < 1e-9);
        assert!((nilai(&conn, id, "Desa B").0.unwrap()    - 75.0).abs() < 1e-9);
        let kec = 20.0 / 30.0 * 100.0;
        assert!((nilai(&conn, id, "Rendang").0.unwrap()    - kec).abs() < 1e-9);
        assert!((nilai(&conn, id, "Karangasem").0.unwrap() - kec).abs() < 1e-9);
    }

    #[test]
    fn data_hilang_dan_bagi_nol() {
        let conn = db();
        // Desa A punya data, Desa B pembilang nol → tidak_berlaku
        obs(&conn, 1, 3, "2026", 10.0);
        obs(&conn, 2, 3, "2026",  5.0);
        obs(&conn, 1, 4, "2026",  0.0);
        obs(&conn, 2, 4, "2026",  0.0);
        let id = buat(&conn, &pkk()).unwrap();
        hitung(&conn, id, "2026").unwrap();
        assert_eq!(nilai(&conn, id, "Desa B").1, "tidak_berlaku");

        // Desa B tanpa data → kosong
        let conn2 = db();
        obs(&conn2, 1, 3, "2026", 10.0);
        obs(&conn2, 2, 3, "2026",  5.0);
        let id2 = buat(&conn2, &pkk()).unwrap();
        hitung(&conn2, id2, "2026").unwrap();
        assert_eq!(nilai(&conn2, id2, "Desa B").1, "kosong");
    }

    #[test]
    fn hasil_menjadi_usang_dan_dihitung_ulang() {
        let conn = db();
        isi_penuh(&conn);
        let id = buat(&conn, &pkk()).unwrap();
        hitung(&conn, id, "2026").unwrap();
        assert_eq!(jumlah_usang(&conn).unwrap(), 0);

        // variabel kader tidak dipakai indikator → tidak mempengaruhi
        obs(&conn, 3, 3, "2026", 99.0);
        assert_eq!(jumlah_usang(&conn).unwrap(), 0);

        // ubah observasi variabel yang dipakai → usang
        conn.execute(
            "UPDATE observation SET value_num = 8 WHERE variable_id = 2 AND unit_id = 3",
            [],
        )
        .unwrap();
        assert_eq!(jumlah_usang(&conn).unwrap(), 4);
        assert!(hasil(&conn, id, "2026")
            .unwrap()
            .iter()
            .all(|h| h.is_stale));

        assert_eq!(hitung_ulang_usang(&conn).unwrap(), 1);
        assert_eq!(jumlah_usang(&conn).unwrap(), 0);
        assert!((nilai(&conn, id, "Desa A").0.unwrap() - 80.0).abs() < 1e-9);
    }

    #[test]
    fn ubah_formula_menaikkan_versi_dan_menandai_usang() {
        let conn = db();
        isi_penuh(&conn);
        let id = buat(&conn, &pkk()).unwrap();
        hitung(&conn, id, "2026").unwrap();

        let mut baru = pkk();
        baru.formula = "ratio(binaan, kelompok)".into();
        ubah(&conn, id, &baru).unwrap();
        assert_eq!(ambil(&conn, id).unwrap().method_version, 2);
        assert_eq!(jumlah_usang(&conn).unwrap(), 4);

        // ubah nama saja → versi tidak naik
        let mut nama = baru.clone();
        nama.name = "Rasio binaan".into();
        ubah(&conn, id, &nama).unwrap();
        assert_eq!(ambil(&conn, id).unwrap().method_version, 2);
    }
}