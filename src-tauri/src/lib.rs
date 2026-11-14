// SPDX-License-Identifier: AGPL-3.0-only
pub mod commands;
pub mod database;
pub mod expression;
pub mod sidecar;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::cmd_buat_dataset,
            commands::cmd_ambil_dataset,
            commands::cmd_daftar_dataset,
            commands::cmd_ubah_dataset,
            commands::cmd_hapus_dataset,
            commands::cmd_buat_variable,
            commands::cmd_daftar_variable,
            commands::cmd_ubah_variable,
            commands::cmd_arsipkan_variable,
            commands::cmd_atur_urutan_variable,
            commands::cmd_daftar_unit,
            commands::cmd_daftar_anak_unit,
            commands::cmd_impor_wilayah_csv,
            commands::cmd_audit_log,
            commands::cmd_daftar_label,
            commands::cmd_tambah_label_dataset,
            commands::cmd_hapus_label_dataset,
            commands::cmd_label_dataset,
            commands::cmd_buat_backup,
            commands::cmd_daftar_backup,
            commands::cmd_restore_backup,
            commands::cmd_inspeksi_file,
            commands::cmd_baca_tabel,
            commands::cmd_buat_sesi_impor,
            commands::cmd_simpan_staging,
            commands::cmd_simpan_unit_match,
            commands::cmd_komit_impor,
            commands::cmd_undo_batch,
            commands::cmd_validasi_impor,
            commands::cmd_simpan_observasi,
            commands::cmd_ubah_observasi,
            commands::cmd_buat_indicator,
            commands::cmd_ambil_indicator,
            commands::cmd_daftar_indicator,
            commands::cmd_ubah_indicator,
            commands::cmd_arsipkan_indicator,
            commands::cmd_hitung_indicator,
            commands::cmd_hasil_indicator,
            commands::cmd_jumlah_usang,
            commands::cmd_hitung_ulang_usang,
            commands::cmd_daftar_semua_label,
            commands::cmd_beri_label_variabel,
            commands::cmd_cabut_label_variabel,
            commands::cmd_label_variabel,
            commands::cmd_beri_label_indikator,
            commands::cmd_cabut_label_indikator,
            commands::cmd_label_indikator,
            commands::cmd_entitas_sektoral,
            commands::cmd_buat_mapping,
            commands::cmd_ubah_mapping,
            commands::cmd_hapus_mapping,
            commands::cmd_daftar_mapping,
            commands::cmd_susun_lembar,
            commands::cmd_tandai_sel,
            commands::cmd_batal_tandai_sel,
            commands::cmd_tandai_perlu_diperbarui,
            commands::cmd_ringkasan_lembar,
            commands::cmd_catat_submission,
            commands::cmd_daftar_submission,
            commands::cmd_ekspor_csv,
            commands::cmd_ekspor_json,
        ])
        .setup(|app| {
            let data_dir = app.path().app_data_dir()
                .expect("Tidak dapat menentukan folder data aplikasi.");
            std::fs::create_dir_all(&data_dir)
                .expect("Gagal membuat folder data aplikasi.");
            let db_path     = data_dir.join("tabularium.sqlite");
            let backup_dir  = data_dir.join("backups");
            let db_state = commands::DbState::buka(db_path, backup_dir)
                .expect("Gagal membuka database TABULARIUM.");
            app.manage(db_state);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tabularium");
}
