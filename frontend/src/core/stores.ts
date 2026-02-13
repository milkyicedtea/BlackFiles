import {type Readable, writable} from 'svelte/store'
import type {Theme} from "./theme"
import {isValidTheme} from "./themeUtils"
import {browser} from "$app/environment"

interface ThemeStore extends Readable<Theme> {
  toggle: () => void
}

function createThemeStore(): ThemeStore | void {
  if (!browser) {
    return
  }
  const saved = localStorage.getItem('theme')
  const initial: Theme = isValidTheme(saved) ? saved : 'light'

  const { subscribe, update } = writable<Theme>(initial)

  return {
    subscribe,
    toggle: () => {
      update(current => {
        const newTheme = current === 'dark' ? 'light' : 'dark'
        localStorage.setItem('theme', newTheme)
        document.documentElement.setAttribute('data-theme', newTheme)
        return newTheme
      })
    }
  }
}

export const theme = createThemeStore()