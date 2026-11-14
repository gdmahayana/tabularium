<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Mapping, type MappingInput, type Indicator, type Variable, type Dataset } from '#lib/api';

  const PROFILE_ID = 'karangasem-sdi-sektoral';

  let mappings    = $state<Mapping[]>([]);
  let indicators  = $state<Indicator[]>([]);
  let datasets    = $state<Dataset[]>([]);
  let variables   = $state<Record<number, Variable[]>>({});
  let loading     = $state(true);
  let error       = $state('');
  let tampil_form = $state(false);
  let edit_id     = $state<number | null>(null);
  let form_loading = $state(false);
  let form_error  = $state('');

  const SOURCE_TYPES = ['variable', 'indicator'];
  const AGGREGATIONS = ['sum_all_villages', 'ratio_of_sums', 'avg', 'last_value'];

  let form = $state<MappingInput>({
    profile_id: PROFILE_ID,
    bidang: '', sub_bidang: '', kategori: '',
    judul_kolom: '', satuan_unit: '',
    source_type: 'indicator', source_ref: '',
    aggregation: 'ratio_of_sums', unit_factor: 1,
  });

  async function muat() {
    loading = true; error = '';
    try {
      [mappings, indicators, datasets] = await Promise.all([
        api.mapping.daftar(PROFILE_ID),
        api.indicator.daftar(),
        api.dataset.daftar(),
      ]);
      for (const d of datasets) {
        variables[d.id] = await api.variable.daftar(d.id);
      }
    } catch (e) { error = String(e); }
    finally { loading = false; }
  }

  // Semua variabel dari semua dataset (untuk dropdown source_ref)
  let semua_variabel = $derived(() =>
    Object.values(variables).flat()
  );

  function buka_form(m?: Mapping) {
    form_error = '';
    if (m) {
      edit_id = m.id;
      form = {
        profile_id: m.profile_id,
        bidang: m.bidang, sub_bidang: m.sub_bidang,
        kategori: m.kategori, judul_kolom: m.judul_kolom,
        satuan_unit: m.satuan_unit, source_type: m.source_type,
        source_ref: m.source_ref, aggregation: m.aggregation,
        unit_factor: m.unit_factor,
      };
    } else {
      edit_id = null;
      form = {
        profile_id: PROFILE_ID, bidang: '', sub_bidang: '', kategori: '',
        judul_kolom: '', satuan_unit: '',
        source_type: 'indicator', source_ref: '',
        aggregation: 'ratio_of_sums', unit_factor: 1,
      };
    }
    tampil_form = true;
  }

  async function simpan() {
    if (!form.bidang.trim() || !form.judul_kolom.trim() || !form.source_ref.trim()) {
      form_error = 'Bidang, judul kolom, dan sumber wajib diisi.'; return;
    }
    form_loading = true; form_error = '';
    try {
      if (edit_id) await api.mapping.ubah(edit_id, form);
      else await api.mapping.buat(form);
      tampil_form = false;
      await muat();
    } catch (e) { form_error = String(e); }
    finally { form_loading = false; }
  }

  async function hapus(id: number) {
    if (!confirm('Hapus pemetaan ini?')) return;
    try { await api.mapping.hapus(id); await muat(); }
    catch (e) { error = String(e); }
  }

  // Isi otomatis dari profil JSON karangasem
  async function isi_dari_profil() {
    // Pemetaan awal berdasarkan karangasem-sdi-v0.json
    const templates: MappingInput[] = [
      {
        profile_id: PROFILE_ID,
        bidang: 'CONTOH BIDANG', sub_bidang: 'CONTOH SUB BIDANG',
        kategori: 'Pembinaan PKK',
        judul_kolom: 'Jumlah Kelompok PKK',
        satuan_unit: 'Kelompok',
        source_type: 'variable', source_ref: 'jumlah_kelompok',
        aggregation: 'sum_all_villages', unit_factor: 1,
      },
      {
        profile_id: PROFILE_ID,
        bidang: 'CONTOH BIDANG', sub_bidang: 'CONTOH SUB BIDANG',
        kategori: 'Pembinaan PKK',
        judul_kolom: 'Persentase Kelompok PKK Binaan',
        satuan_unit: 'Persen',
        source_type: 'indicator', source_ref: 'pkk_binaan_persen',
        aggregation: 'ratio_of_sums', unit_factor: 1,
      },
    ];
    form_loading = true;
    for (const t of templates) {
      try { await api.mapping.buat(t); } catch (_) { /* skip duplikat */ }
    }
    form_loading = false;
    await muat();
  }

  onMount(muat);
</script>

<div class="p-6 w-full">
  <div class="flex items-center justify-between mb-6">
    <div>
      <h1 class="text-xl font-semibold">Pemetaan Portal</h1>
      <p class="text-sm mt-0.5" style="color:var(--muted)">
        Profil: {PROFILE_ID}
      </p>
    </div>
    <div class="flex gap-2">
      {#if mappings.length === 0}
        <button onclick={isi_dari_profil} disabled={form_loading}
          class="px-3 py-1.5 text-sm border rounded-md" style="border-color:var(--border)">
          {form_loading ? 'Memuat...' : 'Isi dari profil PKK'}
        </button>
      {/if}
      <button onclick={() => buka_form()}
        class="px-3 py-1.5 text-sm rounded-md text-white"
        style="background:var(--primary)">
        + Pemetaan Baru
      </button>
    </div>
  </div>

  {#if error}
    <p class="text-sm text-red-600 mb-4">{error}</p>
  {:else if loading}
    <p class="text-sm" style="color:var(--muted)">Memuat...</p>
  {:else if mappings.length === 0}
    <div class="border rounded-lg p-8 text-center" style="border-color:var(--border)">
      <p class="text-sm mb-3" style="color:var(--muted)">Belum ada pemetaan.</p>
      <button onclick={isi_dari_profil}
        class="px-4 py-2 text-sm rounded-md text-white" style="background:var(--primary)">
        Isi dari profil PKK (contoh)
      </button>
    </div>
  {:else}
    <div class="border rounded-lg overflow-hidden" style="border-color:var(--border)">
      <table class="w-full text-sm">
        <thead>
          <tr style="background:var(--surface)">
            <th class="text-left px-4 py-2 font-medium">Bidang / Kategori</th>
            <th class="text-left px-4 py-2 font-medium">Judul Kolom</th>
            <th class="text-left px-4 py-2 font-medium">Sumber</th>
            <th class="text-left px-4 py-2 font-medium">Satuan</th>
            <th class="px-4 py-2"></th>
          </tr>
        </thead>
        <tbody>
          {#each mappings as m}
            <tr class="border-t" style="border-color:var(--border)">
              <td class="px-4 py-2">
                <div class="font-medium">{m.bidang}</div>
                <div class="text-xs" style="color:var(--muted)">{m.kategori}</div>
              </td>
              <td class="px-4 py-2">{m.judul_kolom}</td>
              <td class="px-4 py-2">
                <span class="text-xs px-1.5 py-0.5 rounded"
                  style="background:{m.source_type === 'indicator' ? '#eff6ff' : '#f0fdf4'};
                         color:{m.source_type === 'indicator' ? '#1d4ed8' : '#15803d'}">
                  {m.source_type}
                </span>
                <span class="ml-1 font-mono text-xs">{m.source_ref}</span>
              </td>
              <td class="px-4 py-2 text-xs" style="color:var(--muted)">{m.satuan_unit}</td>
              <td class="px-4 py-2">
                <div class="flex gap-2 justify-end">
                  <button onclick={() => buka_form(m)}
                    class="text-xs text-blue-600 hover:underline">Ubah</button>
                  <button onclick={() => hapus(m.id)}
                    class="text-xs text-red-500 hover:underline">Hapus</button>
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
    <div class="bg-white rounded-xl shadow-xl w-full max-w-lg mx-4 p-6 overflow-y-auto max-h-[90vh]">
      <h2 class="text-base font-semibold mb-4">
        {edit_id ? 'Ubah Pemetaan' : 'Pemetaan Baru'}
      </h2>

      {#if form_error}
        <p class="text-sm text-red-600 mb-3">{form_error}</p>
      {/if}

      <div class="space-y-3">
        <div class="grid grid-cols-2 gap-3">
          <label class="block">
            <span class="text-xs font-medium text-gray-600">Bidang *</span>
            <input bind:value={form.bidang} class="mt-1 w-full border rounded px-2 py-1.5 text-sm"
              style="border-color:var(--border)" placeholder="Pemberdayaan Masyarakat" />
          </label>
          <label class="block">
            <span class="text-xs font-medium text-gray-600">Sub Bidang</span>
            <input bind:value={form.sub_bidang} class="mt-1 w-full border rounded px-2 py-1.5 text-sm"
              style="border-color:var(--border)" placeholder="PKK" />
          </label>
        </div>

        <label class="block">
          <span class="text-xs font-medium text-gray-600">Kategori Laporan</span>
          <input bind:value={form.kategori} class="mt-1 w-full border rounded px-2 py-1.5 text-sm"
            style="border-color:var(--border)" placeholder="Pembinaan PKK" />
        </label>

        <div class="grid grid-cols-2 gap-3">
          <label class="block">
            <span class="text-xs font-medium text-gray-600">Judul Kolom *</span>
            <input bind:value={form.judul_kolom} class="mt-1 w-full border rounded px-2 py-1.5 text-sm"
              style="border-color:var(--border)" placeholder="Jumlah Kelompok PKK" />
          </label>
          <label class="block">
            <span class="text-xs font-medium text-gray-600">Satuan Unit</span>
            <input bind:value={form.satuan_unit} class="mt-1 w-full border rounded px-2 py-1.5 text-sm"
              style="border-color:var(--border)" placeholder="Kelompok" />
          </label>
        </div>

        <div class="grid grid-cols-2 gap-3">
          <label class="block">
            <span class="text-xs font-medium text-gray-600">Tipe sumber</span>
            <select bind:value={form.source_type}
              class="mt-1 w-full border rounded px-2 py-1.5 text-sm"
              style="border-color:var(--border)">
              {#each SOURCE_TYPES as t}
                <option value={t}>{t}</option>
              {/each}
            </select>
          </label>
          <label class="block">
            <span class="text-xs font-medium text-gray-600">Agregasi</span>
            <select bind:value={form.aggregation}
              class="mt-1 w-full border rounded px-2 py-1.5 text-sm"
              style="border-color:var(--border)">
              {#each AGGREGATIONS as a}
                <option value={a}>{a}</option>
              {/each}
            </select>
          </label>
        </div>

        <label class="block">
          <span class="text-xs font-medium text-gray-600">
            {form.source_type === 'indicator' ? 'Kode indikator *' : 'Kode variabel *'}
          </span>
          {#if form.source_type === 'indicator'}
            <select bind:value={form.source_ref}
              class="mt-1 w-full border rounded px-2 py-1.5 text-sm"
              style="border-color:var(--border)">
              <option value="">— pilih indikator —</option>
              {#each indicators as ind}
                <option value={ind.code}>{ind.name} ({ind.code})</option>
              {/each}
            </select>
          {:else}
            <select bind:value={form.source_ref}
              class="mt-1 w-full border rounded px-2 py-1.5 text-sm"
              style="border-color:var(--border)">
              <option value="">— pilih variabel —</option>
              {#each semua_variabel() as v}
                <option value={v.code}>{v.name} ({v.code})</option>
              {/each}
            </select>
          {/if}
        </label>

        <label class="block">
          <span class="text-xs font-medium text-gray-600">Faktor konversi satuan</span>
          <input bind:value={form.unit_factor} type="number" step="0.001"
            class="mt-1 w-full border rounded px-2 py-1.5 text-sm"
            style="border-color:var(--border)" />
          <p class="text-xs mt-0.5" style="color:var(--muted)">1 = tanpa konversi</p>
        </label>
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