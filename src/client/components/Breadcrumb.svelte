<script lang="ts">
  import { goto } from "$app/navigation"
  import { resolve } from "$app/paths"
  import { useTheme } from "svelte-themes"
  import type {ThemeName} from "@client/core/theme"
  import {createThemeStyles, type ThemeResult} from "@client/core/themeUtils"

  let { pathParts } = $props()

  const themeStore = useTheme()
  const currentTheme = $derived((themeStore.resolvedTheme as ThemeName) || 'light')
  const { styles }: ThemeResult = $derived(createThemeStyles(currentTheme))

  async function handleBreadcrumbClick(index: number) {
    const path = index === -1 ? '' : pathParts.slice(0, index + 1).join('/')
    await goto(resolve(path ? `/${path}` : '/'))
  }
</script>

<div class="ml-6 mb-3 text-sm">
  <button
    class="cursor-pointer hover:underline"
    style={styles.link}
    onclick={() => handleBreadcrumbClick(-1)}
  >
    storage
  </button>
  {#each pathParts as part, i (i)}
    {#if i === 0}
      <span style={styles.separator}>/</span>
    {:else}
      <span style={styles.separator}>&nbsp;/</span>
    {/if}
    <button
      class="cursor-pointer hover:underline"
      style={styles.link}
      onclick={() => handleBreadcrumbClick(i)}
    >
      {part}
    </button>
  {/each}
</div>