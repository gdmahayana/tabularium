# ADR-0001: Penyimpanan dataset dengan model observasi (format panjang)

- **Status:** Diterima (Spike A lulus 2026-11-09)
- **Tanggal:** 2026-11-09

## Konteks

Dataset buatan pengguna harus mendukung audit per sel, identitas variabel yang stabil, banyak periode,
dimensi disagregasi, dan analisis lintas dataset. Skala OPD: ribuan hingga ratusan ribu sel.

## Opsi

- A. Tabel fisik per dataset (DDL dinamis)
- B. Observasi format panjang terpusat (dataset, unit, periode, variabel, dimensi, nilai bertipe)
- C. JSON per baris

## Keputusan

Opsi **B**, dengan view/tabel turunan per dataset untuk analisis cepat. Spike A memvalidasi, bukan membuka ulang;
opsi A/C dipertimbangkan lagi hanya bila ada kegagalan terukur.

## Kriteria lulus Spike A

- Agregasi (jumlah per kecamatan/tahun) atas 100 ribu sel selesai dalam hitungan detik
- Audit per sel berjalan tanpa penalti besar pada impor
- Kueri yang diperlukan Tahap 3 (indikator, lembar input) tidak rumit berlebihan
- Dua dataset berstruktur beda (PKK dan Aset Desa dummy) sama-sama termodelkan tanpa DDL dinamis

## Bukti / hasil uji

Spike A lulus 2026-11-09:

- 312 observasi (PKK 72 + Aset Desa 240) di SQLite 4.0 KB
- Agregasi jumlah per kecamatan per tahun: 0.28 ms
- Anomali (binaan > kelompok) terdeteksi lewat JOIN sederhana
- Sel kosong tercatat dengan null_reason
- Dua dataset berbeda struktur termodelkan tanpa DDL dinamis
- Skrip: tests/spike_a_observation_model.py
