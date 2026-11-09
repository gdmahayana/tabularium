# SPDX-License-Identifier: AGPL-3.0-only
import re, pandas as pd
from openpyxl import load_workbook
SKIP = re.compile(r"(subtotal|total|jumlah|rekapitulasi|sumber|keterangan)", re.IGNORECASE)
def _skip(row): return bool(SKIP.search(" ".join(str(v) for v in row if v)))
def _header(ws, n=3):
    for i,row in enumerate(ws.iter_rows(values_only=True)):
        if len([v for v in row if v])>=n and not _skip(row): return i
    return 0
def read_messy_sheet(ws, n=3):
    rows=list(ws.iter_rows(values_only=True)); hi=_header(ws,n)
    rh=list(rows[hi]); sub=rows[hi+1] if hi+1<len(rows) else [None]*len(rh)
    hdr=[]; seen={}
    for t,b in zip(rh,sub):
        nm=(f"{t}_{b}" if t and b and str(t)!=str(b) else str(t or b or "col")).strip().lower()
        nm=re.sub(r"\s+","_",nm)
        if nm in seen: seen[nm]+=1; nm=f"{nm}_{seen[nm]}"
        else: seen[nm]=0
        hdr.append(nm)
    data=[list(r) for r in rows[hi+2:] if any(v is not None for v in r) and not _skip(r)]
    df=pd.DataFrame(data,columns=hdr).dropna(axis=1,how="all").reset_index(drop=True)
    return df
def read_excel_file(path, n=3):
    wb=load_workbook(path,data_only=True); res={}
    for name in wb.sheetnames:
        df=read_messy_sheet(wb[name],n); res[name]=df
        print(f"  Sheet [{name}]: {len(df)} baris x {len(df.columns)} kolom — {list(df.columns)}")
    return res
