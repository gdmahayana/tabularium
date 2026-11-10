## Project Configuration

- **Language**: TypeScript
- **Package Manager**: npm
- **Add-ons**: eslint, prettier

---

# AGENTS.md — Panduan untuk asisten AI pengembang TABULARIUM

Baca berkas ini sebelum mengubah apa pun. Jika instruksi pengguna bertentangan dengan berkas ini, tanyakan dulu.

## Apa ini

TABULARIUM: platform desktop local-first untuk pengelolaan data, Statistik Sektoral, dan penyiapan
input ke SDI/Satu Data. Pengguna utama: aparatur pemerintahan nonteknis. Antarmuka **berbahasa
Indonesia**; identifier, kode, nama berkas, dan komentar kode **berbahasa Inggris**.

## Aturan keras (tidak boleh dilanggar)

1. **Hanya Rust yang menulis ke database.** Python sidecar tidak pernah membuka koneksi tulis ke SQLite.
   Sidecar boleh membaca read-only atau menerima berkas sementara (Parquet/CSV).
2. **Frontend tidak mengakses database.** Frontend hanya memanggil Tauri Commands.
3. **Setiap perubahan nilai data lewat jalur audit.** Tidak ada penulisan langsung yang melewati audit trail.
4. **Formula indikator tidak mengeksekusi kode bebas.** Tidak ada `eval`/`exec`; ekspresi diurai dan dievaluasi mesin sendiri di Rust.
5. **Sidecar tanpa akses jaringan** dan tidak menjalankan kode dari data pengguna.
6. **Nilai kosong bukan nol.** Bedakan: kosong, tidak berlaku, tidak diketahui, nol sebenarnya.
7. **Tidak ada data nyata** di repo, di percakapan AI, atau di fixture tes. Hanya data sintetis di `examples/`.
8. **Struktur isian portal tidak ditanam di kode.** Gunakan Profil Portal di `profiles/`.
9. **Jangan menambah dependency tanpa alasan tertulis**, dan periksa lisensinya kompatibel dengan AGPL-3.0 (MIT/Apache-2.0/BSD boleh).
10. **Tidak ada fitur AI/cloud** di dalam produk.

## Cara kerja

- **Spesifikasi sebelum kode:** modul baru mulai dari `docs/spec/`; keputusan teknis dicatat sebagai ADR di `docs/adr/`.
- **Irisan kecil:** satu fitur, satu tes, satu commit. Hindari perubahan besar sekaligus.
- **Tes dulu / bersama kode:** kode tanpa tes tidak diterima. Prioritas: _golden file_ impor Excel
  (`tests/golden-files/`) dan _property test_ mesin indikator.
- **Area berisiko, butuh tinjauan manusia:** migrasi database, audit trail, penghitungan indikator,
  apa pun yang menulis ke data pengguna. Tandai jelas di deskripsi commit.
- Pesan error untuk pengguna: bahasa Indonesia sederhana + langkah perbaikan.
- Hasil turunan (indikator) ditandai _usang_ saat sumber berubah; jangan menimpa diam-diam.

## Perintah (diisi seiring implementasi)

```text
# Rust      : cargo test --manifest-path src-tauri/Cargo.toml
# Python    : cd sidecar && uv run pytest
# Frontend  : npm run check && npm run test
```

## Konvensi

- Commit: Conventional Commits (`feat:`, `fix:`, `docs:`, `test:`, `chore:`), dengan sign-off DCO (`git commit -s`).
- Setiap berkas sumber berisi `SPDX-License-Identifier: AGPL-3.0-only` (varian final menunggu ADR).
- Rust: `cargo fmt`, `cargo clippy -- -D warnings`. Python: `ruff`. TS: `eslint` + `prettier`.
- Nama tabel/kolom database: `snake_case`, bahasa Inggris.

## Peta folder

`src/` frontend Svelte · `src-tauri/` Rust core · `sidecar/` Python · `profiles/` profil portal ·
`docs/adr|spec|normative/` · `examples/` data sintetis · `tests/` integrasi + golden files

## Tahap saat ini

Tahap 0: Discovery & Spike (A: model observasi, B: kemas sidecar, C: Excel berantakan, D: mesin ekspresi).
Jangan memulai fitur Tahap 1+ sebelum keempat spike punya ADR.
