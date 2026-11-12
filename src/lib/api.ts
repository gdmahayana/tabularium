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
