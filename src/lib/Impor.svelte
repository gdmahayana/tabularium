<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { api } from "#lib/api";

  // --- State wizard ---
  type Langkah = "pilih_file" | "petakan" | "validasi" | "konfirmasi" | "selesai";
  let langkah = $state<Langkah>("pilih_file");

  let filePath = $state("");
  let sheetList = $state<string[]>([]);
  let sheetPilih = $state("");
  let preview = $state<Record<string,unknown>[]>([]);
  let kolomFile = $state<string[]>([]);

  let datasets = $state<{id:number;name:string;period_type:string}[]>([]);
  let datasetId = $state(0);
  let periodValue = $state("");
  let variables = $state<{id:number;name:string;code:string;data_type:string;unit:string|null}[]>([]);
  let pemetaan = $state<Record<string,number>>({}); // kolom file -> variable_id
  let kolomWilayah = $state("");

  let sessionId = $state(0);
  let rowsPath = $state("");
  let hasilValidasi = $state<{result:string;issues:{severity:string;message:string}[]}>({result:"",issues:[]});
  let hasilKomit = $state<{batch_id:number;inserted:number;skipped:number}|null>(null);
  let loading = $state(false);
  let error = $state("");

  // --- Langkah 1: pilih file ---
  async function pilihFile() {
    const path = await open({ filters: [{ name: "Excel", extensions: ["xlsx","xls"] }] });
    if (!path || typeof path !== "string") return;
    filePath = path;
    loading = true; error = "";
    try {
      const info = await api.impor.inspeksiFile(filePath);
      sheetList = Object.keys(info);
      sheetPilih = sheetList[0] ?? "";
      if (sheetPilih) {
        kolomFile = info[sheetPilih].columns;
        preview = info[sheetPilih].preview;
      }
      datasets = await invoke("cmd_daftar_dataset");
    } catch(e) { error = String(e); }
    loading = false;
  }

  async function lanjutKePetakan() {
    if (!filePath || !sheetPilih || !datasetId) { error = "Pilih file, sheet, dan dataset."; return; }
    variables = await invoke("cmd_daftar_variable", { datasetId });
    langkah = "petakan";
  }

  // --- Langkah 2: petakan kolom ---
  function lanjutKeValidasi() {
    if (!kolomWilayah) { error = "Pilih kolom nama wilayah."; return; }
    langkah = "validasi";
    jalankanValidasi();
  }

  async function jalankanValidasi() {
    loading = true; error = "";
    try {
      // Baca tabel dari sidecar
      const tabel = await api.impor.bacaTabel(filePath, sheetPilih);
      rowsPath = tabel.out_path;
      // Buat sesi
      sessionId = await api.impor.buatSesi(datasetId, filePath.split("/").pop()!, sheetPilih, periodValue);
      // Baca rows dari jsonl
      const rows: Record<string,unknown>[] = await invoke("cmd_baca_jsonl", { path: rowsPath });
      await api.impor.simpanStaging(sessionId, rows);
      // Pencocokan wilayah
      const namaWilayah = [...new Set(rows.map(r => String(r[kolomWilayah] ?? "")))];
      const unitDesa = await invoke("cmd_daftar_unit", { level: "desa" });
      const matches = await invoke("cmd_match_units", { names: namaWilayah, units: unitDesa });
      await api.impor.simpanUnitMatch(sessionId, matches);
      // Validasi
      const colMap: [string,number,string][] = Object.entries(pemetaan)
        .map(([col, varId]) => {
          const v = variables.find(x => x.id === varId);
          return [col, varId, v?.data_type ?? "teks"] as [string,number,string];
        });
      hasilValidasi = await api.impor.validasi(sessionId, datasetId, colMap, kolomWilayah, periodValue);
    } catch(e) { error = String(e); }
    loading = false;
  }

  async function komitImpor() {
    loading = true; error = "";
    try {
      const rows: Record<string,unknown>[] = await invoke("cmd_baca_jsonl", { path: rowsPath });
      const colMap: [string,number,number][] = Object.entries(pemetaan)
        .map(([col, varId]) => {
          const v = variables.find(x => x.id === varId);
          return [col, varId, 0] as [string,number,number];
        });
      hasilKomit = await api.impor.komit(sessionId, datasetId, periodValue, rows, colMap, kolomWilayah);
      langkah = "selesai";
    } catch(e) { error = String(e); }
    loading = false;
  }

  async function undoBatch() {
    if (!hasilKomit) return;
    await api.impor.undoBatch(hasilKomit.batch_id);
    langkah = "pilih_file";
    hasilKomit = null;
  }
</script>

<div class="p-6 max-w-3xl mx-auto">
  <h1 class="text-xl font-bold mb-4">Impor Data</h1>

  {#if error}
    <div class="mb-4 p-3 bg-red-50 border border-red-200 rounded text-sm text-red-700">{error}</div>
  {/if}

  <!-- Langkah 1: Pilih File -->
  {#if langkah === "pilih_file"}
    <div class="space-y-4">
      <button onclick={pilihFile}
        class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 text-sm">
        Pilih File Excel
      </button>
      {#if filePath}
        <p class="text-sm text-gray-600">File: <span class="font-mono">{filePath.split("/").pop()}</span></p>
        <div class="flex gap-3 items-end">
          <div>
            <label class="block text-xs text-gray-500 mb-1">Sheet</label>
            <select bind:value={sheetPilih} class="border rounded px-2 py-1 text-sm">
              {#each sheetList as s}<option>{s}</option>{/each}
            </select>
          </div>
          <div>
            <label class="block text-xs text-gray-500 mb-1">Dataset Tujuan</label>
            <select bind:value={datasetId} class="border rounded px-2 py-1 text-sm">
              <option value={0}>-- pilih --</option>
              {#each datasets as d}<option value={d.id}>{d.name}</option>{/each}
            </select>
          </div>
          <div>
            <label class="block text-xs text-gray-500 mb-1">Periode</label>
            <input bind:value={periodValue} placeholder="2024" class="border rounded px-2 py-1 text-sm w-24"/>
          </div>
        </div>
        {#if preview.length}
          <div class="overflow-x-auto border rounded">
            <table class="text-xs w-full">
              <thead class="bg-gray-50">
                <tr>{#each kolomFile as k}<th class="px-2 py-1 text-left font-medium">{k}</th>{/each}</tr>
              </thead>
              <tbody>
                {#each preview as row}
                  <tr class="border-t">
                    {#each kolomFile as k}
                      <td class="px-2 py-1">{row[k] ?? ""}</td>
                    {/each}
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
        <button onclick={lanjutKePetakan}
          class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 text-sm">
          Lanjut →
        </button>
      {/if}
      {#if loading}<p class="text-sm text-gray-400">Membaca file...</p>{/if}
    </div>

  <!-- Langkah 2: Petakan Kolom -->
  {:else if langkah === "petakan"}
    <div class="space-y-4">
      <h2 class="font-semibold text-sm">Petakan Kolom ke Variabel</h2>
      <div>
        <label class="block text-xs text-gray-500 mb-1">Kolom Nama Wilayah</label>
        <select bind:value={kolomWilayah} class="border rounded px-2 py-1 text-sm">
          <option value="">-- pilih --</option>
          {#each kolomFile as k}<option>{k}</option>{/each}
        </select>
      </div>
      {#each kolomFile.filter(k => k !== kolomWilayah && k !== "period_value") as kol}
        <div class="flex items-center gap-3">
          <span class="font-mono text-xs w-48 truncate">{kol}</span>
          <span class="text-gray-400">→</span>
          <select
            value={pemetaan[kol] ?? 0}
            onchange={(e) => { pemetaan = {...pemetaan, [kol]: Number((e.target as HTMLSelectElement).value)} }}
            class="border rounded px-2 py-1 text-sm">
            <option value={0}>-- abaikan --</option>
            {#each variables as v}<option value={v.id}>{v.name}</option>{/each}
          </select>
        </div>
      {/each}
      <div class="flex gap-2">
        <button onclick={() => langkah = "pilih_file"} class="px-4 py-2 border rounded text-sm">← Kembali</button>
        <button onclick={lanjutKeValidasi} class="px-4 py-2 bg-blue-600 text-white rounded text-sm">Validasi →</button>
      </div>
    </div>

  <!-- Langkah 3: Validasi -->
  {:else if langkah === "validasi"}
    <div class="space-y-4">
      {#if loading}
        <p class="text-sm text-gray-400">Memvalidasi data...</p>
      {:else}
        <div class="flex items-center gap-2">
          <span class="font-semibold text-sm">Hasil Validasi:</span>
          <span class="px-2 py-0.5 rounded text-xs font-medium
            {hasilValidasi.result === "lolos" ? "bg-green-100 text-green-700" :
             hasilValidasi.result === "peringatan" ? "bg-yellow-100 text-yellow-700" :
             "bg-red-100 text-red-700"}">
            {hasilValidasi.result.toUpperCase()}
          </span>
        </div>
        {#if hasilValidasi.issues.length}
          <div class="space-y-1 max-h-64 overflow-y-auto">
            {#each hasilValidasi.issues as isu}
              <div class="text-xs p-2 rounded
                {isu.severity === "fail" ? "bg-red-50 text-red-700" : "bg-yellow-50 text-yellow-700"}">
                {isu.message}
              </div>
            {/each}
          </div>
        {:else}
          <p class="text-sm text-green-600">Tidak ada masalah ditemukan.</p>
        {/if}
        <div class="flex gap-2">
          <button onclick={() => langkah = "petakan"} class="px-4 py-2 border rounded text-sm">← Kembali</button>
          {#if hasilValidasi.result !== "gagal"}
            <button onclick={() => langkah = "konfirmasi"} class="px-4 py-2 bg-blue-600 text-white rounded text-sm">Lanjut →</button>
          {/if}
        </div>
      {/if}
    </div>

  <!-- Langkah 4: Konfirmasi -->
  {:else if langkah === "konfirmasi"}
    <div class="space-y-4">
      <p class="text-sm">Data siap diimpor. Tindakan ini tidak dapat diubah kecuali dengan undo batch.</p>
      <div class="flex gap-2">
        <button onclick={() => langkah = "validasi"} class="px-4 py-2 border rounded text-sm">← Kembali</button>
        <button onclick={komitImpor} class="px-4 py-2 bg-green-600 text-white rounded text-sm">
          {loading ? "Mengimpor..." : "Impor Sekarang"}
        </button>
      </div>
    </div>

  <!-- Langkah 5: Selesai -->
  {:else if langkah === "selesai" && hasilKomit}
    <div class="space-y-4">
      <div class="p-4 bg-green-50 border border-green-200 rounded">
        <p class="font-semibold text-green-700">Impor Berhasil</p>
        <p class="text-sm mt-1">{hasilKomit.inserted} baris ditambahkan · {hasilKomit.skipped} baris dilewati</p>
        <p class="text-xs text-gray-500 mt-1">Batch ID: {hasilKomit.batch_id}</p>
      </div>
      <div class="flex gap-2">
        <a href="/katalog" class="px-4 py-2 bg-blue-600 text-white rounded text-sm">Ke Katalog</a>
        <button onclick={undoBatch} class="px-4 py-2 border border-red-300 text-red-600 rounded text-sm">
          Batalkan Impor Ini
        </button>
      </div>
    </div>
  {/if}
</div>
