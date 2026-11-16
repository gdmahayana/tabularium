<script lang="ts">
  import ImporWizard from '#lib/Impor.svelte';
  let tab = $state<'impor' | 'manual'>('impor');
</script>

<div class="p-6">
  <h1 class="text-xl font-semibold mb-1">Pengumpulan Data</h1>
  <p class="text-sm mb-5" style="color:var(--muted)">
    Masukkan data ke dataset — lewat impor file atau input manual baris per baris.
  </p>

  <div class="flex gap-1 border-b mb-6" style="border-color:var(--border)">
    {#each [['impor','📥 Impor File'], ['manual','✏️ Input Manual']] as [val, label]}
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

  {#if tab === 'impor'}
    <ImporWizard />
  {:else}
    <div class="rounded-lg border p-10 text-center" style="border-color:var(--border)">
      <p class="font-medium mb-1">Input Manual</p>
      <p class="text-sm" style="color:var(--muted)">
        Pilih dataset dan isi nilai satu per satu. Tersedia di pembaruan berikutnya.
      </p>
    </div>
  {/if}
</div>