<script lang="ts">
  import '../app.css';
  import { page } from '$app/state';
  let { children } = $props();

  const menu = [
    { href: '/',         label: 'Ringkasan' },
    { href: '/katalog',  label: 'Katalog Data' },
    { href: '/impor',    label: 'Impor Data' },
  ];
</script>

<div class="flex h-screen">
  <!-- Sidebar -->
  <aside class="w-52 shrink-0 border-r flex flex-col bg-white" style="border-color:var(--border)">
    <div class="px-4 py-4 border-b font-bold text-base tracking-tight" style="border-color:var(--border)">
      TABULARIUM
    </div>
    <nav class="flex-1 py-2">
      {#each menu as m}
        <a
          href={m.href}
          class="flex items-center px-4 py-2 rounded-md mx-2 my-0.5 text-sm transition-colors
                 {page.url.pathname === m.href || (page.url.pathname.startsWith(m.href) && m.href !== '/')
                   ? 'bg-blue-50 text-blue-700 font-medium'
                   : 'text-gray-600 hover:bg-gray-100'}"
        >
          {m.label}
        </a>
      {/each}
    </nav>
    <div class="px-4 py-3 text-xs border-t" style="color:var(--muted);border-color:var(--border)">
      v0.1.0 · Tahap 2
    </div>
  </aside>

  <!-- Konten utama -->
  <main class="flex-1 overflow-auto">
    {@render children()}
  </main>
</div>
