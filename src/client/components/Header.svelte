<script lang="ts">
  import blackFolder from "@client/core/assets/icons8-folder-color-120.png"
  import { useTheme } from 'svelte-themes'
  import type {ThemeName} from "@client/core/theme"
  import {createThemeStyles, type ThemeResult} from "@client/core/themeUtils"

  const themeStore = useTheme()
  const currentTheme = $derived((themeStore.resolvedTheme as ThemeName) || 'light')
  const { classes, styles }: ThemeResult = $derived(createThemeStyles(currentTheme))

  function toggleTheme() {
    themeStore.theme = currentTheme === 'dark' ? 'light' : 'dark'
  }
</script>

<div class="mb-4 flex justify-between items-center select-none">
  <div class="flex items-center">
    <img width="40rem" src={blackFolder} alt="" draggable="false" />
    &nbsp;
    <h1 class={classes.heading} style={styles.heading}>BlackFiles Browser</h1>
  </div>

  <button class="btn btn-soft" style={styles.button} onclick={toggleTheme}>
    {currentTheme === 'dark' ? '🌻 Latte' : '🌺 Macchiato'}
  </button>
</div>