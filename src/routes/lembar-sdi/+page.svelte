<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type SelLembar, type RefUnit, type RingkasanLembar,
           type Mapping, type MappingInput, type Indicator, type Variable, type Dataset } from '#lib/api';

  let tab = $state<'lembar' | 'pemetaan'>('lembar');
  const PROFILE_ID = 'karangasem-sdi-sektoral';

  // ── State Lembar ─────────────────────────────────────────────
  let period    = $state('2024');
  let region_id = $state(0);
  let wilayah   = $state<RefUnit[]>([]);
  let lembar    = $state<SelLembar[]>([]);
  let ringkasan = $state<RingkasanLembar | null>(null);
  let loading   = $state(false);
  let error     = $state('');
  let pesan     = $state('');

  let per_bidang = $derived(() => {
    const map: Record<string, Record<string, SelLembar[]>> = {};
    for (const sel of lembar) {
      if (!map[sel.bidang]) map[sel.bidang] = {};
      if (!map[sel.bidang][sel.kategori]) map[sel.bidang][sel.kategori] = [];
      map[sel.bidang][sel.kategori].push(sel);
    }
    return map;
  });

  // ── State Pemetaan ───────────────────────────────────────────
  let mappings   = $state<Mapping[]>([]);
  let indicators = $state<Indicator[]>([]);
  let datasets   = $state<Dataset[]>([]);
  let variables  = $state<Record<number, Variable[]>>({});
  let tampil_form = $state(false);
  let edit_id     = $state<number | null>(null);
  let form_loading = $state(false);
  let form_error  = $state('');

  let form = $state<MappingInput>({
    profile_id: PROFILE_ID, bidang: '', sub_bidang: '', kategori: '',
    judul_kolom: '', satuan_unit: '', source_type: 'indicator',
    source_ref: '', aggregation: 'ratio_of_sums', unit_factor: 1,
  });

  let semua_variabel = $derived(() => Object.values(variables).flat());

  async function muat_semua() {
    loading = true; error = '';
    try {
      wilayah = await api.unit.daftar('kabupaten');
      if (wilayah.length > 0 && region_id === 0) region_id = wilayah[0].id;
      [mappings, indicators, datasets] = await Promise.all([
        api.mapping.daftar(PROFILE_ID),
        api.indicator.daftar(),
        api.dataset.daftar(),
      ]);
      for (const d of datasets) variables[d.id] = await api.variable.daftar(d.id);
    } catch (e) { error = String(e); }
    finally { loading = false; }
  }

  async function muat_lembar() {
    if (!region_id) return;
    loading = true; error = ''; pesan = '';
    try {
      [lembar, ringkasan] = await Promise.all([
        api.sdi.susunLembar(PROFILE_ID, period, region_id),
        api.sdi.ringkasan(PROFILE_ID, period, region_id),
      ]);
    } catch (e) { error = String(e); }
    finally { loading = false; }
  }

  async function tandai(sel: SelLembar) {
    try {
      await api.sdi.tandaiSel({
        profile_id: sel.profile_id, period_value: sel.period_value,
        bidang: sel.bidang, sub_bidang: sel.sub_bidang,
        kategori: sel.kategori, judul_kolom: sel.judul_kolom,
        nilai_saat_input: sel.nilai ?? undefined, operator: 'pengguna',
      });
      await muat_lembar();
    } catch (e) { error = String(e); }
  }

  async function batal_tandai(sel: SelLembar) {
    try {
      await api.sdi.batalTandai(sel.profile_id, sel.period_value,
        sel.bidang, sel.sub_bidang, sel.kategori, sel.judul_kolom);
      await muat_lembar();
    } catch (e) { error = String(e); }
  }

  function salin(teks: string) {
    navigator.clipboard.writeText(teks).then(() => {
      pesan = '✓ Disalin!'; setTimeout(() => pesan = '', 1500);
    });
  }

  async function ekspor_csv() {
    try {
      const csv = await api.sdi.eksporCsv(PROFILE_ID, period, region_id);
      const blob = new Blob([csv], { type: 'text/csv' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url; a.download = `sdi-${period}.csv`; a.click();
      URL.revokeObjectURL(url);
    } catch (e) { error = String(e); }
  }

  function warna(sel: SelLembar) {
    if (sel.perlu_diperbarui) return 'bg-amber-50';
    if (sel.sudah_diinput)    return 'bg-green-50';
    return '';
  }

  // Pemetaan
  function buka_form_pemetaan(m?: Mapping) {
    form_error = '';
    if (m) {
      edit_id = m.id;
      form = { profile_id: m.profile_id, bidang: m.bidang, sub_bidang: m.sub_bidang,
               kategori: m.kategori, judul_kolom: m.judul_kolom, satuan_unit: m.satuan_unit,
               source_type: m.source_type, source_ref: m.source_ref,
               aggregation: m.aggregation, unit_factor: m.unit_factor };
    } else {
      edit_id = null;
      form = { profile_id: PROFILE_ID, bidang: '', sub_bidang: '', kategori: '',
               judul_kolom: '', satuan_unit: '', source_type: 'indicator',
               source_ref: '', aggregation: 'ratio_of_sums', unit_factor: 1 };
    }
    tampil_form = true;
  }

  async function simpan_pemetaan() {
    if (!form.bidang.trim() || !form.judul_kolom.trim() || !form.source_ref.trim()) {
      form_error = 'Bidang, judul kolom, dan sumber wajib diisi.'; return;
    }
    form_loading = true; form_error = '';
    try {
      if (edit_id) await api.mapping.ubah(edit_id, form);
      else await api.mapping.buat(form);
      tampil_form = false;
      mappings = await api.mapping.daftar(PROFILE_ID);
    } catch (e) { form_error = String(e); }
    finally { form_loading = false; }
  }

  async function hapus_pemetaan(id: number) {
    if (!confirm('Hapus pemetaan ini?')) return;
    try { await api.mapping.hapus(id); mappings = await api.mapping.daftar(PROFILE_ID); }
    catch (e) { error = String(e); }
  }

  async function isi_dari_profil() {
    form_loading = true;
    const templates: MappingInput[] = [
      { profile_id: PROFILE_ID, bidang: 'Pemberdayaan Masyarakat', sub_bidang: 'PKK',
        kategori: 'Pembinaan PKK', judul_kolom: 'Jumlah Kelompok PKK',
        satuan_unit: 'Kelompok', source_type: 'variable',
        source_ref: 'jumlah_kelompok_pkk', aggregation: 'sum_all_villages', unit_factor: 1 },
      { profile_id: PROFILE_ID, bidang: 'Pemberdayaan Masyarakat', sub_bidang: 'PKK',
        kategori: 'Pembinaan PKK', judul_kolom: 'Persentase Kelompok PKK Binaan',
        satuan_unit: 'Persen', source_type: 'indicator',
        source_ref: 'pkk_binaan_persen', aggregation: 'ratio_of_sums', unit_factor: 1 },
    ];
    for (const t of templates) {
      try { await api.mapping.buat(t); } catch (_) {}
    }
    form_loading = false;
    mappings = await api.mapping.daftar(PROFILE_ID);
    pesan = '✓ Pemetaan PKK berhasil dibuat. Buka tab Lembar Input untuk melihat hasilnya.';
    setTimeout(() => pesan = '', 4000);
  }

  onMount(async () => { await muat_semua(); await muat_lembar(); });
</script>

<div class="p-6">
  <div class="flex items-start justify-between mb-5">
    <div>
      <h1 class="text-xl font-semibold">Lembar Input SDI</h1>
      <p class="text-sm mt-0.5" style="color:var(--muted)">Profil: {PROFILE_ID}</p>
    </div>
    {#if tab === 'lembar'}
      <button onclick={ekspor_csv}
        class="px-3 py-1.5 text-sm border rounded-md" style="border-color:var(--border)">
        Ekspor CSV
      </button>
    {/if}
  </div>

  {#if pesan}
    <div class="mb-4 p-3 rounded-lg text-sm bg-blue-50 text-blue-700">{pesan}</div>
  {/if}
  {#if error}
    <p class="text-sm text-red-600 mb-3">{error}</p>
  {/if}

  <div class="flex gap-1 border-b mb-5" style="border-color:var(--border)">
    {#each [['lembar','📋 Lembar Input'], ['pemetaan','🔗 Pemetaan Portal']] as [val, label]}
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

  <!-- Tab: Lembar Input -->
  {#if tab === 'lembar'}
    <div class="flex gap-3 mb-5 items-end flex-wrap">
      <label class="block">
        <span class="text-xs font-medium text-gray-600">Tahun data</span>
        <input bind:value={period} type="text"
          class="mt-1 block border rounded px-2 py-1.5 text-sm w-24"
          style="border-color:var(--border)" />
      </label>
      <label class="block">
        <span class="text-xs font-medium text-gray-600">Wilayah</span>
        <select bind:value={region_id}
          class="mt-1 block border rounded px-2 py-1.5 text-sm" style="border-color:var(--border)">
          {#each wilayah as w}<option value={w.id}>{w.name}</option>{/each}
        </select>
      </label>
      <button onclick={muat_lembar}
        class="px-3 py-1.5 text-sm rounded-md text-white mb-0.5" style="background:var(--primary)">
        Tampilkan
      </button>
    </div>

    {#if ringkasan}
      <div class="grid grid-cols-4 gap-3 mb-5">
        {#each [
          { label: 'Total sel',        val: ringkasan.total_sel,        warna: '' },
          { label: 'Sudah diinput',    val: ringkasan.sudah_diinput,    warna: 'text-green-600' },
          { label: 'Perlu diperbarui', val: ringkasan.perlu_diperbarui, warna: 'text-amber-600' },
          { label: 'Belum diinput',    val: ringkasan.belum_diinput,    warna: '' },
        ] as k}
          <div class="rounded-lg border p-3" style="background:var(--surface);border-color:var(--border)">
            <div class="text-lg font-bold {k.warna}" style="{!k.warna ? 'color:var(--primary)' : ''}">{k.val}</div>
            <div class="text-xs mt-0.5" style="color:var(--muted)">{k.label}</div>
          </div>
        {/each}
      </div>
    {/if}

    {#if loading}
      <p class="text-sm" style="color:var(--muted)">Memuat lembar...</p>
    {:else if lembar.length === 0}
      <div class="rounded-lg border p-10 text-center" style="border-color:var(--border)">
        <p class="font-medium mb-1">Lembar kosong</p>
        <p class="text-sm mb-4" style="color:var(--muted)">
          Buat pemetaan portal terlebih dahulu agar nilai muncul di sini.
        </p>
        <button onclick={() => tab = 'pemetaan'}
          class="px-4 py-2 text-sm rounded-md text-white" style="background:var(--primary)">
          → Buat Pemetaan
        </button>
      </div>
    {:else}
      {#each Object.entries(per_bidang()) as [bidang, kategoris]}
        <div class="mb-6">
          <h2 class="text-xs font-semibold uppercase tracking-wide mb-3" style="color:var(--muted)">{bidang}</h2>
          {#each Object.entries(kategoris) as [kategori, sels]}
            <div class="mb-4">
              <h3 class="text-sm font-medium mb-2">{kategori}</h3>
              <div class="border rounded-lg overflow-hidden" style="border-color:var(--border)">
                <table class="w-full text-sm">
                  <thead style="background:var(--surface)">
                    <tr>
                      <th class="text-left px-4 py-2.5 font-medium">Judul Kolom</th>
                      <th class="text-right px-4 py-2.5 font-medium">Nilai</th>
                      <th class="text-left px-4 py-2.5 font-medium">Satuan</th>
                      <th class="text-left px-4 py-2.5 font-medium">Status</th>
                      <th class="px-4 py-2.5"></th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each sels as sel}
                      <tr class="border-t {warna(sel)}" style="border-color:var(--border)">
                        <td class="px-4 py-2.5">
                          {sel.judul_kolom}
                          {#if sel.is_stale}<span class="ml-1 text-xs text-amber-500">(usang)</span>{/if}
                          {#if sel.perlu_diperbarui}<span class="ml-1 text-xs text-amber-600 font-medium">⚠ perlu diperbarui di portal</span>{/if}
                        </td>
                        <td class="px-4 py-2.5 text-right font-mono">
                          {#if sel.nilai_tampil}
                            <button onclick={() => salin(sel.nilai_tampil ?? '')}
                              class="hover:bg-blue-50 px-1.5 py-0.5 rounded font-medium" title="Klik untuk menyalin">
                              {sel.nilai_tampil}
                            </button>
                          {:else}
                            <span style="color:var(--muted)">—</span>
                          {/if}
                        </td>
                        <td class="px-4 py-2.5 text-xs" style="color:var(--muted)">{sel.satuan_unit}</td>
                        <td class="px-4 py-2.5">
                          {#if sel.value_status === 'kosong'}
                            <span class="text-xs" style="color:var(--muted)">kosong</span>
                          {:else if sel.sudah_diinput}
                            <span class="text-xs text-green-600 font-medium">✓ diinput</span>
                          {:else}
                            <span class="text-xs" style="color:var(--muted)">belum</span>
                          {/if}
                        </td>
                        <td class="px-4 py-2.5">
                          <div class="flex gap-2 justify-end">
                            {#if sel.nilai_tampil}
                              <button onclick={() => salin(sel.nilai_tampil ?? '')}
                                class="text-xs text-blue-600 hover:underline">Salin</button>
                            {/if}
                            {#if sel.sudah_diinput}
                              <button onclick={() => batal_tandai(sel)}
                                class="text-xs hover:underline" style="color:var(--muted)">Batal</button>
                            {:else if sel.nilai_tampil}
                              <button onclick={() => tandai(sel)}
                                class="text-xs text-green-600 hover:underline font-medium">✓ Tandai</button>
                            {/if}
                          </div>
                        </td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            </div>
          {/each}
        </div>
      {/each}
    {/if}

  <!-- Tab: Pemetaan Portal -->
  {:else}
    <div class="flex items-center justify-between mb-4">
      <p class="text-sm" style="color:var(--muted)">
        Hubungkan variabel atau indikator ke kolom portal SDI.
      </p>
      <div class="flex gap-2">
        {#if mappings.length === 0}
          <button onclick={isi_dari_profil} disabled={form_loading}
            class="px-3 py-1.5 text-sm border rounded-md" style="border-color:var(--border)">
            {form_loading ? 'Memuat...' : 'Isi contoh PKK'}
          </button>
        {/if}
        <button onclick={() => buka_form_pemetaan()}
          class="px-3 py-1.5 text-sm rounded-md text-white" style="background:var(--primary)">
          + Pemetaan Baru
        </button>
      </div>
    </div>

    {#if mappings.length === 0}
      <div class="rounded-lg border p-10 text-center" style="border-color:var(--border)">
        <p class="font-medium mb-1">Belum ada pemetaan</p>
        <p class="text-sm mb-4" style="color:var(--muted)">
          Pemetaan menghubungkan indikator atau variabel ke kolom pada portal Satu Data.
        </p>
        <button onclick={isi_dari_profil}
          class="px-4 py-2 text-sm rounded-md text-white" style="background:var(--primary)">
          Isi contoh pemetaan PKK
        </button>
      </div>
    {:else}
      <div class="border rounded-lg overflow-hidden" style="border-color:var(--border)">
        <table class="w-full text-sm">
          <thead style="background:var(--surface)">
            <tr>
              <th class="text-left px-4 py-2.5 font-medium">Bidang / Kategori</th>
              <th class="text-left px-4 py-2.5 font-medium">Judul Kolom Portal</th>
              <th class="text-left px-4 py-2.5 font-medium">Sumber</th>
              <th class="text-left px-4 py-2.5 font-medium">Satuan</th>
              <th class="px-4 py-2.5"></th>
            </tr>
          </thead>
          <tbody>
            {#each mappings as m}
              <tr class="border-t" style="border-color:var(--border)">
                <td class="px-4 py-2.5">
                  <div class="font-medium">{m.bidang}</div>
                  <div class="text-xs" style="color:var(--muted)">{m.kategori}</div>
                </td>
                <td class="px-4 py-2.5">{m.judul_kolom}</td>
                <td class="px-4 py-2.5">
                  <span class="text-xs px-1.5 py-0.5 rounded mr-1"
                    style="background:{m.source_type === 'indicator' ? '#eff6ff' : '#f0fdf4'};
                           color:{m.source_type === 'indicator' ? '#1d4ed8' : '#15803d'}">
                    {m.source_type}
                  </span>
                  <span class="font-mono text-xs">{m.source_ref}</span>
                </td>
                <td class="px-4 py-2.5 text-xs" style="color:var(--muted)">{m.satuan_unit}</td>
                <td class="px-4 py-2.5">
                  <div class="flex gap-2 justify-end">
                    <button onclick={() => buka_form_pemetaan(m)}
                      class="text-xs text-blue-600 hover:underline">Ubah</button>
                    <button onclick={() => hapus_pemetaan(m.id)}
                      class="text-xs text-red-500 hover:underline">Hapus</button>
                  </div>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}
</div>

<!-- Modal Form Pemetaan -->
{#if tampil_form}
  <div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50">
    <div class="bg-white rounded-xl shadow-xl w-full max-w-lg mx-4 p-6 overflow-y-auto max-h-[90vh]">
      <h2 class="text-base font-semibold mb-4">{edit_id ? 'Ubah Pemetaan' : 'Pemetaan Baru'}</h2>
      {#if form_error}<p class="text-sm text-red-600 mb-3">{form_error}</p>{/if}
      <div class="space-y-3">
        <div class="grid grid-cols-2 gap-3">
          <label class="block">
            <span class="text-xs font-medium text-gray-600">Bidang *</span>
            <input bind:value={form.bidang} class="mt-1 w-full border rounded px-2 py-1.5 text-sm"
              style="border-color:var(--border)" />
          </label>
          <label class="block">
            <span class="text-xs font-medium text-gray-600">Sub Bidang</span>
            <input bind:value={form.sub_bidang} class="mt-1 w-full border rounded px-2 py-1.5 text-sm"
              style="border-color:var(--border)" />
          </label>
        </div>
        <label class="block">
          <span class="text-xs font-medium text-gray-600">Kategori Laporan</span>
          <input bind:value={form.kategori} class="mt-1 w-full border rounded px-2 py-1.5 text-sm"
            style="border-color:var(--border)" />
        </label>
        <div class="grid grid-cols-2 gap-3">
          <label class="block">
            <span class="text-xs font-medium text-gray-600">Judul Kolom *</span>
            <input bind:value={form.judul_kolom} class="mt-1 w-full border rounded px-2 py-1.5 text-sm"
              style="border-color:var(--border)" />
          </label>
          <label class="block">
            <span class="text-xs font-medium text-gray-600">Satuan Unit</span>
            <input bind:value={form.satuan_unit} class="mt-1 w-full border rounded px-2 py-1.5 text-sm"
              style="border-color:var(--border)" />
          </label>
        </div>
        <div class="grid grid-cols-2 gap-3">
          <label class="block">
            <span class="text-xs font-medium text-gray-600">Tipe sumber</span>
            <select bind:value={form.source_type}
              class="mt-1 w-full border rounded px-2 py-1.5 text-sm" style="border-color:var(--border)">
              <option value="indicator">indicator</option>
              <option value="variable">variable</option>
            </select>
          </label>
          <label class="block">
            <span class="text-xs font-medium text-gray-600">Ref sumber *</span>
            {#if form.source_type === 'indicator'}
              <select bind:value={form.source_ref}
                class="mt-1 w-full border rounded px-2 py-1.5 text-sm" style="border-color:var(--border)">
                <option value="">— pilih —</option>
                {#each indicators as i}<option value={i.code}>{i.name}</option>{/each}
              </select>
            {:else}
              <select bind:value={form.source_ref}
                class="mt-1 w-full border rounded px-2 py-1.5 text-sm" style="border-color:var(--border)">
                <option value="">— pilih —</option>
                {#each semua_variabel() as v}<option value={v.code}>{v.name}</option>{/each}
              </select>
            {/if}
          </label>
        </div>
      </div>
      <div class="flex justify-end gap-2 mt-5">
        <button onclick={() => tampil_form = false}
          class="px-3 py-1.5 text-sm border rounded-md" style="border-color:var(--border)">Batal</button>
        <button onclick={simpan_pemetaan} disabled={form_loading}
          class="px-3 py-1.5 text-sm rounded-md text-white" style="background:var(--primary)">
          {form_loading ? 'Menyimpan...' : 'Simpan'}
        </button>
      </div>
    </div>
  </div>
{/if}