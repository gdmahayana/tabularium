<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type SelLembar, type RefUnit, type RingkasanLembar } from '#lib/api';

  const PROFILE_ID = 'karangasem-sdi-sektoral';

  let period    = $state('2026');
  let region_id = $state(0);
  let wilayah   = $state<RefUnit[]>([]);
  let lembar    = $state<SelLembar[]>([]);
  let ringkasan = $state<RingkasanLembar | null>(null);
  let loading   = $state(false);
  let error     = $state('');
  let pesan     = $state('');

  // Grup lembar per bidang
  let per_bidang = $derived(() => {
    const map: Record<string, Record<string, SelLembar[]>> = {};
    for (const sel of lembar) {
      if (!map[sel.bidang]) map[sel.bidang] = {};
      if (!map[sel.bidang][sel.kategori]) map[sel.bidang][sel.kategori] = [];
      map[sel.bidang][sel.kategori].push(sel);
    }
    return map;
  });

  async function muat_wilayah() {
    try {
      wilayah = await api.unit.daftar('kabupaten');
      if (wilayah.length > 0) region_id = wilayah[0].id;
    } catch (e) { error = String(e); }
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
        profile_id: sel.profile_id,
        period_value: sel.period_value,
        bidang: sel.bidang,
        sub_bidang: sel.sub_bidang,
        kategori: sel.kategori,
        judul_kolom: sel.judul_kolom,
        nilai_saat_input: sel.nilai ?? undefined,
        operator: 'pengguna',
      });
      await muat_lembar();
    } catch (e) { error = String(e); }
  }

  async function batal_tandai(sel: SelLembar) {
    try {
      await api.sdi.batalTandai(
        sel.profile_id, sel.period_value,
        sel.bidang, sel.sub_bidang, sel.kategori, sel.judul_kolom,
      );
      await muat_lembar();
    } catch (e) { error = String(e); }
  }

  function salin(teks: string) {
    navigator.clipboard.writeText(teks).then(() => {
      pesan = 'Disalin!';
      setTimeout(() => pesan = '', 1500);
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

  function warna_status(sel: SelLembar): string {
    if (sel.perlu_diperbarui) return 'bg-amber-50 border-amber-300';
    if (sel.sudah_diinput)    return 'bg-green-50 border-green-200';
    if (sel.value_status === 'kosong') return 'bg-gray-50 border-gray-200';
    return 'bg-white border-gray-200';
  }

  onMount(async () => { await muat_wilayah(); await muat_lembar(); });
</script>

<div class="p-6 w-full">
  <!-- Header -->
  <div class="flex items-start justify-between mb-4">
    <div>
      <h1 class="text-xl font-semibold">Lembar Input SDI</h1>
      <p class="text-sm mt-0.5" style="color:var(--muted)">
        Profil: {PROFILE_ID}
      </p>
    </div>
    <button onclick={ekspor_csv}
      class="px-3 py-1.5 text-sm border rounded-md" style="border-color:var(--border)">
      Ekspor CSV
    </button>
  </div>

  <!-- Filter -->
  <div class="flex gap-3 mb-4 items-end">
    <label class="block">
      <span class="text-xs font-medium text-gray-600">Tahun data</span>
      <input bind:value={period} type="text"
        class="mt-1 block border rounded px-2 py-1.5 text-sm w-24"
        style="border-color:var(--border)" />
    </label>
    <label class="block">
      <span class="text-xs font-medium text-gray-600">Wilayah</span>
      <select bind:value={region_id}
        class="mt-1 block border rounded px-2 py-1.5 text-sm"
        style="border-color:var(--border)">
        {#each wilayah as w}
          <option value={w.id}>{w.name}</option>
        {/each}
      </select>
    </label>
    <button onclick={muat_lembar}
      class="px-3 py-1.5 text-sm rounded-md text-white mb-0.5"
      style="background:var(--primary)">
      Tampilkan
    </button>
  </div>

  <!-- Ringkasan -->
  {#if ringkasan}
    <div class="grid grid-cols-4 gap-3 mb-5">
      {#each [
        { label: 'Total sel',         val: ringkasan.total_sel },
        { label: 'Sudah diinput',     val: ringkasan.sudah_diinput },
        { label: 'Perlu diperbarui',  val: ringkasan.perlu_diperbarui },
        { label: 'Belum diinput',     val: ringkasan.belum_diinput },
      ] as k}
        <div class="rounded-lg border p-3" style="background:var(--surface);border-color:var(--border)">
          <div class="text-lg font-bold" style="color:var(--primary)">{k.val}</div>
          <div class="text-xs mt-0.5" style="color:var(--muted)">{k.label}</div>
        </div>
      {/each}
    </div>
  {/if}

  {#if pesan}
    <p class="text-sm text-green-600 mb-3">{pesan}</p>
  {/if}
  {#if error}
    <p class="text-sm text-red-600 mb-3">{error}</p>
  {/if}

  {#if loading}
    <p class="text-sm" style="color:var(--muted)">Memuat lembar...</p>
  {:else if lembar.length === 0}
    <p class="text-sm" style="color:var(--muted)">
      Belum ada pemetaan untuk profil ini. Tambahkan pemetaan terlebih dahulu.
    </p>
  {:else}
    <!-- Per bidang -->
    {#each Object.entries(per_bidang()) as [bidang, kategoris]}
      <div class="mb-6">
        <h2 class="text-sm font-semibold uppercase tracking-wide mb-3"
          style="color:var(--muted)">{bidang}</h2>

        {#each Object.entries(kategoris) as [kategori, sels]}
          <div class="mb-4">
            <h3 class="text-sm font-medium mb-2">{kategori}</h3>
            <div class="border rounded-lg overflow-hidden" style="border-color:var(--border)">
              <table class="w-full text-sm">
                <thead>
                  <tr style="background:var(--surface)">
                    <th class="text-left px-4 py-2 font-medium">Judul Kolom</th>
                    <th class="text-left px-4 py-2 font-medium">Nilai</th>
                    <th class="text-left px-4 py-2 font-medium">Satuan</th>
                    <th class="text-left px-4 py-2 font-medium">Status</th>
                    <th class="px-4 py-2"></th>
                  </tr>
                </thead>
                <tbody>
                  {#each sels as sel}
                    <tr class="border-t {warna_status(sel)}"
                      style="border-color:var(--border)">
                      <td class="px-4 py-2">
                        {sel.judul_kolom}
                        {#if sel.is_stale}
                          <span class="ml-1 text-xs text-amber-500">(usang)</span>
                        {/if}
                        {#if sel.perlu_diperbarui}
                          <span class="ml-1 text-xs text-amber-600 font-medium">⚠ perlu diperbarui</span>
                        {/if}
                      </td>
                      <td class="px-4 py-2 font-mono">
                        {#if sel.nilai_tampil}
                          <button onclick={() => salin(sel.nilai_tampil ?? '')}
                            class="hover:bg-blue-50 px-1 rounded" title="Salin nilai">
                            {sel.nilai_tampil}
                          </button>
                        {:else}
                          <span style="color:var(--muted)">—</span>
                        {/if}
                      </td>
                      <td class="px-4 py-2 text-gray-500 text-xs">{sel.satuan_unit}</td>
                      <td class="px-4 py-2">
                        {#if sel.value_status === 'kosong'}
                          <span class="text-xs text-gray-400">kosong</span>
                        {:else if sel.value_status === 'tidak_berlaku'}
                          <span class="text-xs text-gray-400">tidak berlaku</span>
                        {:else if sel.sudah_diinput}
                          <span class="text-xs text-green-600 font-medium">✓ diinput</span>
                        {:else}
                          <span class="text-xs text-gray-400">belum</span>
                        {/if}
                      </td>
                      <td class="px-4 py-2">
                        <div class="flex gap-2 justify-end">
                          {#if sel.nilai_tampil}
                            <button onclick={() => salin(sel.nilai_tampil ?? '')}
                              class="text-xs text-blue-600 hover:underline">Salin</button>
                          {/if}
                          {#if sel.sudah_diinput}
                            <button onclick={() => batal_tandai(sel)}
                              class="text-xs text-gray-400 hover:underline">Batal</button>
                          {:else if sel.nilai_tampil}
                            <button onclick={() => tandai(sel)}
                              class="text-xs text-green-600 hover:underline">✓ Tandai</button>
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
</div>