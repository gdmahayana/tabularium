<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Indicator, type IndicatorInput, type Variable,
           type Dataset, type HasilIndikator } from '#lib/api';

  let tab = $state<'indikator' | 'hasil'>('indikator');

  // ── State Indikator ─────────────────────────────────────────
  let indicators = $state<Indicator[]>([]);
  let datasets   = $state<Dataset[]>([]);
  let variables  = $state<Record<number, Variable[]>>({});
  let loading    = $state(true);
  let error      = $state('');
  let usang      = $state(0);

  let tampil_form  = $state(false);
  let edit_id      = $state<number | null>(null);
  let form_loading = $state(false);
  let form_error   = $state('');

  let form = $state<IndicatorInput & { dataset_id: number }>({
    dataset_id: 0, code: '', name: '', definition: '', interpretation: '',
    unit: '', formula: '', aggregation_rule: 'sum', variables: [],
  });

  // ── State Hasil ─────────────────────────────────────────────
  let pilih_ind    = $state<number | null>(null);
  let pilih_period = $state('2024');
  let hasil        = $state<HasilIndikator[]>([]);
  let hasil_loading = $state(false);
  let hitung_pesan = $state('');

  async function muat() {
    loading = true; error = '';
    try {
      [indicators, datasets] = await Promise.all([
        api.indicator.daftar(), api.dataset.daftar()
      ]);
      usang = await api.indicator.jumlahUsang();
      for (const d of datasets) variables[d.id] = await api.variable.daftar(d.id);
    } catch (e) { error = String(e); }
    finally { loading = false; }
  }

  function buka_form(ind?: Indicator) {
    form_error = '';
    if (ind) {
      edit_id = ind.id;
      const var0 = ind.variables[0];
      const ds_id = var0 ? Object.entries(variables).find(([,vs]) =>
        vs.some(v => v.id === var0.variable_id))?.[0] : '0';
      form = {
        dataset_id: Number(ds_id ?? 0), code: ind.code, name: ind.name,
        definition: ind.definition ?? '', interpretation: ind.interpretation ?? '',
        unit: ind.unit ?? '', formula: ind.formula,
        aggregation_rule: ind.aggregation_rule,
        variables: ind.variables.map(v => ({ ...v })),
      };
    } else {
      edit_id = null;
      form = { dataset_id: datasets[0]?.id ?? 0, code: '', name: '',
               definition: '', interpretation: '', unit: '', formula: '',
               aggregation_rule: 'sum', variables: [] };
    }
    tampil_form = true;
  }

  function tambah_var() {
    form.variables = [...form.variables, { variable_id: 0, alias: '' }];
  }

  function hapus_var(i: number) {
    form.variables = form.variables.filter((_, idx) => idx !== i);
  }

  async function simpan() {
    if (!form.code.trim() || !form.name.trim() || !form.formula.trim()) {
      form_error = 'Kode, nama, dan formula wajib diisi.'; return;
    }
    if (form.variables.some(v => !v.alias.trim() || !v.variable_id)) {
      form_error = 'Semua variabel harus memiliki alias dan variabel terpilih.'; return;
    }
    form_loading = true; form_error = '';
    const input: IndicatorInput = {
      code: form.code.trim(), name: form.name.trim(),
      definition: form.definition || undefined,
      interpretation: form.interpretation || undefined,
      unit: form.unit || undefined, formula: form.formula.trim(),
      aggregation_rule: form.aggregation_rule, variables: form.variables,
    };
    try {
      if (edit_id) await api.indicator.ubah(edit_id, input);
      else await api.indicator.buat(input);
      tampil_form = false; await muat();
    } catch (e) { form_error = String(e); }
    finally { form_loading = false; }
  }

  async function hitung(ind: Indicator) {
    hasil_loading = true; hitung_pesan = '';
    try {
      const n = await api.indicator.hitung(ind.id, pilih_period);
      hitung_pesan = `✓ ${ind.name} — ${n} wilayah dihitung untuk periode ${pilih_period}.`;
      usang = await api.indicator.jumlahUsang();
      // Langsung tampilkan hasil
      pilih_ind = ind.id; tab = 'hasil';
      hasil = await api.indicator.hasil(ind.id, pilih_period);
    } catch (e) { hitung_pesan = String(e); }
    finally { hasil_loading = false; }
  }

  async function lihat_hasil(ind: Indicator) {
    pilih_ind = ind.id; pilih_period = pilih_period;
    hasil_loading = true; tab = 'hasil';
    try { hasil = await api.indicator.hasil(ind.id, pilih_period); }
    catch (e) { error = String(e); }
    finally { hasil_loading = false; }
  }

  async function hitung_ulang() {
    hasil_loading = true;
    try {
      const n = await api.indicator.hitungUlang();
      hitung_pesan = `✓ ${n} pasangan dihitung ulang.`;
      usang = await api.indicator.jumlahUsang();
    } catch (e) { hitung_pesan = String(e); }
    finally { hasil_loading = false; }
  }

  const nama_ind = $derived(() =>
    indicators.find(i => i.id === pilih_ind)?.name ?? ''
  );

  onMount(muat);
</script>

<div class="p-6">
  <div class="flex items-start justify-between mb-5">
    <div>
      <h1 class="text-xl font-semibold">Intelijen Statistik</h1>
      <p class="text-sm mt-0.5" style="color:var(--muted)">
        Kelola indikator dan lihat hasil perhitungan per wilayah.
      </p>
    </div>
    {#if usang > 0}
      <div class="text-sm text-amber-600 flex items-center gap-2">
        ⚠ {usang} hasil usang
        <button onclick={hitung_ulang} class="underline text-xs">Hitung ulang</button>
      </div>
    {/if}
  </div>

  {#if hitung_pesan}
    <div class="mb-4 p-3 rounded-lg text-sm bg-blue-50 text-blue-700">{hitung_pesan}</div>
  {/if}

  <div class="flex gap-1 border-b mb-5" style="border-color:var(--border)">
    {#each [['indikator','📐 Daftar Indikator'], ['hasil','📊 Hasil Perhitungan']] as [val, label]}
      <button
        onclick={() => tab = val as typeof tab}
        class="px-4 py-2 text-sm border-b-2 transition-colors -mb-px
               {tab === val
                 ? 'border-blue-600 text-blue-700 font-medium'
                 : 'border-transparent text-gray-500 hover:text-gray-700'}">
        {label}
      </button>
    {/each}
  </div>

  <!-- Tab: Daftar Indikator -->
  {#if tab === 'indikator'}
    <div class="flex justify-between items-center mb-4">
      <div class="flex items-center gap-3">
        <span class="text-sm" style="color:var(--muted)">Periode hitung:</span>
        <input bind:value={pilih_period} type="text"
          class="border rounded px-2 py-1 text-sm w-20" style="border-color:var(--border)" />
      </div>
      <button onclick={() => buka_form()}
        class="px-3 py-1.5 text-sm rounded-md text-white" style="background:var(--primary)">
        + Indikator Baru
      </button>
    </div>

    {#if error}
      <p class="text-sm text-red-600 mb-3">{error}</p>
    {:else if loading}
      <p class="text-sm" style="color:var(--muted)">Memuat...</p>
    {:else if indicators.length === 0}
      <div class="rounded-lg border p-10 text-center" style="border-color:var(--border)">
        <p class="font-medium mb-1">Belum ada indikator</p>
        <p class="text-sm mb-4" style="color:var(--muted)">
          Buat indikator untuk menghitung nilai dari variabel dataset.
        </p>
        <button onclick={() => buka_form()}
          class="px-4 py-2 text-sm rounded-md text-white" style="background:var(--primary)">
          + Indikator Pertama
        </button>
      </div>
    {:else}
      <div class="border rounded-lg overflow-hidden" style="border-color:var(--border)">
        <table class="w-full text-sm">
          <thead style="background:var(--surface)">
            <tr>
              <th class="text-left px-4 py-2.5 font-medium">Nama</th>
              <th class="text-left px-4 py-2.5 font-medium">Formula</th>
              <th class="text-left px-4 py-2.5 font-medium">Satuan</th>
              <th class="text-left px-4 py-2.5 font-medium">Versi</th>
              <th class="px-4 py-2.5"></th>
            </tr>
          </thead>
          <tbody>
            {#each indicators as ind}
              <tr class="border-t hover:bg-gray-50" style="border-color:var(--border)">
                <td class="px-4 py-2.5">
                  <div class="font-medium">{ind.name}</div>
                  <div class="text-xs font-mono" style="color:var(--muted)">{ind.code}</div>
                </td>
                <td class="px-4 py-2.5 font-mono text-xs" style="color:var(--muted)">{ind.formula}</td>
                <td class="px-4 py-2.5 text-xs" style="color:var(--muted)">{ind.unit ?? '—'}</td>
                <td class="px-4 py-2.5 text-xs" style="color:var(--muted)">v{ind.method_version}</td>
                <td class="px-4 py-2.5">
                  <div class="flex gap-2 justify-end items-center">
                    <button onclick={() => lihat_hasil(ind)}
                      class="text-xs text-blue-600 hover:underline">Lihat hasil</button>
                    <button onclick={() => hitung(ind)} disabled={hasil_loading}
                      class="text-xs px-2 py-0.5 rounded border text-green-700"
                      style="border-color:#86efac">
                      {hasil_loading ? '...' : '▶ Hitung'}
                    </button>
                    <button onclick={() => buka_form(ind)}
                      class="text-xs text-gray-500 hover:underline">Ubah</button>
                  </div>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}

  <!-- Tab: Hasil -->
  {:else}
    <div class="flex gap-3 items-end mb-4">
      <label class="block">
        <span class="text-xs font-medium text-gray-600">Indikator</span>
        <select bind:value={pilih_ind}
          class="mt-1 block border rounded px-2 py-1.5 text-sm" style="border-color:var(--border)">
          <option value={null}>— pilih —</option>
          {#each indicators as ind}
            <option value={ind.id}>{ind.name}</option>
          {/each}
        </select>
      </label>
      <label class="block">
        <span class="text-xs font-medium text-gray-600">Periode</span>
        <input bind:value={pilih_period} type="text"
          class="mt-1 block border rounded px-2 py-1.5 text-sm w-20"
          style="border-color:var(--border)" />
      </label>
      <button
        onclick={async () => {
          if (!pilih_ind) return;
          hasil_loading = true;
          try { hasil = await api.indicator.hasil(pilih_ind, pilih_period); }
          catch (e) { error = String(e); }
          finally { hasil_loading = false; }
        }}
        class="px-3 py-1.5 text-sm rounded-md text-white mb-0.5"
        style="background:var(--primary)">
        Tampilkan
      </button>
    </div>

    {#if hasil_loading}
      <p class="text-sm" style="color:var(--muted)">Memuat hasil...</p>
    {:else if hasil.length === 0}
      <div class="rounded-lg border p-8 text-center" style="border-color:var(--border)">
        <p class="text-sm" style="color:var(--muted)">
          {pilih_ind ? 'Belum ada hasil. Klik ▶ Hitung di tab Daftar Indikator.' : 'Pilih indikator untuk melihat hasil.'}
        </p>
      </div>
    {:else}
      <div class="mb-2 text-sm font-medium">{nama_ind()} — Periode {pilih_period}</div>
      <div class="border rounded-lg overflow-hidden" style="border-color:var(--border)">
        <table class="w-full text-sm">
          <thead style="background:var(--surface)">
            <tr>
              <th class="text-left px-4 py-2.5 font-medium">Wilayah</th>
              <th class="text-left px-4 py-2.5 font-medium">Tingkat</th>
              <th class="text-right px-4 py-2.5 font-medium">Nilai</th>
              <th class="text-left px-4 py-2.5 font-medium">Status</th>
            </tr>
          </thead>
          <tbody>
            {#each hasil as h}
              <tr class="border-t {h.is_stale ? 'bg-amber-50' : ''}"
                style="border-color:var(--border)">
                <td class="px-4 py-2.5 font-medium">{h.region_name}</td>
                <td class="px-4 py-2.5 text-xs" style="color:var(--muted)">{h.region_level}</td>
                <td class="px-4 py-2.5 text-right font-mono">
                  {h.value_num !== null ? h.value_num.toFixed(2) : '—'}
                </td>
                <td class="px-4 py-2.5">
                  {#if h.is_stale}
                    <span class="text-xs text-amber-600">⚠ usang</span>
                  {:else if h.value_status === 'terisi'}
                    <span class="text-xs text-green-600">✓</span>
                  {:else}
                    <span class="text-xs" style="color:var(--muted)">{h.value_status}</span>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}
</div>

<!-- Modal Form Indikator -->
{#if tampil_form}
  <div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50">
    <div class="bg-white rounded-xl shadow-xl w-full max-w-2xl mx-4 p-6 overflow-y-auto max-h-[90vh]">
      <h2 class="text-base font-semibold mb-4">
        {edit_id ? 'Ubah Indikator' : 'Indikator Baru'}
      </h2>
      {#if form_error}
        <p class="text-sm text-red-600 mb-3">{form_error}</p>
      {/if}
      <div class="space-y-3">
        <div class="grid grid-cols-2 gap-3">
          <label class="block">
            <span class="text-xs font-medium text-gray-600">Kode *</span>
            <input bind:value={form.code} class="mt-1 w-full border rounded px-2 py-1.5 text-sm font-mono"
              style="border-color:var(--border)" placeholder="pkk_binaan_persen" />
          </label>
          <label class="block">
            <span class="text-xs font-medium text-gray-600">Satuan</span>
            <input bind:value={form.unit} class="mt-1 w-full border rounded px-2 py-1.5 text-sm"
              style="border-color:var(--border)" placeholder="persen" />
          </label>
        </div>
        <label class="block">
          <span class="text-xs font-medium text-gray-600">Nama *</span>
          <input bind:value={form.name} class="mt-1 w-full border rounded px-2 py-1.5 text-sm"
            style="border-color:var(--border)" />
        </label>
        <label class="block">
          <span class="text-xs font-medium text-gray-600">Formula *</span>
          <input bind:value={form.formula} class="mt-1 w-full border rounded px-2 py-1.5 text-sm font-mono"
            style="border-color:var(--border)" placeholder="ratio(binaan, kelompok) * 100" />
          <p class="text-xs mt-1" style="color:var(--muted)">Fungsi: sum, avg, count, ratio, if. Pakai alias variabel di bawah.</p>
        </label>
        <label class="block">
          <span class="text-xs font-medium text-gray-600">Definisi</span>
          <textarea bind:value={form.definition} rows="2"
            class="mt-1 w-full border rounded px-2 py-1.5 text-sm" style="border-color:var(--border)"></textarea>
        </label>
        <div>
          <div class="flex items-center justify-between mb-1">
            <span class="text-xs font-medium text-gray-600">Dataset sumber</span>
            <select bind:value={form.dataset_id}
              class="border rounded px-2 py-1 text-xs" style="border-color:var(--border)">
              {#each datasets as d}<option value={d.id}>{d.name}</option>{/each}
            </select>
          </div>
          <div class="text-xs font-medium text-gray-600 mb-1">Variabel → Alias dalam formula</div>
          {#each form.variables as v, i}
            <div class="flex gap-2 mb-1 items-center">
              <select bind:value={v.variable_id}
                class="flex-1 border rounded px-2 py-1 text-xs" style="border-color:var(--border)">
                <option value={0}>— pilih variabel —</option>
                {#each variables[form.dataset_id] ?? [] as va}
                  <option value={va.id}>{va.name} ({va.code})</option>
                {/each}
              </select>
              <span class="text-xs text-gray-400">→</span>
              <input bind:value={v.alias} placeholder="alias"
                class="w-28 border rounded px-2 py-1 text-xs font-mono" style="border-color:var(--border)" />
              <button onclick={() => hapus_var(i)} class="text-red-400 text-xs">✕</button>
            </div>
          {/each}
          <button onclick={tambah_var} class="text-xs text-blue-600 hover:underline mt-1">+ Tambah variabel</button>
        </div>
      </div>
      <div class="flex justify-end gap-2 mt-5">
        <button onclick={() => tampil_form = false}
          class="px-3 py-1.5 text-sm border rounded-md" style="border-color:var(--border)">Batal</button>
        <button onclick={simpan} disabled={form_loading}
          class="px-3 py-1.5 text-sm rounded-md text-white" style="background:var(--primary)">
          {form_loading ? 'Menyimpan...' : 'Simpan'}
        </button>
      </div>
    </div>
  </div>
{/if}