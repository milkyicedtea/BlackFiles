<script lang="ts">
  import { onMount } from 'svelte';
  import { theme } from './core/stores';
  import { formatSize, formatDate } from './core/formatUtils';

  interface FileItem {
    name: string;
    path: string;
    is_dir: boolean;
    modified: number;
    size: number;
  }

  let files: FileItem[] = [];
  let loading = true;
  let error: string | null = null;
  let currentPath = '';

  $: pathParts = currentPath ? currentPath.split('/').filter(Boolean) : [];

  // Theme-dependent classes
  $: bodyClass = $theme === 'dark' ? 'min-h-screen bg-[#191919] p-8' : 'min-h-screen bg-[#f7f6f3] p-8';
  $: h1Class = $theme === 'dark' ? 'text-lg font-bold text-gray-100' : 'text-lg font-bold text-gray-800';
  $: containerClass = $theme === 'dark'
    ? 'bg-[#202020] rounded-lg border border-gray-800 shadow-sm overflow-hidden'
    : 'bg-white rounded-lg border border-gray-200 shadow-sm overflow-hidden';
  $: linkClass = $theme === 'dark' ? '!text-blue-400 hover:underline' : '!text-blue-600 hover:underline';
  $: separatorClass = $theme === 'dark' ? 'text-gray-600' : 'text-gray-400';
  $: hoverClass = $theme === 'dark' ? 'hover:bg-gray-800/50' : 'hover:bg-gray-50';
  $: textClass = $theme === 'dark' ? 'text-gray-200' : 'text-gray-800';
  $: mutedClass = 'text-gray-500';
  $: dirClass = $theme === 'dark' ? 'text-blue-400 font-medium' : 'text-blue-600 font-medium';
  $: loadingClass = $theme === 'dark' ? 'text-gray-600' : 'text-gray-400';
  $: errorBg = $theme === 'dark' ? 'bg-red-900/20' : 'bg-red-50';
  $: errorText = $theme === 'dark' ? 'text-red-400' : 'text-red-700';
  $: errorBorder = $theme === 'dark' ? 'border-red-900/50' : 'border-red-200';

  function handleFileClick(file: FileItem) {
    if (file.is_dir) {
      loadDirectory(file.path);
    } else {
      // Simple navigation triggers download - same as original
      window.location.href = `/files/${file.path}`;
    }
  }

  async function loadDirectory(path: string, pushState: boolean = true) {
    loading = true;
    error = null;

    try {
      const url = path ? `/api/list/${path}` : '/api/list';
      const response = await fetch(url);

      if (!response.ok) {
        throw new Error('Failed to load directory');
      }

      files = await response.json();
      currentPath = path;

      if (pushState) {
        const newUrl = path ? `/${path}` : '/';
        window.history.pushState({ path }, '', newUrl);
      }
    } catch (err: any) {
      error = err.message;
      files = [];
    } finally {
      loading = false;
    }
  }

  function handleBreadcrumbClick(index: number) {
    const path = index === -1 ? '' : pathParts.slice(0, index + 1).join('/');
    loadDirectory(path);
  }

  function toggleTheme() {
    theme.set($theme === 'dark' ? 'light' : 'dark');
  }

  onMount(() => {
    // Set initial theme
    document.documentElement.setAttribute('data-theme', $theme);

    // Handle browser back/forward
    window.addEventListener('popstate', (event) => {
      const path = event.state?.path || '';
      loadDirectory(path, false);
    });

    // Load initial directory
    const initialPath = window.location.pathname.replace(/^\/+/, '');
    if (!initialPath.startsWith('api/') && !initialPath.startsWith('files/')) {
      loadDirectory(initialPath, false);
    }
  });
</script>

<div class={bodyClass}>
  <div class="max-w-4xl mx-auto">
    <!-- Header -->
    <div class="mb-4 flex justify-between items-center">
      <h1 class={h1Class}>📁 BlackFiles Browser</h1>
      <button class="btn btn-soft {$theme === 'dark' ? 'text-gray-100' : 'text-gray-600'}" on:click={toggleTheme}>
        {$theme === 'dark' ? '☀️ Light' : '🌙 Dark'}
      </button>
    </div>

    <!-- Breadcrumb -->
    <div class="ml-6 mb-3 text-sm">
      <button class="{linkClass} cursor-pointer" on:click={() => handleBreadcrumbClick(-1)}>
        storage
      </button>
      {#each pathParts as part, i}
        {#if i === 0}
          <span class={separatorClass}>/</span>
        {:else}
          <span class={separatorClass}>&nbsp;/</span>
        {/if}
        <button class="{linkClass} cursor-pointer" on:click={() => handleBreadcrumbClick(i)}>
          {part}
        </button>
      {/each}
    </div>

    <!-- File List Container -->
    <div class={containerClass}>
      <ul>
        {#if loading}
          <li class="px-6 py-12 text-center {loadingClass} text-sm">
            Loading...
          </li>
        {:else if error}
          <li class="px-6 py-5 {errorBg} {errorText} {errorBorder} mx-4 my-4 rounded-lg border text-sm">
            Error loading directory: {error}
          </li>
        {:else if files.length === 0}
          <li class="px-6 py-16 text-center {loadingClass} text-sm">
            This folder is empty
          </li>
        {:else}
          {#each files as file, i}
            <!-- Mobile: Stacked layout -->
            <li class="md:hidden relative">
              {#if i > 0}
                <div class="absolute top-0 left-1/2 -translate-x-1/2 w-[90%] h-px {$theme === 'dark' ? 'bg-gray-700' : 'bg-gray-200'}"></div>
              {/if}
              <div
                role="button"
                tabindex="0"
                class="px-4 py-3 {hoverClass} cursor-pointer transition-colors"
                on:click={() => handleFileClick(file)}
                on:keydown={(e) => e.key === 'Enter' && handleFileClick(file)}>
                <div class="flex items-start gap-3">
                  <span class="text-xl mt-0.5">{file.is_dir ? '📁' : '📄'}</span>
                  <div class="flex-1 min-w-0">
                    <div class="text-sm {file.is_dir ? dirClass : textClass} truncate">
                      {file.name}
                    </div>
                    <div class="flex gap-3 text-xs {mutedClass} mt-1">
                      <span>{file.is_dir ? '—' : formatSize(file.size)}</span>
                      <span>•</span>
                      <span>{formatDate(file.modified)}</span>
                    </div>
                  </div>
                </div>
              </div>
            </li>

            <!-- Desktop: Grid layout -->
            <li class="hidden md:block relative">
              {#if i > 0}
                <div class="absolute top-0 left-1/2 -translate-x-1/2 w-[90%] h-px {$theme === 'dark' ? 'bg-gray-700' : 'bg-gray-200'}"></div>
              {/if}
              <div
                role="button"
                tabindex="0"
                class="grid px-6 py-3 {hoverClass} cursor-pointer transition-colors grid-cols-[24px_1fr_auto] gap-4 items-center"
                on:click={() => handleFileClick(file)}
                on:keydown={(e) => e.key === 'Enter' && handleFileClick(file)}>
                <span class="text-xl">{file.is_dir ? '📁' : '📄'}</span>
                <span class="text-sm {file.is_dir ? dirClass : textClass} truncate">
                  {file.name}
                </span>
                <div class="flex items-center gap-2 text-xs {mutedClass}">
                  <span class="whitespace-nowrap">
                    {file.is_dir ? '—' : formatSize(file.size)}
                  </span>
                  <span>•</span>
                  <span class="whitespace-nowrap">
                    {formatDate(file.modified)}
                  </span>
                </div>
              </div>
            </li>
          {/each}
        {/if}
      </ul>
    </div>
  </div>
</div>

<style>
  @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap');

  :global(body) {
    font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    margin: 0;
    padding: 0;
  }
</style>