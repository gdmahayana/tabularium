use rusqlite::Connection;
use rust_xlsxwriter::*;

pub fn export_shs_rencana(conn: &Connection, tahun_anggaran_id: i64, output_path: &str) -> Result<usize, String> {
    let mut stmt = conn.prepare(
        "SELECT jenis, kode, uraian, spesifikasi, satuan, harga FROM shs WHERE tahun_anggaran_id = ?1 AND sumber = 'rencana' ORDER BY jenis, kode"
    ).map_err(|e| e.to_string())?;

    let rows: Vec<(String, String, String, Option<String>, String, f64)> = stmt
        .query_map(rusqlite::params![tahun_anggaran_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let count = rows.len();
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();

    let fmt_header = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x1e3a5f))
        .set_font_color(Color::White)
        .set_border(FormatBorder::Thin);
    let fmt_number = Format::new().set_num_format("#,##0");
    let fmt_border = Format::new().set_border(FormatBorder::Thin);

    let headers = ["No", "Jenis", "Kode", "Uraian", "Spesifikasi", "Satuan", "Harga Satuan"];
    for (i, h) in headers.iter().enumerate() {
        sheet.write_with_format(0, i as u16, *h, &fmt_header).map_err(|e| e.to_string())?;
    }
    sheet.set_column_width(0, 5).map_err(|e| e.to_string())?;
    sheet.set_column_width(1, 8).map_err(|e| e.to_string())?;
    sheet.set_column_width(2, 30).map_err(|e| e.to_string())?;
    sheet.set_column_width(3, 40).map_err(|e| e.to_string())?;
    sheet.set_column_width(4, 30).map_err(|e| e.to_string())?;
    sheet.set_column_width(5, 12).map_err(|e| e.to_string())?;
    sheet.set_column_width(6, 18).map_err(|e| e.to_string())?;

    for (i, (jenis, kode, uraian, spesifikasi, satuan, harga)) in rows.iter().enumerate() {
        let row = (i + 1) as u32;
        sheet.write_with_format(row, 0, (i + 1) as u32, &fmt_border).map_err(|e| e.to_string())?;
        sheet.write_with_format(row, 1, jenis.as_str(), &fmt_border).map_err(|e| e.to_string())?;
        sheet.write_with_format(row, 2, kode.as_str(), &fmt_border).map_err(|e| e.to_string())?;
        sheet.write_with_format(row, 3, uraian.as_str(), &fmt_border).map_err(|e| e.to_string())?;
        sheet.write_with_format(row, 4, spesifikasi.as_deref().unwrap_or(""), &fmt_border).map_err(|e| e.to_string())?;
        sheet.write_with_format(row, 5, satuan.as_str(), &fmt_border).map_err(|e| e.to_string())?;
        sheet.write_with_format(row, 6, *harga, &fmt_number).map_err(|e| e.to_string())?;
    }

    workbook.save(output_path).map_err(|e| e.to_string())?;
    Ok(count)
}

#[derive(Debug)]
struct ExportRow {
    program_kode:     String,
    program_uraian:   String,
    kegiatan_kode:    String,
    kegiatan_uraian:  String,
    sub_kode:         String,
    sub_uraian:       String,
    dsk_id:           i64,
    dsk_pagu:         f64,
    rek_kode:         String,
    rek_uraian:       String,
    peng_label:       String,
    peng_jenis:       String,
    ket_label:        String,
    item_uraian:      String,
    item_spesifikasi: Option<String>,
    item_volume:      f64,
    item_satuan:      String,
    item_koef2:       Option<f64>,
    item_satuan2:     Option<String>,
    item_koef3:       Option<f64>,
    item_satuan3:     Option<String>,
    item_koef4:       Option<f64>,
    item_satuan4:     Option<String>,
    item_harga:       f64,
    item_ppn:         f64,
}

fn hitung_jumlah(row: &ExportRow) -> f64 {
    let koef = row.item_volume
        * row.item_koef2.unwrap_or(1.0)
        * row.item_koef3.unwrap_or(1.0)
        * row.item_koef4.unwrap_or(1.0);
    koef * row.item_harga * (1.0 + row.item_ppn / 100.0)
}

fn fmt_koef_str(v: f64, s: Option<&str>) -> String {
    match s {
        Some(sat) if !sat.is_empty() => format!("{} {}", format_angka(v), sat),
        _ => String::new(),
    }
}

fn format_angka(v: f64) -> String {
    if v.fract() == 0.0 {
        let n = v as i64;
        let s = n.to_string();
        let mut result = String::new();
        for (i, c) in s.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 { result.push('.'); }
            result.push(c);
        }
        result.chars().rev().collect()
    } else {
        format!("{:.2}", v)
    }
}

pub fn export_kertas_kerja(
    conn: &Connection,
    dokumen_id: i64,
    output_path: &str,
) -> Result<(), String> {
    let sql = "
        SELECT
            mp.kode, mp.uraian,
            mk.kode, mk.uraian,
            msk.kode, msk.uraian,
            dsk.id, dsk.pagu,
            r.kode, r.uraian,
            rp.label, rp.jenis,
            rk.label,
            ri.uraian, ri.spesifikasi,
            ri.volume, ri.satuan,
            ri.koef2, ri.satuan2,
            ri.koef3, ri.satuan3,
            ri.koef4, ri.satuan4,
            ri.harga, ri.ppn
        FROM dokumen_sub_kegiatan dsk
        JOIN master_sub_kegiatan msk ON msk.id = dsk.master_sub_kegiatan_id
        JOIN master_kegiatan mk ON mk.id = msk.master_kegiatan_id
        JOIN master_program mp ON mp.id = mk.master_program_id
        JOIN rincian_rekening rrek ON rrek.dokumen_sub_kegiatan_id = dsk.id
        JOIN rekening r ON r.id = rrek.rekening_id
        JOIN rincian_pengelompokan rp ON rp.rincian_rekening_id = rrek.id
        JOIN rincian_keterangan rk ON rk.rincian_pengelompokan_id = rp.id
        JOIN rincian_item ri ON ri.rincian_keterangan_id = rk.id
        WHERE dsk.dokumen_id = ?1
        ORDER BY mp.kode, mk.kode, msk.kode, dsk.id,
                 rrek.urutan, rp.urutan, rk.urutan, ri.urutan
    ";

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows: Vec<ExportRow> = stmt
        .query_map(rusqlite::params![dokumen_id], |row| {
            Ok(ExportRow {
                program_kode:     row.get(0)?,
                program_uraian:   row.get(1)?,
                kegiatan_kode:    row.get(2)?,
                kegiatan_uraian:  row.get(3)?,
                sub_kode:         row.get(4)?,
                sub_uraian:       row.get(5)?,
                dsk_id:           row.get(6)?,
                dsk_pagu:         row.get(7)?,
                rek_kode:         row.get(8)?,
                rek_uraian:       row.get(9)?,
                peng_label:       row.get(10)?,
                peng_jenis:       row.get(11)?,
                ket_label:        row.get(12)?,
                item_uraian:      row.get(13)?,
                item_spesifikasi: row.get(14)?,
                item_volume:      row.get(15)?,
                item_satuan:      row.get(16)?,
                item_koef2:       row.get(17)?,
                item_satuan2:     row.get(18)?,
                item_koef3:       row.get(19)?,
                item_satuan3:     row.get(20)?,
                item_koef4:       row.get(21)?,
                item_satuan4:     row.get(22)?,
                item_harga:       row.get(23)?,
                item_ppn:         row.get(24)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut workbook = Workbook::new();

    let navy      = Color::RGB(0x1e3a5f);
    let biru_tua  = Color::RGB(0x2e5fa3);
    let biru_muda = Color::RGB(0xdce6f1);
    let abu       = Color::RGB(0xf2f2f2);

    let fmt_title        = Format::new().set_bold().set_font_size(12.0).set_font_color(navy);
    let fmt_program      = Format::new().set_bold().set_background_color(navy).set_font_color(Color::White).set_border(FormatBorder::Thin);
    let fmt_kegiatan     = Format::new().set_bold().set_background_color(biru_tua).set_font_color(Color::White).set_border(FormatBorder::Thin);
    let fmt_sub          = Format::new().set_bold().set_background_color(biru_muda).set_font_color(navy).set_border(FormatBorder::Thin);
    let fmt_rekening     = Format::new().set_bold().set_background_color(abu).set_border(FormatBorder::Thin);
    let fmt_peng         = Format::new().set_italic().set_indent(1).set_border(FormatBorder::Thin);
    let fmt_ket          = Format::new().set_indent(2).set_border(FormatBorder::Thin);
    let fmt_item         = Format::new().set_indent(3).set_border(FormatBorder::Thin);
    let fmt_number       = Format::new().set_num_format("#,##0").set_border(FormatBorder::Thin);
    let fmt_number_bold  = Format::new().set_bold().set_num_format("#,##0").set_border(FormatBorder::Thin);
    let fmt_total_sub    = Format::new().set_bold().set_background_color(biru_muda).set_num_format("#,##0").set_border(FormatBorder::Thin);
    let fmt_selisih_ok   = Format::new().set_bold().set_font_color(Color::RGB(0x217346)).set_num_format("#,##0").set_border(FormatBorder::Thin);
    let fmt_selisih_warn = Format::new().set_bold().set_font_color(Color::RGB(0xc00000)).set_num_format("#,##0").set_border(FormatBorder::Thin);
    let fmt_blank_sub    = Format::new().set_bold().set_background_color(biru_muda).set_border(FormatBorder::Thin);
    let fmt_blank_prog   = Format::new().set_bold().set_background_color(navy).set_border(FormatBorder::Thin);

    let sheet = workbook.add_worksheet();
    sheet.set_name("Rincian Anggaran").map_err(|e| e.to_string())?;
    sheet.set_column_width(0, 45).map_err(|e| e.to_string())?;
    sheet.set_column_width(1, 25).map_err(|e| e.to_string())?;
    sheet.set_column_width(2, 15).map_err(|e| e.to_string())?;
    sheet.set_column_width(3, 12).map_err(|e| e.to_string())?;
    sheet.set_column_width(4, 12).map_err(|e| e.to_string())?;
    sheet.set_column_width(5, 12).map_err(|e| e.to_string())?;
    sheet.set_column_width(6, 18).map_err(|e| e.to_string())?;
    sheet.set_column_width(7, 18).map_err(|e| e.to_string())?;

    let mut r: u32 = 0;
    sheet.merge_range(r, 0, r, 7, "KERTAS KERJA RINCIAN ANGGARAN", &fmt_title).map_err(|e| e.to_string())?;
    r += 1;
    sheet.write_blank(r, 0, &Format::new()).map_err(|e| e.to_string())?;
    r += 1;

    for (c, h) in ["Uraian","Spesifikasi","Vol x Sat","Koef 2","Koef 3","Koef 4","Harga Satuan","Jumlah"].iter().enumerate() {
        sheet.write_with_format(r, c as u16, *h, &fmt_program).map_err(|e| e.to_string())?;
    }
    r += 1;

    let mut last_program  = String::new();
    let mut last_kegiatan = String::new();
    let mut last_sub      = String::new();
    let mut last_rek      = String::new();
    let mut last_peng     = String::new();
    let mut last_ket      = String::new();
    let mut last_pagu:  f64 = 0.0;
    let mut total_ket:  f64 = 0.0;
    let mut total_peng: f64 = 0.0;
    let mut total_rek:  f64 = 0.0;
    let mut total_sub:  f64 = 0.0;

    macro_rules! flush_ket {
        () => { if !last_ket.is_empty() {
            let lbl = last_ket.split('|').last().unwrap_or("").to_string();
            sheet.merge_range(r, 0, r, 6, &format!("  Total {}", lbl), &fmt_number_bold).map_err(|e| e.to_string())?;
            sheet.write_with_format(r, 7, total_ket, &fmt_number_bold).map_err(|e| e.to_string())?;
            r += 1; total_ket = 0.0;
        }}
    }
    macro_rules! flush_peng {
        () => { if !last_peng.is_empty() {
            let lbl = last_peng.split('|').last().unwrap_or("").to_string();
            sheet.merge_range(r, 0, r, 6, &format!("  Subtotal {}", lbl), &fmt_number_bold).map_err(|e| e.to_string())?;
            sheet.write_with_format(r, 7, total_peng, &fmt_number_bold).map_err(|e| e.to_string())?;
            r += 1; total_peng = 0.0;
        }}
    }
    macro_rules! flush_rek {
        () => { if !last_rek.is_empty() {
            let parts: Vec<&str> = last_rek.split('|').collect();
            let rek_lbl = if parts.len() >= 2 {
                format!("{} - {}", parts[parts.len()-2], parts[parts.len()-1])
            } else { last_rek.clone() };
            sheet.merge_range(r, 0, r, 6, &format!("Total {}", rek_lbl), &fmt_rekening).map_err(|e| e.to_string())?;
            sheet.write_with_format(r, 7, total_rek, &fmt_number_bold).map_err(|e| e.to_string())?;
            r += 1; total_rek = 0.0;
        }}
    }
    macro_rules! flush_sub {
        () => { if !last_sub.is_empty() {
            let sub_lbl = last_sub.split('|').nth(1).unwrap_or("").to_string();
            sheet.merge_range(r, 0, r, 6, &format!("TOTAL RINCIAN {}", sub_lbl), &fmt_blank_sub).map_err(|e| e.to_string())?;
            sheet.write_with_format(r, 7, total_sub, &fmt_total_sub).map_err(|e| e.to_string())?;
            r += 1;
            sheet.merge_range(r, 0, r, 6, &format!("PAGU {}", sub_lbl), &fmt_blank_sub).map_err(|e| e.to_string())?;
            sheet.write_with_format(r, 7, last_pagu, &fmt_total_sub).map_err(|e| e.to_string())?;
            r += 1;
            let selisih = last_pagu - total_sub;
            let f_sel = if selisih >= 0.0 { &fmt_selisih_ok } else { &fmt_selisih_warn };
            sheet.merge_range(r, 0, r, 6, &format!("SELISIH {}", sub_lbl), f_sel).map_err(|e| e.to_string())?;
            sheet.write_with_format(r, 7, selisih, f_sel).map_err(|e| e.to_string())?;
            r += 1; r += 1;
            total_sub = 0.0;
        }}
    }

    for rd in &rows {
        let jumlah   = hitung_jumlah(rd);
        let sub_key  = format!("{}|{}", rd.dsk_id, rd.sub_kode);
        let rek_key  = format!("{}|{}|{}", sub_key, rd.rek_kode, rd.rek_uraian);
        let peng_key = format!("{}|{}", rek_key, rd.peng_label);
        let ket_key  = format!("{}|{}", peng_key, rd.ket_label);

        if rd.program_kode != last_program {
            flush_ket!(); flush_peng!(); flush_rek!(); flush_sub!();
            sheet.merge_range(r, 0, r, 7, &format!("PROGRAM: {} \u{2014} {}", rd.program_kode, rd.program_uraian), &fmt_program).map_err(|e| e.to_string())?;
            r += 1;
            last_program = rd.program_kode.clone();
            last_kegiatan = String::new();
        }
        if rd.kegiatan_kode != last_kegiatan {
            flush_ket!(); flush_peng!(); flush_rek!(); flush_sub!();
            sheet.merge_range(r, 0, r, 7, &format!("  KEGIATAN: {} \u{2014} {}", rd.kegiatan_kode, rd.kegiatan_uraian), &fmt_kegiatan).map_err(|e| e.to_string())?;
            r += 1;
            last_kegiatan = rd.kegiatan_kode.clone();
            last_sub = String::new();
        }
        if sub_key != last_sub {
            flush_ket!(); flush_peng!(); flush_rek!(); flush_sub!();
            sheet.merge_range(r, 0, r, 7, &format!("    SUB KEGIATAN: {} \u{2014} {}", rd.sub_kode, rd.sub_uraian), &fmt_sub).map_err(|e| e.to_string())?;
            r += 1;
            sheet.merge_range(r, 0, r, 6, "    Pagu:", &fmt_blank_sub).map_err(|e| e.to_string())?;
            sheet.write_with_format(r, 7, rd.dsk_pagu, &fmt_total_sub).map_err(|e| e.to_string())?;
            r += 1;
            last_sub  = sub_key.clone();
            last_pagu = rd.dsk_pagu;
            last_rek  = String::new();
        }
        if rek_key != last_rek {
            flush_ket!(); flush_peng!(); flush_rek!();
            sheet.merge_range(r, 0, r, 7, &format!("      {} \u{2014} {}", rd.rek_kode, rd.rek_uraian), &fmt_rekening).map_err(|e| e.to_string())?;
            r += 1;
            last_rek  = rek_key.clone();
            last_peng = String::new();
        }
        if peng_key != last_peng {
            flush_ket!(); flush_peng!();
            let jenis_lbl = if rd.peng_jenis == "pemaketan" { "Paket" } else { "#" };
            sheet.merge_range(r, 0, r, 7, &format!("        [{}] {}", jenis_lbl, rd.peng_label), &fmt_peng).map_err(|e| e.to_string())?;
            r += 1;
            last_peng = peng_key.clone();
            last_ket  = String::new();
        }
        if ket_key != last_ket {
            flush_ket!();
            sheet.merge_range(r, 0, r, 7, &format!("          {}", rd.ket_label), &fmt_ket).map_err(|e| e.to_string())?;
            r += 1;
            last_ket = ket_key.clone();
        }

        let vol_sat = format!("{} {}", format_angka(rd.item_volume), rd.item_satuan);
        let k2 = fmt_koef_str(rd.item_koef2.unwrap_or(1.0), rd.item_satuan2.as_deref());
        let k3 = fmt_koef_str(rd.item_koef3.unwrap_or(1.0), rd.item_satuan3.as_deref());
        let k4 = fmt_koef_str(rd.item_koef4.unwrap_or(1.0), rd.item_satuan4.as_deref());

        sheet.write_with_format(r, 0, &format!("            {}", rd.item_uraian), &fmt_item).map_err(|e| e.to_string())?;
        sheet.write_with_format(r, 1, rd.item_spesifikasi.as_deref().unwrap_or(""), &fmt_item).map_err(|e| e.to_string())?;
        sheet.write_with_format(r, 2, &vol_sat, &fmt_item).map_err(|e| e.to_string())?;
        sheet.write_with_format(r, 3, &k2, &fmt_item).map_err(|e| e.to_string())?;
        sheet.write_with_format(r, 4, &k3, &fmt_item).map_err(|e| e.to_string())?;
        sheet.write_with_format(r, 5, &k4, &fmt_item).map_err(|e| e.to_string())?;
        sheet.write_with_format(r, 6, rd.item_harga, &fmt_number).map_err(|e| e.to_string())?;
        sheet.write_with_format(r, 7, jumlah, &fmt_number).map_err(|e| e.to_string())?;
        r += 1;

        total_ket  += jumlah;
        total_peng += jumlah;
        total_rek  += jumlah;
        total_sub  += jumlah;
    }
    flush_ket!(); flush_peng!(); flush_rek!(); flush_sub!();

    // Sheet 2: Rekapitulasi
    let sheet2 = workbook.add_worksheet();
    sheet2.set_name("Rekapitulasi").map_err(|e| e.to_string())?;
    sheet2.set_column_width(0, 15).map_err(|e| e.to_string())?;
    sheet2.set_column_width(1, 40).map_err(|e| e.to_string())?;
    sheet2.set_column_width(2, 20).map_err(|e| e.to_string())?;
    sheet2.set_column_width(3, 20).map_err(|e| e.to_string())?;
    sheet2.set_column_width(4, 20).map_err(|e| e.to_string())?;

    for (c, h) in ["Kode","Sub Kegiatan","Pagu","Total Rincian","Selisih"].iter().enumerate() {
        sheet2.write_with_format(0, c as u16, *h, &fmt_program).map_err(|e| e.to_string())?;
    }

    struct RekapItem { kode: String, uraian: String, pagu: f64, total: f64 }
    let mut rekap: Vec<RekapItem> = Vec::new();
    let mut cur_dsk: i64 = -1;
    for rd in &rows {
        if rd.dsk_id != cur_dsk {
            rekap.push(RekapItem { kode: rd.sub_kode.clone(), uraian: rd.sub_uraian.clone(), pagu: rd.dsk_pagu, total: 0.0 });
            cur_dsk = rd.dsk_id;
        }
        if let Some(last) = rekap.last_mut() { last.total += hitung_jumlah(rd); }
    }

    let mut grand_pagu:  f64 = 0.0;
    let mut grand_total: f64 = 0.0;
    for (i, item) in rekap.iter().enumerate() {
        let rr = (i + 1) as u32;
        let selisih = item.pagu - item.total;
        let f_sel = if selisih >= 0.0 { &fmt_selisih_ok } else { &fmt_selisih_warn };
        sheet2.write_with_format(rr, 0, &item.kode,  &fmt_item).map_err(|e| e.to_string())?;
        sheet2.write_with_format(rr, 1, &item.uraian, &fmt_item).map_err(|e| e.to_string())?;
        sheet2.write_with_format(rr, 2, item.pagu,    &fmt_number).map_err(|e| e.to_string())?;
        sheet2.write_with_format(rr, 3, item.total,   &fmt_number).map_err(|e| e.to_string())?;
        sheet2.write_with_format(rr, 4, selisih,      f_sel).map_err(|e| e.to_string())?;
        grand_pagu  += item.pagu;
        grand_total += item.total;
    }
    let gr = rekap.len() as u32 + 1;
    let grand_selisih = grand_pagu - grand_total;
    let f_gs = if grand_selisih >= 0.0 { &fmt_selisih_ok } else { &fmt_selisih_warn };
    sheet2.write_with_format(gr, 0, "TOTAL",       &fmt_program).map_err(|e| e.to_string())?;
    sheet2.write_blank(gr, 1,                      &fmt_blank_prog).map_err(|e| e.to_string())?;
    sheet2.write_with_format(gr, 2, grand_pagu,    &fmt_number_bold).map_err(|e| e.to_string())?;
    sheet2.write_with_format(gr, 3, grand_total,   &fmt_number_bold).map_err(|e| e.to_string())?;
    sheet2.write_with_format(gr, 4, grand_selisih, f_gs).map_err(|e| e.to_string())?;

    workbook.save(output_path).map_err(|e| e.to_string())?;
    Ok(())
}