# SPDX-License-Identifier: AGPL-3.0-only
"""Generate synthetic (dummy) data. All names/numbers are fictional. Seeded for reproducibility."""
import csv, random
from openpyxl import Workbook
from openpyxl.styles import Font, Alignment
from openpyxl.utils import get_column_letter

random.seed(42)
KEC = {"Kecamatan Contoh A": 4, "Kecamatan Contoh B": 5, "Kecamatan Contoh C": 3}

# --- master wilayah ---
wilayah = []
for ki, (kec, n) in enumerate(KEC.items(), start=1):
    for di in range(1, n + 1):
        wilayah.append({"kode_kecamatan": f"K{ki:02d}", "kecamatan": kec,
                        "kode_desa": f"K{ki:02d}D{di:02d}", "desa": f"Desa Contoh {chr(64+ki)}{di}"})
with open("examples/wilayah_dummy.csv", "w", newline="", encoding="utf-8") as f:
    w = csv.DictWriter(f, fieldnames=list(wilayah[0])); w.writeheader(); w.writerows(wilayah)

# --- PKK per desa per tahun (grain: desa x tahun) ---
rows = {}
for tahun in (2024, 2025):
    for d in wilayah:
        kel = random.randint(6, 20)
        binaan = random.randint(max(0, kel - 6), kel)
        kader = kel * random.randint(3, 6)
        rows[(d["kode_desa"], tahun)] = (kel, binaan, kader)
# sengaja satu anomali (binaan > kelompok) untuk menguji validasi
rows[("K02D03", 2025)] = (8, 11, 30)
# sengaja satu sel kosong
with open("examples/pkk_dummy_long.csv", "w", newline="", encoding="utf-8") as f:
    w = csv.writer(f)
    w.writerow(["kode_desa", "desa", "tahun", "jumlah_kelompok_pkk", "jumlah_kelompok_binaan", "jumlah_kader"])
    for d in wilayah:
        for tahun in (2024, 2025):
            kel, bin_, kad = rows[(d["kode_desa"], tahun)]
            if d["kode_desa"] == "K01D02" and tahun == 2025:
                kad = ""
            w.writerow([d["kode_desa"], d["desa"], tahun, kel, bin_, kad])

# --- Excel berantakan (Spike C): judul bertingkat, sel digabung, subtotal, 2 sheet ---
wb = Workbook()
for sheet_i, tahun in enumerate((2024, 2025)):
    ws = wb.active if sheet_i == 0 else wb.create_sheet()
    ws.title = f"Data {tahun}"
    ws["A1"] = f"REKAPITULASI PEMBINAAN PKK TAHUN {tahun}"; ws["A1"].font = Font(bold=True, size=14)
    ws.merge_cells("A1:F1")
    ws["A2"] = "Sumber: Bidang Contoh (data dummy)"; ws.merge_cells("A2:F2")
    ws["A4"], ws["B4"], ws["C4"], ws["E4"] = "No", "Kecamatan / Desa", "Kelompok PKK", "Kader"
    ws.merge_cells("A4:A5"); ws.merge_cells("B4:B5"); ws.merge_cells("C4:D4"); ws.merge_cells("E4:F4")
    ws["C5"], ws["D5"], ws["E5"], ws["F5"] = "Jumlah", "Binaan", "Jumlah", "(orang)"
    for c in "ABCDEF":
        ws[f"{c}4"].font = Font(bold=True); ws[f"{c}4"].alignment = Alignment(horizontal="center")
    r, no = 6, 1
    for kec in KEC:
        ws.cell(r, 2, kec).font = Font(bold=True); r += 1
        first = r
        for d in [x for x in wilayah if x["kecamatan"] == kec]:
            kel, bin_, kad = rows[(d["kode_desa"], tahun)]
            ws.cell(r, 1, no); ws.cell(r, 2, d["desa"].upper() if no % 4 == 0 else d["desa"])
            ws.cell(r, 3, kel); ws.cell(r, 4, bin_); ws.cell(r, 5, kad); ws.cell(r, 6, "orang")
            r += 1; no += 1
        ws.cell(r, 2, f"Subtotal {kec}").font = Font(italic=True)
        for col in (3, 4, 5):
            L = get_column_letter(col)
            ws.cell(r, col, f"=SUM({L}{first}:{L}{r-1})")
        r += 1
    ws.cell(r, 2, "TOTAL KABUPATEN").font = Font(bold=True)
    ws.column_dimensions["B"].width = 32
wb.save("tests/golden-files/pkk_messy_dummy.xlsx")

# --- Dataset kedua, struktur berbeda: Aset Desa (grain: 1 aset) ---
jenis = ["Balai Banjar", "Pura", "Kendaraan Operasional", "Tanah Kas Desa", "Posyandu"]
kondisi = ["Baik", "Rusak Ringan", "Rusak Berat"]
with open("examples/aset_desa_dummy.csv", "w", newline="", encoding="utf-8") as f:
    w = csv.writer(f)
    w.writerow(["id_aset", "kode_desa", "jenis_aset", "kondisi", "tahun_perolehan", "nilai_perolehan_rp"])
    for i in range(1, 61):
        d = random.choice(wilayah)
        w.writerow([f"A{i:03d}", d["kode_desa"], random.choice(jenis), random.choice(kondisi),
                    random.randint(2005, 2024), random.randint(5, 900) * 1_000_000])
print("ok")
