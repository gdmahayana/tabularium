// SPDX-License-Identifier: AGPL-3.0-only
//! Memuat dan memvalidasi Profil Portal dari file JSON di folder profiles/.
//! Profil tidak disimpan di database — ia dibaca dari disk setiap kali dibutuhkan
//! sehingga dapat diedit tanpa migrasi.
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub ref_code: Option<String>,
    #[serde(rename = "ref")]
    pub ref_field: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalColumn {
    pub judul_kolom: String,
    pub satuan_unit: String,
    pub source: PortalSource,
    pub aggregation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalEntry {
    pub bidang: String,
    pub sub_bidang: String,
    pub kategori_laporan: String,
    pub columns: Vec<PortalColumn>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalProfile {
    pub profile_id: String,
    pub version: String,
    pub status: Option<String>,
    pub portal: Option<String>,
    pub hierarchy: Vec<String>,
    pub entries: Vec<PortalEntry>,
}

pub fn muat(path: &Path) -> Result<PortalProfile, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("Gagal membaca profil {}: {e}", path.display()))?;
    let profile: PortalProfile = serde_json::from_str(&raw)
        .map_err(|e| format!("Profil JSON tidak valid {}: {e}", path.display()))?;
    validasi(&profile)?;
    Ok(profile)
}

fn validasi(p: &PortalProfile) -> Result<(), String> {
    if p.profile_id.trim().is_empty() {
        return Err("profile_id wajib diisi.".into());
    }
    for (i, e) in p.entries.iter().enumerate() {
        if e.bidang.trim().is_empty() {
            return Err(format!("entries[{i}]: bidang wajib diisi."));
        }
        if e.columns.is_empty() {
            return Err(format!(
                "entries[{i}] '{}': columns tidak boleh kosong.",
                e.kategori_laporan
            ));
        }
        for (j, c) in e.columns.iter().enumerate() {
            if c.judul_kolom.trim().is_empty() {
                return Err(format!(
                    "entries[{i}].columns[{j}]: judul_kolom wajib diisi."
                ));
            }
            let src_type = c.source.ref_field.as_deref()
                .or(c.source.ref_code.as_deref())
                .unwrap_or("");
            if src_type.trim().is_empty() {
                return Err(format!(
                    "entries[{i}].columns[{j}] '{}': source.ref wajib diisi.",
                    c.judul_kolom
                ));
            }
            if !["variable", "indicator"].contains(&c.source.source_type.as_str()) {
                return Err(format!(
                    "entries[{i}].columns[{j}]: source.type harus 'variable' atau 'indicator'."
                ));
            }
        }
    }
    Ok(())
}

/// Kembalikan daftar kolom datar dari seluruh entries profil.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KolomDatar {
    pub bidang: String,
    pub sub_bidang: String,
    pub kategori_laporan: String,
    pub judul_kolom: String,
    pub satuan_unit: String,
    pub source_type: String,
    pub source_ref: String,
    pub aggregation: String,
}

pub fn kolom_datar(profile: &PortalProfile) -> Vec<KolomDatar> {
    let mut hasil = vec![];
    for e in &profile.entries {
        for c in &e.columns {
            let source_ref = c.source.ref_field.clone()
                .or_else(|| c.source.ref_code.clone())
                .unwrap_or_default();
            hasil.push(KolomDatar {
                bidang: e.bidang.clone(),
                sub_bidang: e.sub_bidang.clone(),
                kategori_laporan: e.kategori_laporan.clone(),
                judul_kolom: c.judul_kolom.clone(),
                satuan_unit: c.satuan_unit.clone(),
                source_type: c.source.source_type.clone(),
                source_ref,
                aggregation: c.aggregation.clone(),
            });
        }
    }
    hasil
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn profil_valid() -> &'static str {
        r#"{
            "profile_id": "test-profil",
            "version": "0.1.0",
            "hierarchy": ["bidang","sub_bidang","kategori_laporan","judul_kolom"],
            "entries": [{
                "bidang": "Bidang A",
                "sub_bidang": "Sub A",
                "kategori_laporan": "Laporan PKK",
                "columns": [
                    {
                        "judul_kolom": "Jumlah Kelompok",
                        "satuan_unit": "Kelompok",
                        "source": {"type": "variable", "ref": "jumlah_kelompok"},
                        "aggregation": "sum_all_villages"
                    },
                    {
                        "judul_kolom": "Persen Binaan",
                        "satuan_unit": "Persen",
                        "source": {"type": "indicator", "ref": "pkk_binaan_persen"},
                        "aggregation": "ratio_of_sums"
                    }
                ]
            }]
        }"#
    }

    fn tulis_tmp(isi: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(isi.as_bytes()).unwrap();
        f
    }

    #[test]
    fn muat_profil_valid() {
        let f = tulis_tmp(profil_valid());
        let p = muat(f.path()).unwrap();
        assert_eq!(p.profile_id, "test-profil");
        assert_eq!(p.entries.len(), 1);
        assert_eq!(p.entries[0].columns.len(), 2);
    }

    #[test]
    fn kolom_datar_menghasilkan_semua_kolom() {
        let f = tulis_tmp(profil_valid());
        let p = muat(f.path()).unwrap();
        let kd = kolom_datar(&p);
        assert_eq!(kd.len(), 2);
        assert_eq!(kd[0].source_type, "variable");
        assert_eq!(kd[1].source_ref, "pkk_binaan_persen");
    }

    #[test]
    fn profil_tanpa_profile_id_ditolak() {
        let json = r#"{"profile_id":"","version":"0.1","hierarchy":[],"entries":[]}"#;
        let f = tulis_tmp(json);
        assert!(muat(f.path()).unwrap_err().contains("profile_id"));
    }

    #[test]
    fn profil_source_ref_kosong_ditolak() {
        let json = r#"{
            "profile_id":"x","version":"0.1","hierarchy":[],
            "entries":[{"bidang":"B","sub_bidang":"S","kategori_laporan":"K",
                "columns":[{"judul_kolom":"J","satuan_unit":"u",
                    "source":{"type":"variable","ref":""},"aggregation":"sum_all_villages"}]}]}"#;
        let f = tulis_tmp(json);
        assert!(muat(f.path()).unwrap_err().contains("source.ref"));
    }
}