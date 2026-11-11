<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "#lib/api";

  let jumlah_dataset = $state(0);
  let jumlah_variable = $state(0);
  let loading = $state(true);

  onMount(async () => {
    try {
      const datasets = await api.dataset.daftar();
      jumlah_dataset = datasets.length;
      let total_var = 0;
      for (const d of datasets) {
        const vars = await api.variable.daftar(d.id);
        total_var += vars.length;
      }
      jumlah_variable = total_var;
    } catch (e) {
      console.error(e);
    } finally {
      loading = false;
    }
  });
</script>

<div class="p-6">
  <h1 class="text-xl font-semibold mb-6">Ringkasan</h1>
  {#if loading}
    <p class="text-sm" style="color:var(--muted)">Memuat...</p>
  {:else}
    <div class="grid grid-cols-2 gap-4 max-w-md">
      <div class="rounded-lg border p-4" style="background:var(--surface);border-color:var(--border)">
        <div class="text-2xl font-bold" style="color:var(--primary)">{jumlah_dataset}</div>
        <div class="text-sm mt-1" style="color:var(--muted)">Dataset</div>
      </div>
      <div class="rounded-lg border p-4" style="background:var(--surface);border-color:var(--border)">
        <div class="text-2xl font-bold" style="color:var(--primary)">{jumlah_variable}</div>
        <div class="text-sm mt-1" style="color:var(--muted)">Variabel</div>
      </div>
    </div>
  {/if}
</div>
