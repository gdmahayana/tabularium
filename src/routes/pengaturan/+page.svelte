<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '#lib/api';

  let backups = $state<string[]>([]);
  let loading  = $state(false);
  let pesan    = $state('');
  let error    = $state('');

  async function muat() {
    try { backups = await api.backup.daftar(); }
    catch (e) { error = String(e); }
  }

  async function buat_backup() {
    loading = true; pesan = ''; error = '';
    try {
      const path = await api.backup.buat();
      pesan = `✓ Backup berhasil: ${path.split('/').pop()}`;
      await muat();
    } catch (e) { error = String(e); }
    finally { loading = false; }
  }

  async function restore(path: string) {
    if (!confirm(`Pulihkan dari backup ini?\n${path.split('/').pop()}\n\nSeluruh data saat ini akan digantikan.`)) return;
    loading = true; error = '';
    try { await api.backup.restore(path); pesan = '✓ Pemulihan berhasil. Restart aplikasi jika diperlukan.'; }
    catch (e) { error = String(e); }
    finally { loading = false; }
  }

  onMount(muat);
</script>

<div class="p-6">
  <h1 class="text-xl font-semibold mb-1">Pengaturan</h1>
  <p class="text-sm mb-6" style="color:var(--muted)">Kelola backup dan pemulihan data.</p>

  {#if pesan}<div class="mb-4 p-3 rounded-lg text-sm bg-green-50 text-green-700">{pesan}</div>{/if}
  {#if error}<p class="text-sm text-red-600 mb-4">{error}</p>{/if}

  <div class="max-w-lg">
    <div class="flex items-center justify-between mb-4">
      <h2 class="font-medium">Backup & Pemulihan</h2>
      <button onclick={buat_backup} disabled={loading}
        class="px-3 py-1.5 text-sm rounded-md text-white" style="background:var(--primary)">
        {loading ? 'Memproses...' : '+ Buat Backup'}
      </button>
    </div>

    {#if backups.length === 0}
      <p class="text-sm" style="color:var(--muted)">Belum ada backup.</p>
    {:else}
      <div class="border rounded-lg overflow-hidden" style="border-color:var(--border)">
        {#each backups as b, i}
          <div class="flex items-center justify-between px-4 py-2.5 text-sm
                      {i > 0 ? 'border-t' : ''}" style="border-color:var(--border)">
            <span class="font-mono text-xs">{b.split('/').pop()}</span>
            <button onclick={() => restore(b)}
              class="text-xs text-amber-600 hover:underline">Pulihkan</button>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>