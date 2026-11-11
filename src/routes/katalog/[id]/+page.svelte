<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { api, type Dataset, type Variable, type BuatVariableInput, type EntriAuditLog } from '#lib/api';

  const id = Number(page.params.id);
  let dataset   = $state<Dataset | null>(null);
  let variabel  = $state<Variable[]>([]);
  let audit     = $state<EntriAuditLog[]>([]);
  let tab       = $state<'variabel' | 'riwayat'>('variabel');
  let loading   = $state(true);
  let error     = $state('');

  // Form variabel
  let tampil_form_var = $state(false);
  let form_var = $state<BuatVariableInput>({
    dataset_id: id, name: '', code: '', data_type: 'integer',
    unit: '', concept: '', definition: '', is_required: false,
  });
  let form_var_loading = $state(false);
  let form_var_error   = $state('');

  const TIPE_DATA = [
    { value: 'integer',  label: 'Bilangan bulat' },
    { value: 'decimal',  label: 'Bilangan desimal' },
    { value: 'text',     label: 'Teks' },
    { value: 'date',     label: 'Tanggal' },
    { value: 'boolean',  label: 'Ya/Tidak' },
    { value: 'category', label: 'Kategori' },
  ];

  async function muat() {
    loading = true; error = '';
    try {
      [dataset, variabel, audit] = await Promise.all([
        api.dataset.ambil(id),
        api.variable.daftar(id),
        api.audit.log('dataset', id, 50),
      ]);
    } catch (e) { error = String(e); }
    finally { loading = false; }
  }

  async function simpan_variabel() {
    if (!form_var.name.trim()) { form_var_error = 'Nama wajib diisi.'; return; }
    if (!form_var.code.trim()) { form_var_error = 'Kode wajib diisi.'; return; }
    form_var_loading = true; form_var_error = '';
    try {
      await api.variable.buat({
        ...form_var,
        unit:       form_var.unit       || undefined,
        concept:    form_var.concept    || undefined,
        definition: form_var.definition || undefined,
      });
      tampil_form_var = false;
      form_var = { dataset_id: id, name: '', code: '', data_type: 'integer',
                   unit: '', concept: '', definition: '', is_required: false };
      variabel = await api.variable.daftar(id);
      audit    = await api.audit.log('dataset', id, 50);
    } catch (e) { form_var_error = String(e); }
    finally { form_var_loading = false; }
  }

  async function arsipkan(var_id: number, nama: string) {
    if (!confirm(`Arsipkan variabel "${nama}"? Variabel tidak akan dihapus tetapi disembunyikan.`)) return;
    try {
      await api.variable.arsipkan(var_id);
      variabel = await api.variable.daftar(id);
    } catch (e) { alert(String(e)); }
  }

  function format_tanggal(iso: string) {
    return new Date(iso).toLocaleString('id-ID', { dateStyle: 'medium', timeStyle: 'short' });
  }

  onMount(muat);
</script>

{#if loading}
  <div class="p-6 text-sm" style="color:var(--muted)">Memuat...</div>
{:else if error}
  <div class="p-6 text-sm text-red-600">{error}</div>
{:else if dataset}
  <div class="p-6">
    <!-- Header -->
    <div class="mb-6">
      <button onclick={() => goto('/katalog')}
              class="text-sm mb-2 hover:underline" style="color:var(--muted)">
        ← Katalog Data
      </button>
      <div class="flex items-start justify-between">
        <div>
          <h1 class="text-xl font-semibold">{dataset.name}</h1>
          {#if dataset.description}
            <p class="text-sm mt-1" style="color:var(--muted)">{dataset.description}</p>
          {/if}
        </div>
      </div>
      <!-- Metadata ringkas -->
      <div class="mt-3 flex flex-wrap gap-4 text-sm" style="color:var(--muted)">
        {#if dataset.domain}<span>Domain: <strong>{dataset.domain}</strong></span>{/if}
        <span>Periode: <strong>{dataset.period_type}</strong></span>
        {#if dataset.grain_desc}<span>Grain: <strong>{dataset.grain_desc}</strong></span>{/if}
        {#if dataset.owner}<span>PJ: <strong>{dataset.owner}</strong></span>{/if}
        <span>Status: <strong>{dataset.status}</strong></span>
      </div>
    </div>

    <!-- Tab -->
    <div class="flex gap-1 border-b mb-4" style="border-color:var(--border)">
      {#each [['variabel','Variabel'], ['riwayat','Riwayat Perubahan']] as [val, label]}
        <button
          onclick={() => tab = val as typeof tab}
          class="px-4 py-2 text-sm border-b-2 transition-colors -mb-px
                 {tab === val ? 'border-blue-600 text-blue-700 font-medium' : 'border-transparent text-gray-500 hover:text-gray-700'}"
        >{label}</button>
      {/each}
    </div>

    <!-- Tab: Variabel -->
    {#if tab === 'variabel'}
      <div class="flex justify-end mb-3">
        <button onclick={() => tampil_form_var = !tampil_form_var}
                class="px-3 py-1.5 rounded text-sm text-white"
                style="background:var(--primary)">
          + Variabel Baru
        </button>
      </div>

      {#if tampil_form_var}
        <div class="mb-4 rounded-lg border p-4" style="background:var(--surface);border-color:var(--border)">
          <h3 class="font-medium mb-3">Variabel Baru</h3>
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-sm mb-1">Nama <span class="text-red-500">*</span></label>
              <input bind:value={form_var.name} class="w-full border rounded px-2 py-1.5 text-sm"
                     style="border-color:var(--border)" placeholder="mis. Jumlah kelompok PKK" />
            </div>
            <div>
              <label class="block text-sm mb-1">Kode <span class="text-red-500">*</span></label>
              <input bind:value={form_var.code} class="w-full border rounded px-2 py-1.5 text-sm font-mono"
                     style="border-color:var(--border)" placeholder="mis. jumlah_kelompok_pkk" />
            </div>
            <div>
              <label class="block text-sm mb-1">Tipe Data</label>
              <select bind:value={form_var.data_type} class="w-full border rounded px-2 py-1.5 text-sm"
                      style="border-color:var(--border)">
                {#each TIPE_DATA as t}
                  <option value={t.value}>{t.label}</option>
                {/each}
              </select>
            </div>
            <div>
              <label class="block text-sm mb-1">Satuan</label>
              <input bind:value={form_var.unit} class="w-full border rounded px-2 py-1.5 text-sm"
                     style="border-color:var(--border)" placeholder="mis. kelompok, orang, rupiah" />
            </div>
            <div class="col-span-2">
              <label class="block text-sm mb-1">Konsep / Definisi</label>
              <textarea bind:value={form_var.definition} rows="2"
                        class="w-full border rounded px-2 py-1.5 text-sm resize-none"
                        style="border-color:var(--border)"></textarea>
            </div>
            <div class="flex items-center gap-2">
              <input type="checkbox" id="wajib" bind:checked={form_var.is_required} />
              <label for="wajib" class="text-sm">Wajib diisi</label>
            </div>
          </div>
          {#if form_var_error}
            <p class="mt-2 text-sm text-red-600">{form_var_error}</p>
          {/if}
          <div class="mt-3 flex gap-2">
            <button onclick={simpan_variabel} disabled={form_var_loading}
                    class="px-3 py-1.5 rounded text-sm text-white disabled:opacity-50"
                    style="background:var(--primary)">
              {form_var_loading ? 'Menyimpan...' : 'Simpan'}
            </button>
            <button onclick={() => { tampil_form_var = false; form_var_error = ''; }}
                    class="px-3 py-1.5 rounded text-sm border" style="border-color:var(--border)">
              Batal
            </button>
          </div>
        </div>
      {/if}

      {#if variabel.length === 0}
        <div class="rounded-lg border p-6 text-center" style="border-color:var(--border)">
          <p class="text-sm" style="color:var(--muted)">Belum ada variabel.</p>
        </div>
      {:else}
        <div class="rounded-lg border overflow-hidden" style="border-color:var(--border)">
          <table class="w-full text-sm">
            <thead style="background:#f3f4f6">
              <tr>
                <th class="text-left px-4 py-2 font-medium">#</th>
                <th class="text-left px-4 py-2 font-medium">Nama</th>
                <th class="text-left px-4 py-2 font-medium">Kode</th>
                <th class="text-left px-4 py-2 font-medium">Tipe</th>
                <th class="text-left px-4 py-2 font-medium">Satuan</th>
                <th class="text-left px-4 py-2 font-medium">Wajib</th>
                <th class="px-4 py-2"></th>
              </tr>
            </thead>
            <tbody>
              {#each variabel as v}
                <tr class="border-t" style="border-color:var(--border)">
                  <td class="px-4 py-2.5" style="color:var(--muted)">{v.sort_order}</td>
                  <td class="px-4 py-2.5 font-medium">{v.name}</td>
                  <td class="px-4 py-2.5 font-mono text-xs" style="color:var(--muted)">{v.code}</td>
                  <td class="px-4 py-2.5" style="color:var(--muted)">{v.data_type}</td>
                  <td class="px-4 py-2.5" style="color:var(--muted)">{v.unit ?? '—'}</td>
                  <td class="px-4 py-2.5">{v.is_required ? '✓' : ''}</td>
                  <td class="px-4 py-2.5 text-right">
                    <button onclick={() => arsipkan(v.id, v.name)}
                            class="text-xs hover:underline" style="color:var(--muted)">
                      Arsipkan
                    </button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}

    <!-- Tab: Riwayat Perubahan -->
    {:else if tab === 'riwayat'}
      {#if audit.length === 0}
        <p class="text-sm" style="color:var(--muted)">Belum ada riwayat perubahan.</p>
      {:else}
        <div class="rounded-lg border overflow-hidden" style="border-color:var(--border)">
          <table class="w-full text-sm">
            <thead style="background:#f3f4f6">
              <tr>
                <th class="text-left px-4 py-2 font-medium">Waktu</th>
                <th class="text-left px-4 py-2 font-medium">Aksi</th>
                <th class="text-left px-4 py-2 font-medium">Kolom</th>
                <th class="text-left px-4 py-2 font-medium">Sebelum</th>
                <th class="text-left px-4 py-2 font-medium">Sesudah</th>
                <th class="text-left px-4 py-2 font-medium">Oleh</th>
              </tr>
            </thead>
            <tbody>
              {#each audit as a}
                <tr class="border-t" style="border-color:var(--border)">
                  <td class="px-4 py-2.5 whitespace-nowrap" style="color:var(--muted)">{format_tanggal(a.changed_at)}</td>
                  <td class="px-4 py-2.5">
                    <span class="px-1.5 py-0.5 rounded text-xs
                      {a.action === 'create' ? 'bg-green-100 text-green-700' :
                       a.action === 'delete'  ? 'bg-red-100 text-red-700' :
                       a.action === 'archive' ? 'bg-orange-100 text-orange-700' :
                       'bg-blue-100 text-blue-700'}">
                      {a.action}
                    </span>
                  </td>
                  <td class="px-4 py-2.5 font-mono text-xs" style="color:var(--muted)">{a.field ?? '—'}</td>
                  <td class="px-4 py-2.5 max-w-32 truncate" style="color:var(--muted)" title={a.old_value ?? ''}>{a.old_value ?? '—'}</td>
                  <td class="px-4 py-2.5 max-w-32 truncate" title={a.new_value ?? ''}>{a.new_value ?? '—'}</td>
                  <td class="px-4 py-2.5" style="color:var(--muted)">{a.changed_by}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    {/if}
  </div>
{/if}
