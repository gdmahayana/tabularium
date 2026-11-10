# Data Sintetis

Semua nama dan angka **fiktif**. Dibuat ulang dengan `python3 examples/generate_dummy.py` (seed tetap).

- `wilayah_dummy.csv`: master wilayah hierarkis (kecamatan → desa), 12 desa
- `pkk_dummy_long.csv`: PKK, grain desa × tahun (2024, 2025). Sengaja berisi 1 anomali (K02D03/2025: binaan > kelompok) dan 1 sel kosong (K01D02/2025: kader)
- `aset_desa_dummy.csv`: dataset kedua, grain 1 aset (struktur berbeda dari PKK)
- `../tests/golden-files/pkk_messy_dummy.xlsx`: Excel berantakan untuk Spike C (judul bertingkat, sel digabung, subtotal/total berformula, 2 sheet, huruf besar tak konsisten)
