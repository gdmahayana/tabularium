# SPDX-License-Identifier: AGPL-3.0-only
import json, tempfile, pathlib
import openpyxl
from openpyxl.styles import Font, PatternFill, Alignment

def _generate_template(params):
    dataset_name = params["dataset_name"]
    period_value = params.get("period_value", "")
    columns = params["columns"]
    out_path = params["out_path"]
    wb = openpyxl.Workbook()
    ws = wb.active
    ws.title = dataset_name[:31]
    info_cols = ["unit_wilayah", "period_value"] if period_value else ["unit_wilayah"]
    var_cols = [c["code"] for c in columns]
    all_cols = info_cols + var_cols
    label_cols = (["Nama Wilayah", "Periode"] if period_value else ["Nama Wilayah"])
    label_cols += [c["name"] for c in columns]
    unit_cols = ["", ""] if period_value else [""]
    unit_cols += [c.get("unit", "") for c in columns]
    blue = PatternFill("solid", fgColor="4472C4")
    grey = PatternFill("solid", fgColor="D9D9D9")
    bold_white = Font(bold=True, color="FFFFFF")
    bold_black = Font(bold=True)
    for col, (lbl, unt) in enumerate(zip(label_cols, unit_cols), 1):
        h = ws.cell(row=1, column=col, value=lbl)
        h.fill = blue
        h.font = bold_white
        h.alignment = Alignment(horizontal="center", wrap_text=True)
        u = ws.cell(row=2, column=col, value=unt if unt else "")
        u.fill = grey
        u.font = bold_black
        u.alignment = Alignment(horizontal="center")
    for col, code in enumerate(all_cols, 1):
        c = ws.cell(row=3, column=col, value=code)
        c.font = Font(italic=True, color="808080")
    if period_value:
        for row in range(4, 20):
            ws.cell(row=row, column=2, value=period_value)
    ws.column_dimensions["A"].width = 25
    for i in range(2, len(all_cols) + 1):
        ws.column_dimensions[openpyxl.utils.get_column_letter(i)].width = 18
    ws.row_dimensions[1].height = 30
    wb.save(out_path)
    return {"out_path": out_path, "columns": all_cols}
from tabularium_sidecar.importer_excel import read_excel_file

def _inspect_file(params):
    path = params["path"]
    n_prev = int(params.get("preview_rows", 5))
    sheets = read_excel_file(path)
    result = {}
    for name, df in sheets.items():
        result[name] = {"columns": list(df.columns), "row_count": len(df), "preview": json.loads(df.head(n_prev).to_json(orient="records", force_ascii=False))}
    return result

def _read_table(params):
    path = params["path"]
    sheet = params["sheet"]
    sheets = read_excel_file(path)
    if sheet not in sheets:
        raise ValueError(f"Sheet tidak ditemukan: {sheet}")
    df = sheets[sheet]
    out_dir = pathlib.Path(params.get("out_dir") or tempfile.gettempdir())
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / ("import_" + pathlib.Path(path).stem + "_" + sheet + ".jsonl")
    rows = json.loads(df.to_json(orient="records", force_ascii=False))
    fh = open(out_path, "w", encoding="utf-8")
    [fh.write(json.dumps(r, ensure_ascii=False) + chr(10)) for r in rows]
    fh.close()
    return {"out_path": str(out_path), "columns": list(df.columns), "row_count": len(df)}

METHODS = {"inspect_file": _inspect_file, "read_table": _read_table, "generate_template": _generate_template}

def dispatch(method, params):
    if method not in METHODS:
        raise ValueError("Method tidak dikenal: " + method)
    return METHODS[method](params)
