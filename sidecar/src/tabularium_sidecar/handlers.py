# SPDX-License-Identifier: AGPL-3.0-only
import json, tempfile, pathlib
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

METHODS = {"inspect_file": _inspect_file, "read_table": _read_table}

def dispatch(method, params):
    if method not in METHODS:
        raise ValueError("Method tidak dikenal: " + method)
    return METHODS[method](params)
