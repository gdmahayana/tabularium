<script lang="ts">
  import '../app.css';
  import { page } from '$app/state';
  let { children } = $props();

  const menu = [
    { href: '/',                label: 'Ringkasan' },
    { href: '/katalog',         label: 'Katalog Data' },
    { href: '/pengumpulan',     label: 'Pengumpulan Data' },
    { href: '/kualitas',        label: 'Kualitas Data' },
    { href: '/audit',           label: 'Verifikasi & Audit' },
    { href: '/intelijen',       label: 'Intelijen Statistik' },
    { href: '/integrasi',       label: 'Integrasi Data' },
    { href: '/lembar-sdi',      label: 'Lembar Input SDI' },
    { href: '/pengaturan',      label: 'Pengaturan' },
  ];

  function aktif(href: string): boolean {
    if (href === '/') return page.url.pathname === '/';
    return page.url.pathname.startsWith(href);
  }
</script>

<div class="flex h-screen overflow-hidden">
  <!-- Sidebar -->
  <aside class="w-52 shrink-0 border-r flex flex-col bg-white" style="border-color:var(--border)">
    <div class="px-4 py-4 border-b" style="border-color:var(--border)">
      <div class="font-bold text-sm tracking-tight">TABULARIUM</div>
      <div class="text-xs mt-0.5" style="color:var(--muted)">From Data to Knowledge</div>
    </div>
    <nav class="flex-1 py-2 overflow-y-auto">
      {#each menu as m}
        <a
          href={m.href}
          class="flex items-center px-4 py-2 rounded-md mx-2 my-0.5 text-sm transition-colors
                 {aktif(m.href)
                   ? 'bg-blue-50 text-blue-700 font-medium'
                   : 'text-gray-600 hover:bg-gray-100'}"
        >
          {m.label}
        </a>
      {/each}
    </nav>
    <div class="px-4 py-3 border-t text-xs" style="border-color:var(--border);color:var(--muted)">
      v0.1.0 · Tahap 3
    </div>
  </aside>

  <!-- Konten utama -->
  <main class="flex-1 overflow-y-auto">
    {@render children()}
  </main>
</div>