<script lang="ts">
  import {latte, macchiato} from "@client/core/theme"
  import { useTheme } from 'svelte-themes'
  import type {ThemeName} from "@client/core/theme"
  import type {ThemeResult} from "@client/core/themeUtils"
  import {createThemeStyles} from "@client/core/themeUtils"

  interface AuthScreenProps {
    onAuth: () => void
  }

  let { onAuth }: AuthScreenProps = $props()

  const themeStore = useTheme()
  const currentTheme = $derived((themeStore.resolvedTheme as ThemeName) || 'light')
  const { styles }: ThemeResult = $derived(createThemeStyles(currentTheme))

  let tokenInput = $state('')
  let authError: string | null = $state(null)

  async function submitToken() {
    authError = null

    try {
      const res = await fetch('/api/auth', {
        method: 'POST',
        body: JSON.stringify({ token: tokenInput }),
        credentials: 'same-origin',
      })

      if (!res.ok) throw new Error('Invalid token')

      onAuth()
    } catch {
      authError = 'Invalid access token'
    }
  }
</script>

<div class="min-h-screen flex items-center justify-center px-4" style={styles.body}>
  <div class="w-full max-w-sm rounded-lg border p-6 shadow-sm" style={styles.container}>
    <h1 class="text-lg font-semibold mb-4 text-center" style={styles.heading}>
      🔒 Access BlackFiles
    </h1>

    <input
      type="password"
      placeholder="Access token"
      class="w-full px-3 py-2 rounded-md border mb-3 text-sm"
      style={styles.text}
      bind:value={tokenInput}
      onkeydown={(e) => e.key === 'Enter' && submitToken()}
    />

    {#if authError}
      <div
        class="text-sm mb-3"
        style={`color: ${currentTheme === 'dark' ? macchiato.red : latte.red}`}
      >
        {authError}
      </div>
    {/if}

    <button class="w-full py-2 rounded-md text-sm font-medium" style={styles.button} onclick={submitToken}>
      Enter
    </button>
  </div>
</div>