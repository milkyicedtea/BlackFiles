<script lang="ts">
  import { onMount } from 'svelte'
  import { theme } from './core/stores'
  import { formatSize, formatDate } from './core/formatUtils'
  import { latte, macchiato } from "./core/theme";
  import {createThemeStyles, type ThemeClasses, type ThemeStyles} from "./core/themeUtils";

  let tokenInput = ""
  let authenticated = false
  let authError: string | null = null

  interface FileItem {
    name: string
    path: string
    is_dir: boolean
    modified: number
    size: number
  }

  let files: FileItem[] = []
  let loading = true
  let error: string | null = null
  let currentPath = ''

  $: pathParts = currentPath ? currentPath.split('/').filter(Boolean) : []

  // theming
  let classes: ThemeClasses
  let styles: ThemeStyles

  $: ({ classes, styles } = createThemeStyles($theme))

  async function handleFileClick(file: FileItem) {
    if (file.is_dir) {
      await loadDirectory(file.path)
    } else {
      // instead of navigating, try fetching
      window.open(`/files/${file.path}`)
    }
  }

  async function submitToken() {
    authError = null

    try {
      const res = await fetch("/api/auth", {
        method: "POST",
        body: JSON.stringify({ token: tokenInput }),
        credentials: 'include'
      })

      if (!res.ok) throw new Error("Invalid token")

      authenticated = true
      await loadDirectory("", false)
    } catch {
      authError = "Invalid access token"
    }
  }

  async function loadDirectory(path: string, pushState: boolean = true) {
    loading = true
    error = null

    try {
      const url = path ? `/api/list/${path}` : '/api/list'
      const response = await fetch(url, {
        credentials: 'include'
      })

      if (!response.ok) {
        if (response.status === 401) {
          authenticated = false
          return
        }
        throw new Error('Failed to load directory')
      }

      files = await response.json()
      currentPath = path

      if (pushState) {
        const newUrl = path ? `/${path}` : '/'
        window.history.pushState({ path }, '', newUrl)
      }
    } catch (err: any) {
      error = err.message
      files = []
    } finally {
      loading = false
    }
  }

  function handleBreadcrumbClick(index: number) {
    const path = index === -1 ? '' : pathParts.slice(0, index + 1).join('/')
    loadDirectory(path)
  }

  onMount(async () => {
    document.documentElement.setAttribute('data-theme', $theme)

    // Check if already authenticated via cookie
    try {
      const res = await fetch("/api/check", {
        credentials: "include"
      })

      if (res.ok) {
        authenticated = true

        // Handle browser back/forward
        window.addEventListener('popstate', (event) => {
          const path = event.state?.path || ''
          loadDirectory(path, false)
        })

        // Load initial directory
        const initialPath = window.location.pathname.replace(/^\/+/, '')
        if (!initialPath.startsWith('api/') && !initialPath.startsWith('files/')) {
          await loadDirectory(initialPath, false)
        }
      }
    } catch {
      authenticated = false
    }
  })

</script>

{#if !authenticated}
  <div class="min-h-screen flex items-center justify-center px-4" style={styles.body}>
    <div
      class="w-full max-w-sm rounded-lg border p-6 shadow-sm"
      style={styles.container}
    >
      <h1 class="text-lg font-semibold mb-4 text-center" style={styles.heading}>
        🔒 Access BlackFiles
      </h1>

      <input
        type="password"
        placeholder="Access token"
        class="w-full px-3 py-2 rounded-md border mb-3 text-sm"
        style={styles.text}
        bind:value={tokenInput}
        on:keydown={(e) => e.key === 'Enter' && submitToken()}
      />

      {#if authError}
        <div class="text-sm mb-3" style={`color: ${$theme === 'dark' ? macchiato.red : latte.red}`}>
          {authError}
        </div>
      {/if}

      <button
        class="w-full py-2 rounded-md text-sm font-medium"
        style={styles.button}
        on:click={submitToken}
      >
        Enter
      </button>
    </div>
  </div>
{:else}
  <div class={classes.body} style={styles.body}>
    <div class="max-w-4xl mx-auto">
      <!-- Header -->
      <div class="mb-4 flex justify-between items-center">
        <h1 class={classes.heading} style={styles.heading}>🗂️ BlackFiles Browser</h1>
        <button class="btn btn-soft" style={styles.button} on:click={theme.toggle}>
          {$theme === 'dark' ? '☀️ Latte' : '🌙 Macchiato'}
        </button>
      </div>

      <!-- Breadcrumb -->
      <div class="ml-6 mb-3 text-sm">
        <button class="cursor-pointer hover:underline" style={styles.link} on:click={() => handleBreadcrumbClick(-1)}>
          storage
        </button>
        {#each pathParts as part, i}
          {#if i === 0}
            <span style={styles.separator}>/</span>
          {:else}
            <span style={styles.separator}>&nbsp;/</span>
          {/if}
          <button class="cursor-pointer hover:underline" style={styles.link} on:click={() => handleBreadcrumbClick(i)}>
            {part}
          </button>
        {/each}
      </div>

      <!-- File List Container -->
      <div class={classes.container} style={styles.container}>
        <ul>
          {#if loading}
            <li class="px-6 py-12 text-center text-sm" style={styles.loading}>
              Loading...
            </li>
          {:else if error}
            <li class="px-6 py-5 mx-4 my-4 rounded-lg border text-sm" style={styles.error}>
              Error loading directory: {error}
            </li>
          {:else if files.length === 0}
            <li class="px-6 py-16 text-center text-sm" style={styles.loading}>
              This folder is empty
            </li>
          {:else}
            {#each files as file, i}
              <!-- Mobile: Stacked layout -->
              <li class="md:hidden relative">
                {#if i > 0}
                  <div class="absolute top-0 left-1/2 -translate-x-1/2 w-[90%] h-px" style={styles.divider}></div>
                {/if}
                <div
                  role="button"
                  tabindex="0"
                  class={classes.hover}
                  style="padding: 0.75rem 1rem"
                  on:mouseenter={(e) => {
                    e.currentTarget.style.backgroundColor = styles.hover
                  }}
                  on:mouseleave={(e) => {
                    e.currentTarget.style.backgroundColor = 'transparent'
                  }}
                  on:click={() => handleFileClick(file)}
                  on:keydown={(e) => e.key === 'Enter' && handleFileClick(file)}>
                  <div class="flex items-start gap-3">
                    <span class="text-xl mt-0.5">{file.is_dir ? '📁' : '📄'}</span>
                    <div class="flex-1 min-w-0">
                      <div class="text-sm truncate" style={file.is_dir ? styles.directory : styles.text}>
                        {file.name}
                      </div>
                      <div class="flex gap-3 text-xs mt-1" style={styles.muted}>
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
                  <div class="absolute top-0 left-1/2 -translate-x-1/2 w-[90%] h-px" style={styles.divider}></div>
                {/if}
                <div
                  role="button"
                  tabindex="0"
                  class={classes.hover}
                  style="display: grid; grid-template-columns: 24px 1fr auto; gap: 1rem; align-items: center; padding: 0.75rem 1.5rem"
                  on:mouseenter={(e) => {
                    e.currentTarget.style.backgroundColor = styles.hover
                  }}
                  on:mouseleave={(e) => {
                    e.currentTarget.style.backgroundColor = 'transparent'
                  }}
                  on:click={() => handleFileClick(file)}
                  on:keydown={(e) => e.key === 'Enter' && handleFileClick(file)}>
                  <span class="text-xl">{file.is_dir ? '📁' : '📄'}</span>
                  <span class="text-sm truncate" style={file.is_dir ? styles.directory : styles.text}>
                    {file.name}
                  </span>
                  <div class="flex items-center gap-2 text-xs" style={styles.muted}>
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
{/if}

<style>
  @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap');

  :global(body) {
    font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    margin: 0;
    padding: 0;
  }
</style>