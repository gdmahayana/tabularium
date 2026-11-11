# SPDX-License-Identifier: AGPL-3.0-only
import re, pandas as pd
from openpyxl import load_workbook

SKIP = re.compile(r"(subtotal|total|jumlah|rekapitulasi|sumber|keterangan)", re.IGNORECASE)

def _is_skip(row):
    return bool(SKIP.search(" ".join(str(v) for v in row if v is not None)))

def read_template_sheet(ws):
    """Baca sheet template TABULARIUM: baris 1=label, 2=satuan, 3=kode, 4+=data."""
    rows = list(ws.iter_rows(values_only=True))
    if len(rows) < 4:
        return pd.DataFrame()
    codes = [str(v).strip() if v else f"col_{i}" for i, v in enumerate(rows[2])]
    data = [
        list(r) for r in rows[3:]
        if any(v is not None for v in r) and not _is_skip(r)
    ]
    df = pd.DataFrame(data, columns=codes)
    df = df.loc[:, ~df.columns.str.startswith("col_")]
    return df.reset_index(drop=True)

def read_messy_sheet(ws, min_text=3):
    """Fallback: baca sheet tidak terstruktur (bukan template TABULARIUM)."""
    rows = list(ws.iter_rows(values_only=True))
    best_i, best_score = 0, 0
    for i, row in enumerate(rows[:10]):
        if _is_skip(row):
            continue
        score = sum(1 for v in row if v is not None and isinstance(v, str) and v.strip())
        if score > best_score:
            best_score, best_i = score, i
    hdr = []
    seen = {}
    for v in rows[best_i]:
        nm = re.sub(r"[	 ]+", "_", str(v or "col").strip().lower())
        nm = re.sub(r"[^a-z0-9_]", "", nm) or "col"
        if nm in seen:
            seen[nm] += 1
            nm = f"{nm}_{seen[nm]}"
        else:
            seen[nm] = 0
        hdr.append(nm)
    data = [
        list(r) for r in rows[best_i + 1:]
        if any(v is not None for v in r) and not _is_skip(r)
    ]
    return pd.DataFrame(data, columns=hdr).dropna(axis=1, how="all").reset_index(drop=True)

def _is_tabularium_template(ws):
    """Deteksi template TABULARIUM: baris 3 kolom A berisi kode teknis italic."""
    rows = list(ws.iter_rows(values_only=True))
    if len(rows) < 3:
        return False
    row3 = rows[2]
    return row3[0] is not None and str(row3[0]).strip() == "unit_wilayah"

def read_excel_file(path, min_text=3):
    wb = load_workbook(path, data_only=True)
    res = {}
    for name in wb.sheetnames:
        ws = wb[name]
        if _is_tabularium_template(ws):
            df = read_template_sheet(ws)
            source = "template"
        else:
            df = read_messy_sheet(ws, min_text)
            source = "messy"
        res[name] = df
        print(f"  Sheet [{name}] ({source}): {len(df)} baris x {len(df.columns)} kolom — {list(df.columns)}")
    return res
