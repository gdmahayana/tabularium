# ADR-0002: Mesin Ekspresi Indikator di Rust

- **Status:** Diterima (Spike D lulus 2026-11-10)
- **Tanggal:** 2026-11-10

## Konteks
Formula indikator harus dievaluasi dengan aman tanpa mengeksekusi kode bebas.
Mesin ditulis di Rust (bukan Python) agar berjalan di dalam Tauri core,
sesuai aturan AGENTS.md: hanya Rust yang menulis ke database.

## Fungsi yang didukung (MVP)
- Literal angka
- Variabel bernama (nilai diambil dari konteks Map)
- Operator: +, -, *, / (pembagian aman: 0 bila pembagi nol)
- Fungsi: sum([a,b,...]), avg([a,b,...]), count([a,b,...]), ratio(p,q), if(cond,yes,no)
- Tidak ada eval/exec; parser dan evaluator ditulis sendiri

## Kriteria lulus Spike D
- Formula sederhana (a + b, ratio(a,b)*100) dievaluasi dengan benar
- Pembagian dengan nol menghasilkan None/null, bukan panic
- Variabel tidak dikenal menghasilkan error yang jelas
- Property test: formula komuttatif dan asosiatif menghasilkan hasil konsisten

## Bukti / hasil uji
Spike D lulus 2026-11-10:
- 9/9 unit test hijau (literal, add, div-zero, ratio, sum, avg, if, unknown-var, formula PKK)
- Pembagian nol menghasilkan None, bukan panic
- Variabel tidak dikenal menghasilkan EvalError::UnknownVar
- Parser rekursif tanpa eval/exec
- Kode: src-tauri/src/expression.rs
