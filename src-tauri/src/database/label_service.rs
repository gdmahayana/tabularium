// SPDX-License-Identifier: AGPL-3.0-only
//! Operasi label pada dataset, variabel, dan indikator.
//! Label tersedia sejak 0001_init.sql; indicator_label ditambahkan di 0004.
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
}

fn err(e: rusqlite::Error) -> String {
    format!("Kesalahan database: {e}")
}

pub fn daftar_semua(conn: &Connection) -> Result<Vec<Label>, String> {
    let mut st = conn
        .prepare("SELECT id, name, description FROM label ORDER BY name")
        .map_err(err)?;
    let rows = st
        .query_map([], |r| {
            Ok(Label {
                id: r.get(0)?,
                name: r.get(1)?,
                description: r.get(2)?,
            })
        })
        .map_err(err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(err)
}

fn id_dari_nama(conn: &Connection, nama: &str) -> Result<i64, String> {
    conn.query_row(
        "SELECT id FROM label WHERE name = ?1",
        params![nama],
        |r| r.get(0),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            format!("Label '{nama}' tidak ditemukan.")
        }
        other => err(other),
    })
}

// ── Dataset ──────────────────────────────────────────────────────────────────

pub fn beri_label_dataset(
    conn: &Connection,
    dataset_id: i64,
    label_name: &str,
) -> Result<(), String> {
    let label_id = id_dari_nama(conn, label_name)?;
    conn.execute(
        "INSERT OR IGNORE INTO dataset_label (dataset_id, label_id)
         VALUES (?1, ?2)",
        params![dataset_id, label_id],
    )
    .map_err(err)?;
    Ok(())
}

pub fn cabut_label_dataset(
    conn: &Connection,
    dataset_id: i64,
    label_name: &str,
) -> Result<(), String> {
    let label_id = id_dari_nama(conn, label_name)?;
    conn.execute(
        "DELETE FROM dataset_label WHERE dataset_id = ?1 AND label_id = ?2",
        params![dataset_id, label_id],
    )
    .map_err(err)?;
    Ok(())
}

pub fn label_dataset(conn: &Connection, dataset_id: i64) -> Result<Vec<Label>, String> {
    let mut st = conn
        .prepare(
            "SELECT l.id, l.name, l.description
             FROM label l JOIN dataset_label dl ON dl.label_id = l.id
             WHERE dl.dataset_id = ?1 ORDER BY l.name",
        )
        .map_err(err)?;
    let rows = st
        .query_map(params![dataset_id], |r| {
            Ok(Label {
                id: r.get(0)?,
                name: r.get(1)?,
                description: r.get(2)?,
            })
        })
        .map_err(err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(err)
}

// ── Variabel ─────────────────────────────────────────────────────────────────

pub fn beri_label_variabel(
    conn: &Connection,
    variable_id: i64,
    label_name: &str,
) -> Result<(), String> {
    let label_id = id_dari_nama(conn, label_name)?;
    conn.execute(
        "INSERT OR IGNORE INTO variable_label (variable_id, label_id)
         VALUES (?1, ?2)",
        params![variable_id, label_id],
    )
    .map_err(err)?;
    Ok(())
}

pub fn cabut_label_variabel(
    conn: &Connection,
    variable_id: i64,
    label_name: &str,
) -> Result<(), String> {
    let label_id = id_dari_nama(conn, label_name)?;
    conn.execute(
        "DELETE FROM variable_label WHERE variable_id = ?1 AND label_id = ?2",
        params![variable_id, label_id],
    )
    .map_err(err)?;
    Ok(())
}

pub fn label_variabel(conn: &Connection, variable_id: i64) -> Result<Vec<Label>, String> {
    let mut st = conn
        .prepare(
            "SELECT l.id, l.name, l.description
             FROM label l JOIN variable_label vl ON vl.label_id = l.id
             WHERE vl.variable_id = ?1 ORDER BY l.name",
        )
        .map_err(err)?;
    let rows = st
        .query_map(params![variable_id], |r| {
            Ok(Label {
                id: r.get(0)?,
                name: r.get(1)?,
                description: r.get(2)?,
            })
        })
        .map_err(err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(err)
}

// ── Indikator ─────────────────────────────────────────────────────────────────

pub fn beri_label_indikator(
    conn: &Connection,
    indicator_id: i64,
    label_name: &str,
) -> Result<(), String> {
    let label_id = id_dari_nama(conn, label_name)?;
    conn.execute(
        "INSERT OR IGNORE INTO indicator_label (indicator_id, label_id)
         VALUES (?1, ?2)",
        params![indicator_id, label_id],
    )
    .map_err(err)?;
    Ok(())
}

pub fn cabut_label_indikator(
    conn: &Connection,
    indicator_id: i64,
    label_name: &str,
) -> Result<(), String> {
    let label_id = id_dari_nama(conn, label_name)?;
    conn.execute(
        "DELETE FROM indicator_label WHERE indicator_id = ?1 AND label_id = ?2",
        params![indicator_id, label_id],
    )
    .map_err(err)?;
    Ok(())
}

pub fn label_indikator(conn: &Connection, indicator_id: i64) -> Result<Vec<Label>, String> {
    let mut st = conn
        .prepare(
            "SELECT l.id, l.name, l.description
             FROM label l JOIN indicator_label il ON il.label_id = l.id
             WHERE il.indicator_id = ?1 ORDER BY l.name",
        )
        .map_err(err)?;
    let rows = st
        .query_map(params![indicator_id], |r| {
            Ok(Label {
                id: r.get(0)?,
                name: r.get(1)?,
                description: r.get(2)?,
            })
        })
        .map_err(err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(err)
}

// ── Query: semua entitas berlabel Statistik Sektoral ─────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitasSektoral {
    pub dataset_ids: Vec<i64>,
    pub variable_ids: Vec<i64>,
    pub indicator_ids: Vec<i64>,
}

pub fn entitas_sektoral(conn: &Connection) -> Result<EntitasSektoral, String> {
    let label_id: i64 = id_dari_nama(conn, "Statistik Sektoral")?;

    let mut st = conn
        .prepare("SELECT dataset_id FROM dataset_label WHERE label_id = ?1")
        .map_err(err)?;
    let dataset_ids = st
        .query_map(params![label_id], |r| r.get(0))
        .map_err(err)?
        .collect::<Result<Vec<i64>, _>>()
        .map_err(err)?;

    let mut st = conn
        .prepare("SELECT variable_id FROM variable_label WHERE label_id = ?1")
        .map_err(err)?;
    let variable_ids = st
        .query_map(params![label_id], |r| r.get(0))
        .map_err(err)?
        .collect::<Result<Vec<i64>, _>>()
        .map_err(err)?;

    let mut st = conn
        .prepare("SELECT indicator_id FROM indicator_label WHERE label_id = ?1")
        .map_err(err)?;
    let indicator_ids = st
        .query_map(params![label_id], |r| r.get(0))
        .map_err(err)?
        .collect::<Result<Vec<i64>, _>>()
        .map_err(err)?;

    Ok(EntitasSektoral {
        dataset_ids,
        variable_ids,
        indicator_ids,
    })
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
            "INSERT INTO dataset (id, name, period_type)
             VALUES (1, 'Data PKK', 'tahunan');
             INSERT INTO variable (id, dataset_id, name, code, data_type)
             VALUES (1, 1, 'Jumlah kelompok', 'jumlah_kelompok', 'integer');
             INSERT INTO indicator (id, code, name, formula, created_at, updated_at)
             VALUES (1, 'pkk_persen', 'Persen PKK', 'ratio(a,b)',
                 strftime('%Y-%m-%dT%H:%M:%fZ','now'),
                 strftime('%Y-%m-%dT%H:%M:%fZ','now'));"
        ).unwrap();
        conn
    }

    #[test]
    fn label_dataset_variabel_indikator() {
        let conn = db();

        beri_label_dataset(&conn, 1, "Statistik Sektoral").unwrap();
        beri_label_variabel(&conn, 1, "Statistik Sektoral").unwrap();
        beri_label_indikator(&conn, 1, "Statistik Sektoral").unwrap();

        assert_eq!(label_dataset(&conn, 1).unwrap().len(), 1);
        assert_eq!(label_variabel(&conn, 1).unwrap().len(), 1);
        assert_eq!(label_indikator(&conn, 1).unwrap().len(), 1);

        // idempoten: beri dua kali tidak duplikat
        beri_label_dataset(&conn, 1, "Statistik Sektoral").unwrap();
        assert_eq!(label_dataset(&conn, 1).unwrap().len(), 1);

        // cabut
        cabut_label_dataset(&conn, 1, "Statistik Sektoral").unwrap();
        assert_eq!(label_dataset(&conn, 1).unwrap().len(), 0);
    }

    #[test]
    fn label_tidak_ada_ditolak() {
        let conn = db();
        assert!(beri_label_dataset(&conn, 1, "LabelFiktif")
            .unwrap_err()
            .contains("tidak ditemukan"));
    }

    #[test]
    fn entitas_sektoral_query() {
        let conn = db();
        beri_label_dataset(&conn, 1, "Statistik Sektoral").unwrap();
        beri_label_indikator(&conn, 1, "Statistik Sektoral").unwrap();

        let es = entitas_sektoral(&conn).unwrap();
        assert_eq!(es.dataset_ids, vec![1]);
        assert_eq!(es.variable_ids, Vec::<i64>::new());
        assert_eq!(es.indicator_ids, vec![1]);
    }
}