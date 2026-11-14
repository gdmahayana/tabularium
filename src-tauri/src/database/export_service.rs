// SPDX-License-Identifier: AGPL-3.0-only
//! Ekspor Lembar Input SDI ke CSV dan JSON.
//! XLSX ditangani sidecar Python (Tahap lanjut).
use serde_json::json;
use crate::database::sdi_service::SelLembar;

/// Ekspor lembar ke CSV. Kolom: bidang, sub_bidang, kategori, judul_kolom,
/// satuan_unit, nilai, value_status, sudah_diinput, perlu_diperbarui.
pub fn ke_csv(lembar: &[SelLembar]) -> String {
    let mut out = String::new();
    out.push_str(
        "bidang,sub_bidang,kategori,judul_kolom,satuan_unit,\
         nilai,value_status,sudah_diinput,perlu_diperbarui\n",
    );
    for s in lembar {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            csv_esc(&s.bidang),
            csv_esc(&s.sub_bidang),
            csv_esc(&s.kategori),
            csv_esc(&s.judul_kolom),
            csv_esc(&s.satuan_unit),
            s.nilai_tampil.as_deref().unwrap_or(""),
            s.value_status,
            if s.sudah_diinput { "1" } else { "0" },
            if s.perlu_diperbarui { "1" } else { "0" },
        ));
    }
    out
}

fn csv_esc(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Ekspor lembar ke JSON array.
pub fn ke_json(lembar: &[SelLembar]) -> serde_json::Value {
    let rows: Vec<serde_json::Value> = lembar
        .iter()
        .map(|s| {
            json!({
                "bidang": s.bidang,
                "sub_bidang": s.sub_bidang,
                "kategori": s.kategori,
                "judul_kolom": s.judul_kolom,
                "satuan_unit": s.satuan_unit,
                "nilai": s.nilai,
                "nilai_tampil": s.nilai_tampil,
                "value_status": s.value_status,
                "is_stale": s.is_stale,
                "sudah_diinput": s.sudah_diinput,
                "perlu_diperbarui": s.perlu_diperbarui,
                "input_at": s.input_at,
                "catatan": s.catatan,
            })
        })
        .collect();
    json!({ "lembar": rows, "total": rows.len() })
}

/// Teks salin satu sel — nilai dan satuan, siap tempel ke form web.
pub fn salin_sel(sel: &SelLembar) -> String {
    match &sel.nilai_tampil {
        Some(v) => format!("{} {}", v, sel.satuan_unit).trim().to_string(),
        None => String::new(),
    }
}

/// Teks salin satu baris tabel (tab-separated) untuk tempel ke spreadsheet.
pub fn salin_baris(sel: &SelLembar) -> String {
    format!(
        "{}\t{}\t{}\t{}\t{}",
        sel.judul_kolom,
        sel.nilai_tampil.as_deref().unwrap_or(""),
        sel.satuan_unit,
        sel.value_status,
        if sel.sudah_diinput { "✓" } else { "" },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::sdi_service::SelLembar;

    fn sel(judul: &str, nilai: Option<f64>, tampil: Option<&str>, status: &str) -> SelLembar {
        SelLembar {
            profile_id: "test".into(),
            period_value: "2026".into(),
            bidang: "Pemberdayaan".into(),
            sub_bidang: "PKK".into(),
            kategori: "Pembinaan PKK".into(),
            judul_kolom: judul.into(),
            satuan_unit: "Kelompok".into(),
            source_type: "variable".into(),
            source_ref: "jumlah_kelompok".into(),
            aggregation: "sum_all_villages".into(),
            unit_factor: 1.0,
            nilai,
            nilai_tampil: tampil.map(|s| s.into()),
            value_status: status.into(),
            is_stale: false,
            sudah_diinput: false,
            perlu_diperbarui: false,
            input_at: None,
            catatan: None,
        }
    }

    #[test]
    fn csv_header_dan_baris() {
        let lembar = vec![
            sel("Jumlah Kelompok PKK", Some(30.0), Some("30"), "terisi"),
            sel("Nilai, dengan koma", None, None, "kosong"),
        ];
        let csv = ke_csv(&lembar);
        let baris: Vec<&str> = csv.lines().collect();
        assert_eq!(baris.len(), 3);
        assert!(baris[0].starts_with("bidang,"));
        assert!(baris[1].contains("30"));
        // kolom dengan koma harus di-escape dengan tanda kutip
        assert!(baris[2].contains("\"Nilai, dengan koma\""));
    }

    #[test]
    fn json_struktur() {
        let lembar = vec![sel("Jumlah Kelompok PKK", Some(30.0), Some("30"), "terisi")];
        let j = ke_json(&lembar);
        assert_eq!(j["total"], 1);
        assert_eq!(j["lembar"][0]["judul_kolom"], "Jumlah Kelompok PKK");
        assert_eq!(j["lembar"][0]["nilai"], 30.0);
    }

    #[test]
    fn salin_sel_dan_baris() {
        let s = sel("Jumlah Kelompok PKK", Some(30.0), Some("30"), "terisi");
        assert_eq!(salin_sel(&s), "30 Kelompok");
        let baris = salin_baris(&s);
        assert!(baris.contains("Jumlah Kelompok PKK"));
        assert!(baris.contains("30"));

        let kosong = sel("Tanpa Data", None, None, "kosong");
        assert_eq!(salin_sel(&kosong), "");
    }

    #[test]
    fn csv_escape_kutip_ganda() {
        let lembar = vec![sel("Kolom \"petik\"", Some(1.0), Some("1"), "terisi")];
        let csv = ke_csv(&lembar);
        assert!(csv.contains("\"Kolom \"\"petik\"\"\""));
    }
}