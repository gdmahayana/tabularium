# SPDX-License-Identifier: AGPL-3.0-only
import sys, pathlib
sys.path.insert(0, str(pathlib.Path(__file__).parent.parent / "src"))
from tabularium_sidecar.importer_excel import read_excel_file

EXAMPLES = pathlib.Path(__file__).parent.parent.parent / "examples"

def test_pkk_messy_header():
    """File PKK berantakan: header baris 2, bukan baris judul."""
    result = read_excel_file(str(EXAMPLES / "pkk_2024_dummy.xlsx"))
    assert len(result) == 1
    sheet_name = list(result.keys())[0]
    df = result[sheet_name]
    assert "desa" in df.columns
    assert "jumlah_kelompok_pkk" in df.columns
    assert "jumlah_kelompok_binaan" in df.columns
    assert "jumlah_kader" in df.columns

def test_pkk_row_count():
    """Semua baris data terbaca, baris total/jumlah diabaikan."""
    result = read_excel_file(str(EXAMPLES / "pkk_2024_dummy.xlsx"))
    df = list(result.values())[0]
    # minimal 10 desa, maksimal 20 (data dummy 17 desa)
    assert 10 <= len(df) <= 20

def test_pkk_nilai_numerik():
    """Nilai jumlah kelompok harus numerik dan positif."""
    result = read_excel_file(str(EXAMPLES / "pkk_2024_dummy.xlsx"))
    df = list(result.values())[0]
    assert (df["jumlah_kelompok_pkk"].dropna() > 0).all()
    assert (df["jumlah_kader"].dropna() > 0).all()
