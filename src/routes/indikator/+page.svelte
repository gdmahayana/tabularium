<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Indicator, type IndicatorInput, type Variable, type Dataset } from '#lib/api';

  let indicators = $state<Indicator[]>([]);
  let datasets   = $state<Dataset[]>([]);
  let variables  = $state<Record<number, Variable[]>>({});
  let loading    = $state(true);
  let error      = $state('');

  // Form
  let tampil_form   = $state(false);
  let edit_id       = $state<number | null>(null);
  let form_loading  = $state(false);
  let form_error    = $state('');

  let form = $state<IndicatorInput & { dataset_id: number }>({
    dataset_id: 0,
    code: '', name: '', definition: '', interpretation: '',
    unit: '', formula: '', aggregation_rule: 'sum', variables: [],
  });

  // Hitung
  let hitung_id     = $state<number | null>(null);
  let hitung_period = $state('2026');
  let hitung_loading = $state(false);
  let hitung_pesan  = $state('');
  let usang         = $state(0);

  async function muat() {
    loading = true; error = '';
    try {
      [indicators, datasets] = await Promise.all([api.indicator.daftar(), api.dataset.daftar()]);
      usang = await api.indicator.jumlahUsang();
      for (const d of datasets) {
        variables[d.id] = await api.variable.daftar(d.id);
      }
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
        dataset_id: Number(ds_id ?? 0),
        code: ind.code, name: ind.name,
        definition: ind.definition ?? '',
        interpretation: ind.interpretation ?? '',
        unit: ind.unit ?? '',
        formula: ind.formula,
        aggregation_rule: ind.aggregation_rule,
        variables: ind.variables.map(v => ({ ...v })),
      };
    } else {
      edit_id = null;
      form = { dataset_id: datasets[0]?.id ?? 0, code: '', name: '', definition: '',
               interpretation: '', unit: '', formula: '', aggregation_rule: 'sum', variables: [] };
    }
    tampil_form = true;
  }

  function tambah_variabel() {
    form.variables = [...form.variables, { variable_id: 0, alias: '' }];
  }

  function hapus_variabel(i: number) {
    form.variables = form.variables.filter((_, idx) => idx !== i);
  }

  async function simpan() {
    if (!form.code.trim()) { form_error = 'Kode wajib diisi.'; return; }
    if (!form.name.trim()) { form_error = 'Nama wajib diisi.'; return; }
    if (!form.formula.trim()) { form_error = 'Formula wajib diisi.'; return; }
    if (form.variables.some(v => !v.alias.trim() || !v.variable_id)) {
      form_error = 'Semua variabel harus memiliki alias dan variabel terpilih.'; return;
    }
    form_loading = true; form_error = '';
    const input: IndicatorInput = {
      code: form.code.trim(), name: form.name.trim(),
      definition: form.definition || undefined,
      interpretation: form.interpretation || undefined,
      unit: form.unit || undefined,
      formula: form.formula.trim(),
      aggregation_rule: form.aggregation_rule,
      variables: form.variables,
    };
    try {
      if (edit_id) await api.indicator.ubah(edit_id, input);
      else await api.indicator.buat(input);
      tampil_form = false;
      await muat();
    } catch (e) { form_error = String(e); }
    finally { form_loading = false; }
  }

  async function arsipkan(id: number) {
    if (!confirm('Arsipkan indikator ini?')) return;
    try { await api.indicator.arsipkan(id); await muat(); }
    catch (e) { error = String(e); }
  }

  async function hitung(ind: Indicator) {
    hitung_id = ind.id; hitung_loading = true; hitung_pesan = '';
    try {
      const n = await api.indicator.hitung(ind.id, hitung_period);
      hitung_pesan = `Selesai — ${n} wilayah dihitung.`;
      usang = await api.indicator.jumlahUsang();
    } catch (e) { hitung_pesan = String(e); }
    finally { hitung_loading = false; }
  }

  async function hitung_ulang_semua() {
    hitung_loading = true; hitung_pesan = '';
    try {
      const n = await api.indicator.hitungUlang();
      hitung_pesan = `Selesai — ${n} pasangan indikator-periode dihitung ulang.`;
      usang = await api.indicator.jumlahUsang();
    } catch (e) { hitung_pesan = String(e); }
    finally { hitung_loading = false; }
  }

  onMount(muat);
</script>

<div class="p-6 max-w-5xl">
  <div class="flex items-center justify-between mb-6">
    <div>
      <h1 class="text-xl font-semibold">Indikator</h1>
      {#if usang > 0}
        <p class="text-sm text-amber-600 mt-1">
          {usang} hasil usang —
          <button onclick={hitung_ulang_semua} class="underline">hitung ulang semua</button>
        </p>
      {/if}
    </div>
    <button
      onclick={() => buka_form()}
      class="px-3 py-1.5 text-sm rounded-md text-white"
      style="background:var(--primary)">
      + Indikator Baru
    </button>
  </div>

  {#if hitung_pesan}
    <p class="mb-4 text-sm p-3 rounded-md bg-blue-50 text-blue-700">{hitung_pesan}</p>
  {/if}

  {#if error}
    <p class="text-sm text-red-600 mb-4">{error}</p>
  {:else if loading}
    <p class="text-sm" style="color:var(--muted)">Memuat...</p>
  {:else if indicators.length === 0}
    <p class="text-sm" style="color:var(--muted)">Belum ada indikator.</p>
  {:else}
    <div class="border rounded-lg overflow-hidden" style="border-color:var(--border)">
      <table class="w-full text-sm">
        <thead>
          <tr style="background:var(--surface)">
            <th class="text-left px-4 py-2 font-medium">Kode</th>
            <th class="text-left px-4 py-2 font-medium">Nama</th>
            <th class="text-left px-4 py-2 font-medium">Formula</th>
            <th class="text-left px-4 py-2 font-medium">Satuan</th>
            <th class="text-left px-4 py-2 font-medium">Periode</th>
            <th class="px-4 py-2"></th>
          </tr>
        </thead>
        <tbody>
          {#each indicators as ind}
            <tr class="border-t" style="border-color:var(--border)">
              <td class="px-4 py-2 font-mono text-xs">{ind.code}</td>
              <td class="px-4 py-2">{ind.name}</td>
              <td class="px-4 py-2 font-mono text-xs text-gray-500">{ind.formula}</td>
              <td class="px-4 py-2 text-gray-500">{ind.unit ?? '—'}</td>
              <td class="px-4 py-2">
                <div class="flex items-center gap-1">
                  <input
                    type="text"
                    bind:value={hitung_period}
                    class="border rounded px-2 py-0.5 text-xs w-20"
                    style="border-color:var(--border)"
                  />
                  <button
                    onclick={() => hitung(ind)}
                    disabled={hitung_loading && hitung_id === ind.id}
                    class="text-xs px-2 py-0.5 rounded border"
                    style="border-color:var(--border)">
                    {hitung_loading && hitung_id === ind.id ? '...' : 'Hitung'}
                  </button>
                </div>
              </td>
              <td class="px-4 py-2">
                <div class="flex gap-2 justify-end">
                  <button onclick={() => buka_form(ind)}
                    class="text-xs text-blue-600 hover:underline">Ubah</button>
                  <button onclick={() => arsipkan(ind.id)}
                    class="text-xs text-red-500 hover:underline">Arsipkan</button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<!-- Form modal -->
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
            <input bind:value={form.code} class="mt-1 w-full border rounded px-2 py-1.5 text-sm"
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
            style="border-color:var(--border)" placeholder="Persentase Kelompok PKK Binaan" />
        </label>

        <label class="block">
          <span class="text-xs font-medium text-gray-600">Formula *</span>
          <input bind:value={form.formula} class="mt-1 w-full border rounded px-2 py-1.5 text-sm font-mono"
            style="border-color:var(--border)" placeholder="ratio(binaan, kelompok) * 100" />
          <p class="text-xs mt-1" style="color:var(--muted)">
            Fungsi: sum, avg, count, ratio, if. Gunakan alias variabel di bawah.
          </p>
        </label>

        <label class="block">
          <span class="text-xs font-medium text-gray-600">Definisi</span>
          <textarea bind:value={form.definition} rows="2"
            class="mt-1 w-full border rounded px-2 py-1.5 text-sm"
            style="border-color:var(--border)"></textarea>
        </label>

        <!-- Variabel sumber -->
        <div>
          <div class="flex items-center justify-between mb-2">
            <span class="text-xs font-medium text-gray-600">Dataset sumber</span>
            <select bind:value={form.dataset_id}
              class="border rounded px-2 py-1 text-xs" style="border-color:var(--border)">
              {#each datasets as d}
                <option value={d.id}>{d.name}</option>
              {/each}
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
                class="w-28 border rounded px-2 py-1 text-xs font-mono"
                style="border-color:var(--border)" />
              <button onclick={() => hapus_variabel(i)}
                class="text-red-400 hover:text-red-600 text-xs">✕</button>
            </div>
          {/each}
          <button onclick={tambah_variabel}
            class="text-xs text-blue-600 hover:underline mt-1">+ Tambah variabel</button>
        </div>
      </div>

      <div class="flex justify-end gap-2 mt-5">
        <button onclick={() => tampil_form = false}
          class="px-3 py-1.5 text-sm border rounded-md" style="border-color:var(--border)">
          Batal
        </button>
        <button onclick={simpan} disabled={form_loading}
          class="px-3 py-1.5 text-sm rounded-md text-white"
          style="background:var(--primary)">
          {form_loading ? 'Menyimpan...' : 'Simpan'}
        </button>
      </div>
    </div>
  </div>
{/if}