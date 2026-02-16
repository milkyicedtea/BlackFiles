<script lang="ts">
  import GoogleSpin from "@client/components/GoogleSpin.svelte"
  import { useTheme } from "svelte-themes"
  import type { ThemeName } from "@client/core/theme"
  import { createThemeStyles, type ThemeResult } from "@client/core/themeUtils"

  const themeStore = useTheme()
  const currentTheme = $derived((themeStore.resolvedTheme as ThemeName) || 'light')
  const { styles }: ThemeResult = $derived(createThemeStyles(currentTheme))
</script>

<div style={styles.body} class="min-h-screen flex items-center justify-center">
  <GoogleSpin/>
</div>
