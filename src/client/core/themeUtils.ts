import { themes  } from '@client/core/theme'
import type {ThemeName} from '@client/core/theme';

export interface ThemeClasses {
  body: string
  heading: string
  container: string
  hover: string
}

export interface ThemeStyles {
  body: string
  heading: string
  container: string
  link: string
  separator: string
  hover: string
  text: string
  muted: string
  directory: string
  loading: string
  error: string
  divider: string
  button: string
}

export interface ThemeResult {
  classes: ThemeClasses
  styles: ThemeStyles
}

export function createThemeStyles(themeName: ThemeName): ThemeResult {
  const colors = themes[themeName]

  return {
    classes: {
      body: 'min-h-screen p-8',
      heading: 'text-lg font-bold',
      container: 'rounded-lg border shadow-sm overflow-hidden',
      hover: 'cursor-pointer transition-colors',
    },

    styles: {
      body: `background-color: ${colors.base}`,
      heading: `color: ${colors.text}`,
      container: `background-color: ${themeName === 'dark' ? colors.surface0 : colors.mantle}; border-color: ${themeName === 'dark' ? colors.surface2 : colors.surface1}`,
      link: `color: ${colors.blue} !important`,
      separator: `color: ${colors.overlay0}`,
      hover: themeName === 'dark' ? colors.surface1 : themes.light.surface0,
      text: `color: ${colors.text}`,
      muted: `color: ${colors.subtext0}`,
      directory: `color: ${colors.blue}; font-weight: 500`,
      loading: `color: ${colors.subtext0}`,
      error: `background-color: ${themeName === 'dark' ? colors.surface1 : themes.light.surface0}; color: ${colors.red}; border-color: ${themeName === 'dark' ? colors.surface2 : themes.light.surface1}`,
      divider: `background-color: ${colors.surface1}`,
      button: `color: ${colors.text}`,
    },
  }
}