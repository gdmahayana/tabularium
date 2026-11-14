// SPDX-License-Identifier: AGPL-3.0-only
// Wrapper tipir untuk semua Tauri Commands.
// Seluruh akses ke backend melewati file ini.

import { invoke } from '@tauri-apps/api/core';

// ---- Tipe data (cermin dari models.rs) ----

export interface Dataset {
  id: number;
  name: string;
  description: string | null;
  domain: string | null;
  grain_desc: string | null;
  period_type: string;
  owner: string | null;
  source: string | null;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface BuatDatasetInput {
  name: string;
  description?: string;
  domain?: string;
  grain_desc?: string;
  period_type: string;
  owner?: string;
  source?: string;
}

export interface UbahDatasetInput {
  name?: string;
  description?: string;
  domain?: string;
  grain_desc?: string;
  period_type?: string;
  owner?: string;
  source?: string;
  reason?: string;
}

export interface Variable {
  id: number;
  dataset_id: number;
  name: string;
  code: string;
  concept: string | null;
  definition: string | null;
  data_type: string;
  unit: string | null;
  is_required: boolean;
  sort_order: number;
  archived_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface BuatVariableInput {
  dataset_id: number;
  name: string;
  code: string;
  concept?: string;
  definition?: string;
  data_type: string;
  unit?: string;
  is_required?: boolean;
}

export interface EntriAuditLog {
  id: number;
  entity_type: string;
  entity_id: number;
  action: string;
  field: string | null;
  old_value: string | null;
  new_value: string | null;
  reason: string | null;
  changed_by: string;
  changed_at: string;
}

export interface Label {
  id: number;
  name: string;
  description: string | null;
}

export interface RefUnit {
  id: number;
  code: string | null;
  name: string;
  level: string;
  parent_id: number | null;
}


// ---- Tipe impor ----
export interface IsuValidasi {
  row_no: number | null;
  unit_ref: string | null;
  severity: string;
  message: string;
}
export interface HasilValidasi {
  result: string;
  run_id: number;
  issues: IsuValidasi[];
}
export interface HasilKomit {
  batch_id: number;
  inserted: number;
  skipped: number;
}

// ---- Impor ----
// ---- Dataset ----

// ---- Tipe Tahap 3 ----

export interface IndicatorVariabel {
  variable_id: number;
  alias: string;
}

export interface IndicatorInput {
  code: string;
  name: string;
  definition?: string;
  interpretation?: string;
  unit?: string;
  formula: string;
  aggregation_rule?: string;
  variables: IndicatorVariabel[];
}

export interface Indicator {
  id: number;
  code: string;
  name: string;
  definition: string | null;
  interpretation: string | null;
  unit: string | null;
  formula: string;
  aggregation_rule: string;
  method_version: number;
  archived_at: string | null;
  created_at: string;
  updated_at: string;
  variables: IndicatorVariabel[];
}

export interface HasilIndikator {
  indicator_id: number;
  region_id: number;
  region_name: string;
  region_level: string;
  period_value: string;
  value_num: number | null;
  value_status: string;
  is_stale: boolean;
  method_version: number;
  computed_at: string;
}

export interface LabelItem {
  id: number;
  name: string;
  description: string | null;
}

export interface MappingInput {
  profile_id: string;
  bidang: string;
  sub_bidang: string;
  kategori: string;
  judul_kolom: string;
  satuan_unit: string;
  source_type: string;
  source_ref: string;
  aggregation: string;
  unit_factor?: number;
}

export interface Mapping extends MappingInput {
  id: number;
  unit_factor: number;
  created_at: string;
  updated_at: string;
}

export interface TandaiSelInput {
  profile_id: string;
  period_value: string;
  bidang: string;
  sub_bidang: string;
  kategori: string;
  judul_kolom: string;
  nilai_saat_input?: number;
  catatan?: string;
  operator: string;
}

export interface SelLembar {
  profile_id: string;
  period_value: string;
  bidang: string;
  sub_bidang: string;
  kategori: string;
  judul_kolom: string;
  satuan_unit: string;
  source_type: string;
  source_ref: string;
  aggregation: string;
  unit_factor: number;
  nilai: number | null;
  nilai_tampil: string | null;
  value_status: string;
  is_stale: boolean;
  sudah_diinput: boolean;
  perlu_diperbarui: boolean;
  input_at: string | null;
  catatan: string | null;
}

export interface RingkasanLembar {
  total_sel: number;
  sudah_diinput: number;
  perlu_diperbarui: number;
  belum_diinput: number;
  nilai_kosong: number;
}

export interface SubmissionInput {
  profile_id: string;
  period_value: string;
  status: string;
  catatan?: string;
  bukti_ref?: string;
  operator: string;
}

export interface Submission {
  id: number;
  profile_id: string;
  period_value: string;
  status: string;
  catatan: string | null;
  bukti_ref: string | null;
  operator: string;
  created_at: string;
  updated_at: string;
}

export interface EntitasSektoral {
  dataset_ids: number[];
  variable_ids: number[];
  indicator_ids: number[];
}
export const api = {
  dataset: {
    buat:   (input: BuatDatasetInput)              => invoke<Dataset>('cmd_buat_dataset', { input }),
    ambil:  (id: number)                           => invoke<Dataset>('cmd_ambil_dataset', { id }),
    daftar: ()                                     => invoke<Dataset[]>('cmd_daftar_dataset'),
    ubah:   (id: number, input: UbahDatasetInput)  => invoke<Dataset>('cmd_ubah_dataset', { id, input }),
    hapus:  (id: number)                           => invoke<void>('cmd_hapus_dataset', { id }),
  },
  variable: {
    buat:      (input: BuatVariableInput)          => invoke<Variable>('cmd_buat_variable', { input }),
    daftar:    (datasetId: number)                 => invoke<Variable[]>('cmd_daftar_variable', { datasetId }),
    arsipkan:  (id: number)                        => invoke<void>('cmd_arsipkan_variable', { id }),
  },
  audit: {
    log: (entity_type: string, entity_id: number, limit?: number) =>
      invoke<EntriAuditLog[]>('cmd_audit_log', { entityType: entity_type, entityId: entity_id, limit }),
  },
  label: {
    daftar:        ()                              => invoke<Label[]>('cmd_daftar_label'),
    tambahDataset: (dataset_id: number, label_id: number) =>
      invoke<void>('cmd_tambah_label_dataset', { datasetId: dataset_id, labelId: label_id }),
    hapusDataset:  (dataset_id: number, label_id: number) =>
      invoke<void>('cmd_hapus_label_dataset', { datasetId: dataset_id, labelId: label_id }),
    daftarDataset: (datasetId: number)            => invoke<Label[]>('cmd_label_dataset', { datasetId }),
  },
  unit: {
    daftar: (level: string)    => invoke<RefUnit[]>('cmd_daftar_unit', { level }),
    anak:   (parentId: number)  => invoke<RefUnit[]>('cmd_daftar_anak_unit', { parentId }),
  },
  backup: {
    buat:   ()                   => invoke<string>('cmd_buat_backup'),
    daftar: ()                   => invoke<string[]>('cmd_daftar_backup'),
    restore:(backupPath: string)  => invoke<void>('cmd_restore_backup', { backupPath }),
  },

  indicator: {
    buat:          (input: IndicatorInput)           => invoke<number>('cmd_buat_indicator', { input }),
    ambil:         (id: number)                      => invoke<Indicator>('cmd_ambil_indicator', { id }),
    daftar:        ()                                => invoke<Indicator[]>('cmd_daftar_indicator'),
    ubah:          (id: number, input: IndicatorInput) => invoke<void>('cmd_ubah_indicator', { id, input }),
    arsipkan:      (id: number)                      => invoke<void>('cmd_arsipkan_indicator', { id }),
    hitung:        (indicatorId: number, period: string) => invoke<number>('cmd_hitung_indicator', { indicatorId, period }),
    hasil:         (indicatorId: number, period: string) => invoke<HasilIndikator[]>('cmd_hasil_indicator', { indicatorId, period }),
    jumlahUsang:   ()                                => invoke<number>('cmd_jumlah_usang'),
    hitungUlang:   ()                                => invoke<number>('cmd_hitung_ulang_usang'),
  },
  labelItem: {
    semua:             ()                            => invoke<LabelItem[]>('cmd_daftar_semua_label'),
    beriVariabel:      (variableId: number, labelName: string) => invoke<void>('cmd_beri_label_variabel', { variableId, labelName }),
    cabutVariabel:     (variableId: number, labelName: string) => invoke<void>('cmd_cabut_label_variabel', { variableId, labelName }),
    daftarVariabel:    (variableId: number)          => invoke<LabelItem[]>('cmd_label_variabel', { variableId }),
    beriIndikator:     (indicatorId: number, labelName: string) => invoke<void>('cmd_beri_label_indikator', { indicatorId, labelName }),
    cabutIndikator:    (indicatorId: number, labelName: string) => invoke<void>('cmd_cabut_label_indikator', { indicatorId, labelName }),
    daftarIndikator:   (indicatorId: number)         => invoke<LabelItem[]>('cmd_label_indikator', { indicatorId }),
    entitasSektoral:   ()                            => invoke<EntitasSektoral>('cmd_entitas_sektoral'),
  },
  mapping: {
    buat:   (input: MappingInput)                    => invoke<number>('cmd_buat_mapping', { input }),
    ubah:   (id: number, input: MappingInput)        => invoke<void>('cmd_ubah_mapping', { id, input }),
    hapus:  (id: number)                             => invoke<void>('cmd_hapus_mapping', { id }),
    daftar: (profileId: string)                      => invoke<Mapping[]>('cmd_daftar_mapping', { profileId }),
  },
  sdi: {
    susunLembar:       (profileId: string, periodValue: string, regionId: number) =>
      invoke<SelLembar[]>('cmd_susun_lembar', { profileId, periodValue, regionId }),
    tandaiSel:         (input: TandaiSelInput)       => invoke<void>('cmd_tandai_sel', { input }),
    batalTandai:       (profileId: string, periodValue: string, bidang: string, subBidang: string, kategori: string, judulKolom: string) =>
      invoke<void>('cmd_batal_tandai_sel', { profileId, periodValue, bidang, subBidang, kategori, judulKolom }),
    ringkasan:         (profileId: string, periodValue: string, regionId: number) =>
      invoke<RingkasanLembar>('cmd_ringkasan_lembar', { profileId, periodValue, regionId }),
    catatSubmission:   (input: SubmissionInput)      => invoke<number>('cmd_catat_submission', { input }),
    daftarSubmission:  (profileId: string, periodValue: string) =>
      invoke<Submission[]>('cmd_daftar_submission', { profileId, periodValue }),
    eksporCsv:         (profileId: string, periodValue: string, regionId: number) =>
      invoke<string>('cmd_ekspor_csv', { profileId, periodValue, regionId }),
    eksporJson:        (profileId: string, periodValue: string, regionId: number) =>
      invoke<unknown>('cmd_ekspor_json', { profileId, periodValue, regionId }),
  },
  impor: {
    inspeksiFile:   (path: string) =>
      invoke<Record<string, { columns: string[]; row_count: number; preview: Record<string,unknown>[] }>>('cmd_inspeksi_file', { path }),
    bacaTabel:      (path: string, sheet: string, outDir?: string) =>
      invoke<{ out_path: string; columns: string[]; row_count: number }>('cmd_baca_tabel', { path, sheet, outDir }),
    buatSesi:       (datasetId: number, sourceName: string, sourceSheet: string|null, periodValue: string) =>
      invoke<number>('cmd_buat_sesi_impor', { datasetId, sourceName, sourceSheet, periodValue }),
    simpanStaging:  (sessionId: number, rows: unknown[]) =>
      invoke<number>('cmd_simpan_staging', { sessionId, rows }),
    simpanUnitMatch:(sessionId: number, matches: unknown[]) =>
      invoke<void>('cmd_simpan_unit_match', { sessionId, matches }),
    validasi:       (sessionId: number, datasetId: number, colVariableMap: [string,number,string][], colUnitWilayah: string, periodValue: string) =>
      invoke<HasilValidasi>('cmd_validasi_impor', { sessionId, datasetId, colVariableMap, colUnitWilayah, periodValue }),
    komit:          (sessionId: number, datasetId: number, periodValue: string, rows: unknown[], colVariableMap: [string,number,number][], colUnitWilayah: string) =>
      invoke<HasilKomit>('cmd_komit_impor', { sessionId, datasetId, periodValue, rows, colVariableMap, colUnitWilayah }),
    undoBatch:      (batchId: number) =>
      invoke<number>('cmd_undo_batch', { batchId }),
  },
};
