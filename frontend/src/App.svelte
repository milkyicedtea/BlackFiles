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

  // Catppuccin color schemes
  // Latte (light) colors
  const latte = {
    base: '#eff1f5',
    mantle: '#e6e9ef',
    surface0: '#ccd0da',
    surface1: '#bcc0cc',
    surface2: '#acb0be',
    overlay0: '#9ca0b0',
    overlay1: '#8c8fa1',
    text: '#4c4f69',
    subtext0: '#6c6f85',
    subtext1: '#5c5f77',
    blue: '#1e66f5',
    red: '#d20f39',
  };

  // Macchiato (dark) colors
  const macchiato = {
    base: '#24273a',
    mantle: '#1e2030',
    surface0: '#363a4f',
    surface1: '#494d64',
    surface2: '#5b6078',
    overlay0: '#6e738d',
    overlay1: '#8087a2',
    text: '#cad3f5',
    subtext0: '#a5adcb',
    subtext1: '#b8c0e0',
    blue: '#8aadf4',
    red: '#ed8796',
  };

  // Theme-dependent classes using Catppuccin colors
  $: bodyClass = $theme === 'dark'
    ? 'min-h-screen p-8'
    : 'min-h-screen p-8';
  $: bodyStyle = $theme === 'dark'
    ? `background-color: ${macchiato.base}`
    : `background-color: ${latte.base}`;

  $: h1Class = $theme === 'dark' ? 'text-lg font-bold' : 'text-lg font-bold';
  $: h1Style = $theme === 'dark'
    ? `color: ${macchiato.text}`
    : `color: ${latte.text}`;

  $: containerClass = 'rounded-lg border shadow-sm overflow-hidden';
  $: containerStyle = $theme === 'dark'
    ? `background-color: ${macchiato.surface0}; border-color: ${macchiato.surface2}`
    : `background-color: ${latte.mantle}; border-color: ${latte.surface1}`;

  $: linkStyle = $theme === 'dark'
    ? `color: ${macchiato.blue} !important`
    : `color: ${latte.blue} !important`;

  $: separatorStyle = $theme === 'dark'
    ? `color: ${macchiato.overlay0}`
    : `color: ${latte.overlay0}`;

  $: hoverClass = 'cursor-pointer transition-colors';
  $: hoverStyle = $theme === 'dark'
    ? `${macchiato.surface1}`
    : `${latte.surface0}`;

  $: textStyle = $theme === 'dark'
    ? `color: ${macchiato.text}`
    : `color: ${latte.text}`;

  $: mutedStyle = $theme === 'dark'
    ? `color: ${macchiato.subtext0}`
    : `color: ${latte.subtext0}`;

  $: dirStyle = $theme === 'dark'
    ? `color: ${macchiato.blue}; font-weight: 500`
    : `color: ${latte.blue}; font-weight: 500`;

  $: loadingStyle = $theme === 'dark'
    ? `color: ${macchiato.subtext0}`
    : `color: ${latte.subtext0}`;

  $: errorContainerStyle = $theme === 'dark'
    ? `background-color: ${macchiato.surface1}; color: ${macchiato.red}; border-color: ${macchiato.surface2}`
    : `background-color: ${latte.surface0}; color: ${latte.red}; border-color: ${latte.surface1}`;

  $: dividerStyle = $theme === 'dark'
    ? `background-color: ${macchiato.surface1}`
    : `background-color: ${latte.surface1}`;

  $: buttonStyle = $theme === 'dark'
    ? `color: ${macchiato.text}`
    : `color: ${latte.text}`;

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

<div class={bodyClass} style={bodyStyle}>
  <div class="max-w-4xl mx-auto">
    <!-- Header -->
    <div class="mb-4 flex justify-between items-center">
      <h1 class={h1Class} style={h1Style}>📁 BlackFiles Browser</h1>
      <button class="btn btn-soft" style={buttonStyle} on:click={toggleTheme}>
        {$theme === 'dark' ? '☀️ Latte' : '🌙 Macchiato'}
      </button>
    </div>

    <!-- Breadcrumb -->
    <div class="ml-6 mb-3 text-sm">
      <button class="cursor-pointer hover:underline" style={linkStyle} on:click={() => handleBreadcrumbClick(-1)}>
        storage
      </button>
      {#each pathParts as part, i}
        {#if i === 0}
          <span style={separatorStyle}>/</span>
        {:else}
          <span style={separatorStyle}>&nbsp;/</span>
        {/if}
        <button class="cursor-pointer hover:underline" style={linkStyle} on:click={() => handleBreadcrumbClick(i)}>
          {part}
        </button>
      {/each}
    </div>

    <!-- File List Container -->
    <div class={containerClass} style={containerStyle}>
      <ul>
        {#if loading}
          <li class="px-6 py-12 text-center text-sm" style={loadingStyle}>
            Loading...
          </li>
        {:else if error}
          <li class="px-6 py-5 mx-4 my-4 rounded-lg border text-sm" style={errorContainerStyle}>
            Error loading directory: {error}
          </li>
        {:else if files.length === 0}
          <li class="px-6 py-16 text-center text-sm" style={loadingStyle}>
            This folder is empty
          </li>
        {:else}
          {#each files as file, i}
            <!-- Mobile: Stacked layout -->
            <li class="md:hidden relative">
              {#if i > 0}
                <div class="absolute top-0 left-1/2 -translate-x-1/2 w-[90%] h-px" style={dividerStyle}></div>
              {/if}
              <div
                role="button"
                tabindex="0"
                class={hoverClass}
                style="padding: 0.75rem 1rem"
                on:mouseenter={(e) => e.currentTarget.style.backgroundColor = hoverStyle}
                on:mouseleave={(e) => e.currentTarget.style.backgroundColor = 'transparent'}
                on:click={() => handleFileClick(file)}
                on:keydown={(e) => e.key === 'Enter' && handleFileClick(file)}>
                <div class="flex items-start gap-3">
                  <span class="text-xl mt-0.5">{file.is_dir ? '📁' : '📄'}</span>
                  <div class="flex-1 min-w-0">
                    <div class="text-sm truncate" style={file.is_dir ? dirStyle : textStyle}>
                      {file.name}
                    </div>
                    <div class="flex gap-3 text-xs mt-1" style={mutedStyle}>
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
                <div class="absolute top-0 left-1/2 -translate-x-1/2 w-[90%] h-px" style={dividerStyle}></div>
              {/if}
              <div
                role="button"
                tabindex="0"
                class={hoverClass}
                style="display: grid; grid-template-columns: 24px 1fr auto; gap: 1rem; align-items: center; padding: 0.75rem 1.5rem"
                on:mouseenter={(e) => e.currentTarget.style.backgroundColor = hoverStyle}
                on:mouseleave={(e) => e.currentTarget.style.backgroundColor = 'transparent'}
                on:click={() => handleFileClick(file)}
                on:keydown={(e) => e.key === 'Enter' && handleFileClick(file)}>
                <span class="text-xl">{file.is_dir ? '📁' : '📄'}</span>
                <span class="text-sm truncate" style={file.is_dir ? dirStyle : textStyle}>
                  {file.name}
                </span>
                <div class="flex items-center gap-2 text-xs" style={mutedStyle}>
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