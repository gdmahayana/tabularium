# ADR-0001: Penyimpanan dataset dengan model observasi (format panjang)

- **Status:** Diusulkan (menunggu validasi Spike A)
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
(diisi setelah spike)
