<script lang="ts">
  import { page } from '$app/state'
  import { goto } from '$app/navigation'
  import { resolve } from '$app/paths'
  import { useTheme } from 'svelte-themes'
  import { formatSize, formatDate } from '@client/core/formatUtils'
  import { type ThemeName } from '@client/core/theme'
  import { createThemeStyles, type ThemeResult } from '@client/core/themeUtils'
  import type { FileItem } from '@client/types/file'
  import Header from "@client/components/Header.svelte"
  import Breadcrumb from "@client/components/Breadcrumb.svelte"
  import GoogleSpin from "@client/components/GoogleSpin.svelte"
  import FileIcon from "@client/components/FileIcon.svelte"

  let files: FileItem[] = $state([])
  let loading = $state(true)
  let error: string | null = $state(null)

  // Use page state for current path
  const currentPath = $derived(page.url.pathname.replace(/^\/+/, ''))
  const pathParts = $derived(currentPath ? currentPath.split('/').filter(Boolean) : [])

  // theming with svelte-themes
  const themeStore = useTheme()
  const currentTheme = $derived((themeStore.resolvedTheme as ThemeName) || 'light')
  const { classes, styles }: ThemeResult = $derived(createThemeStyles(currentTheme))

  // Load directory whenever the path changes
  $effect(() => {
    if (currentPath !== undefined) {
      loadDirectory(currentPath)
    }
  })

  function downloadFile(path: string) {
    const a = document.createElement('a')
    a.href = `/files/${path}`
    a.rel = 'noopener'
    a.click()
  }

  async function handleFileClick(file: FileItem) {
    if (file.is_dir) {
      // Use goto for navigation instead of manual history manipulation
      await goto(resolve(`/${file.path}`))
    } else {
      downloadFile(file.path)
    }
  }

  async function loadDirectory(path: string) {
    loading = true
    error = null

    try {
      const url = path ? `/api/list/${path}` : '/api/list'
      const response = await fetch(url, {
        credentials: 'same-origin',
      })

      if (!response.ok) {
        throw new Error('Failed to load directory')
      }

      files = await response.json()
    } catch (err: unknown) {
      if (err instanceof Error) {
        error = err.message
      } else {
        error = String(err)
      }
      files = []
    } finally {
      loading = false
    }
  }
</script>

<div class={classes.body} style={styles.body}>
  <div class="max-w-4xl mx-auto">
    <Header/>

    <!-- Breadcrumb -->
    <Breadcrumb {pathParts}/>

    <!-- File List Container -->
    <div class={classes.container} style={styles.container}>
      <ul>
        {#if loading}
          <li class="px-6 py-12 text-center text-sm" style={styles.loading}><GoogleSpin/></li>
        {:else if error}
          <li class="px-6 py-5 mx-4 my-4 rounded-lg border text-sm" style={styles.error}>
            Error loading directory: {error}
          </li>
        {:else if files.length === 0}
          <li class="px-6 py-16 text-center text-sm" style={styles.loading}>
            This folder is empty
          </li>
        {:else}
          {#each files as file, i (i)}
            <!-- Mobile: Stacked layout -->
            <li class="md:hidden relative">
              {#if i > 0}
                <div
                  class="absolute top-0 left-1/2 -translate-x-1/2 w-[90%] h-px"
                  style={styles.divider}
                ></div>
              {/if}
              <div
                role="button"
                tabindex="0"
                class={classes.hover}
                style="padding: 0.75rem 1rem"
                onmouseenter={(e) => {
                  e.currentTarget.style.backgroundColor = styles.hover
                }}
                onmouseleave={(e) => {
                  e.currentTarget.style.backgroundColor = 'transparent'
                }}
                onclick={() => handleFileClick(file)}
                onkeydown={(e) => e.key === 'Enter' && handleFileClick(file)}
              >
                <div class="flex items-start gap-3">
                  <div class="mt-0.5">
                    <FileIcon fileName={file.name} isDirectory={file.is_dir} size="md" />
                  </div>
                  <div class="flex-1 min-w-0">
                    <div
                      class="text-sm truncate"
                      style={file.is_dir ? styles.directory : styles.text}
                    >
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
                <div
                  class="absolute top-0 left-1/2 -translate-x-1/2 w-[90%] h-px"
                  style={styles.divider}
                ></div>
              {/if}
              <div
                role="button"
                tabindex="0"
                class={classes.hover}
                style="display: grid; grid-template-columns: 24px 1fr auto; gap: 1rem; align-items: center; padding: 0.75rem 1.5rem"
                onmouseenter={(e) => {
                  e.currentTarget.style.backgroundColor = styles.hover
                }}
                onmouseleave={(e) => {
                  e.currentTarget.style.backgroundColor = 'transparent'
                }}
                onclick={() => handleFileClick(file)}
                onkeydown={(e) => e.key === 'Enter' && handleFileClick(file)}
              >
                <FileIcon fileName={file.name} isDirectory={file.is_dir} size="md" />
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