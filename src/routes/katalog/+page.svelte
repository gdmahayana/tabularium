<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api, type Dataset, type BuatDatasetInput } from '#lib/api';

  let datasets = $state<Dataset[]>([]);
  let loading  = $state(true);
  let error    = $state('');
  let tampil_form = $state(false);

  // Form buat dataset
  let form = $state<BuatDatasetInput>({
    name: '', description: '', domain: '',
    grain_desc: '', period_type: 'tahunan', owner: '', source: '',
  });
  let form_loading = $state(false);
  let form_error   = $state('');

  const TIPE_PERIODE = [
    { value: 'tahunan',       label: 'Tahunan' },
    { value: 'semesteran',    label: 'Semesteran' },
    { value: 'triwulanan',    label: 'Triwulanan' },
    { value: 'bulanan',       label: 'Bulanan' },
    { value: 'tanggal',       label: 'Tanggal' },
    { value: 'tanpa_periode', label: 'Tanpa Periode' },
  ];

  async function muat() {
    loading = true; error = '';
    try { datasets = await api.dataset.daftar(); }
    catch (e) { error = String(e); }
    finally { loading = false; }
  }

  async function simpan_dataset() {
    if (!form.name.trim()) { form_error = 'Nama dataset wajib diisi.'; return; }
    form_loading = true; form_error = '';
    try {
      await api.dataset.buat({
        ...form,
        description: form.description || undefined,
        domain:      form.domain      || undefined,
        grain_desc:  form.grain_desc  || undefined,
        owner:       form.owner       || undefined,
        source:      form.source      || undefined,
      });
      tampil_form = false;
      form = { name: '', description: '', domain: '', grain_desc: '', period_type: 'tahunan', owner: '', source: '' };
      await muat();
    } catch (e) { form_error = String(e); }
    finally { form_loading = false; }
  }

  const STATUS_WARNA: Record<string, string> = {
    draft: 'bg-gray-100 text-gray-600',
    diperiksa: 'bg-yellow-100 text-yellow-700',
    terverifikasi: 'bg-blue-100 text-blue-700',
    dipublikasikan: 'bg-green-100 text-green-700',
  };

  onMount(muat);
</script>

<div class="p-6">
  <div class="flex items-center justify-between mb-6">
    <h1 class="text-xl font-semibold">Katalog Data</h1>
    <button
      onclick={() => tampil_form = !tampil_form}
      class="px-3 py-1.5 rounded-md text-sm text-white transition-colors"
      style="background:var(--primary)"
    >
      + Dataset Baru
    </button>
  </div>

  <!-- Form buat dataset -->
  {#if tampil_form}
    <div class="mb-6 rounded-lg border p-4" style="background:var(--surface);border-color:var(--border)">
      <h2 class="font-medium mb-4">Dataset Baru</h2>
      <div class="grid grid-cols-2 gap-3">
        <div class="col-span-2">
          <label class="block text-sm mb-1">Nama <span class="text-red-500">*</span></label>
          <input bind:value={form.name} class="w-full border rounded px-2 py-1.5 text-sm"
                 style="border-color:var(--border)" placeholder="mis. Data Pembinaan PKK" />
        </div>
        <div class="col-span-2">
          <label class="block text-sm mb-1">Deskripsi</label>
          <textarea bind:value={form.description} rows="2"
                    class="w-full border rounded px-2 py-1.5 text-sm resize-none"
                    style="border-color:var(--border)"></textarea>
        </div>
        <div>
          <label class="block text-sm mb-1">Domain</label>
          <input bind:value={form.domain} class="w-full border rounded px-2 py-1.5 text-sm"
                 style="border-color:var(--border)" placeholder="mis. pemberdayaan" />
        </div>
        <div>
          <label class="block text-sm mb-1">Tipe Periode</label>
          <select bind:value={form.period_type} class="w-full border rounded px-2 py-1.5 text-sm"
                  style="border-color:var(--border)">
            {#each TIPE_PERIODE as t}
              <option value={t.value}>{t.label}</option>
            {/each}
          </select>
        </div>
        <div class="col-span-2">
          <label class="block text-sm mb-1">Grain (apa yang diwakili satu baris)</label>
          <input bind:value={form.grain_desc} class="w-full border rounded px-2 py-1.5 text-sm"
                 style="border-color:var(--border)" placeholder="mis. Satu desa dalam satu tahun" />
        </div>
        <div>
          <label class="block text-sm mb-1">Penanggung Jawab</label>
          <input bind:value={form.owner} class="w-full border rounded px-2 py-1.5 text-sm"
                 style="border-color:var(--border)" placeholder="mis. DPMD" />
        </div>
        <div>
          <label class="block text-sm mb-1">Sumber</label>
          <input bind:value={form.source} class="w-full border rounded px-2 py-1.5 text-sm"
                 style="border-color:var(--border)" />
        </div>
      </div>

      {#if form_error}
        <p class="mt-3 text-sm text-red-600">{form_error}</p>
      {/if}

      <div class="mt-4 flex gap-2">
        <button onclick={simpan_dataset} disabled={form_loading}
                class="px-3 py-1.5 rounded text-sm text-white disabled:opacity-50"
                style="background:var(--primary)">
          {form_loading ? 'Menyimpan...' : 'Simpan'}
        </button>
        <button onclick={() => { tampil_form = false; form_error = ''; }}
                class="px-3 py-1.5 rounded text-sm border"
                style="border-color:var(--border)">
          Batal
        </button>
      </div>
    </div>
  {/if}

  <!-- Daftar dataset -->
  {#if loading}
    <p class="text-sm" style="color:var(--muted)">Memuat...</p>
  {:else if error}
    <p class="text-sm text-red-600">{error}</p>
  {:else if datasets.length === 0}
    <div class="rounded-lg border p-8 text-center" style="border-color:var(--border)">
      <p class="text-sm" style="color:var(--muted)">Belum ada dataset. Buat dataset pertama.</p>
    </div>
  {:else}
    <div class="rounded-lg border overflow-hidden" style="border-color:var(--border)">
      <table class="w-full text-sm">
        <thead style="background:#f3f4f6">
          <tr>
            <th class="text-left px-4 py-2 font-medium">Nama</th>
            <th class="text-left px-4 py-2 font-medium">Domain</th>
            <th class="text-left px-4 py-2 font-medium">Periode</th>
            <th class="text-left px-4 py-2 font-medium">Status</th>
          </tr>
        </thead>
        <tbody>
          {#each datasets as d}
            <tr class="border-t hover:bg-gray-50 cursor-pointer transition-colors"
                style="border-color:var(--border)"
                onclick={() => goto(`/katalog/${d.id}`)}>
              <td class="px-4 py-2.5 font-medium">{d.name}</td>
              <td class="px-4 py-2.5" style="color:var(--muted)">{d.domain ?? '—'}</td>
              <td class="px-4 py-2.5" style="color:var(--muted)">{d.period_type}</td>
              <td class="px-4 py-2.5">
                <span class="px-2 py-0.5 rounded-full text-xs {STATUS_WARNA[d.status] ?? ''}">
                  {d.status}
                </span>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>
