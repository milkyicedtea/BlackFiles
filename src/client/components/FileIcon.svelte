<script lang="ts">
  import { useTheme } from 'svelte-themes'
  import type { ThemeName } from '@client/core/theme'
  import { getFileIcon } from "@client/utils/fileIconMap"

  interface Props {
    fileName: string
    isDirectory: boolean
    size?: 'sm' | 'md' | 'lg'
  }

  let { fileName, isDirectory, size = 'md' }: Props = $props()

  const themeStore = useTheme()
  const currentTheme = $derived((themeStore.resolvedTheme as ThemeName) || 'light')

  const palette = $derived(currentTheme === 'dark' ? 'macchiato' : 'latte')

  const iconName = $derived(getFileIcon(fileName, isDirectory))

  const iconPath = $derived(`/icons/${palette}/${iconName}.svg`)

  const sizeClass = $derived(
    size === 'sm' ? 'w-4 h-4' :
    size === 'lg' ? 'w-8 h-8' :
    'w-5 h-5'
  )
</script>

<img
  src={iconPath}
  alt={isDirectory ? '📁': '📄'}
  class={`${sizeClass} inline-block align-middle`}
/>