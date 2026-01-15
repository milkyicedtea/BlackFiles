import {getThemeColors, latte, type Theme} from "./theme"

export function isValidTheme(value: any): value is Theme {
  return value === 'light' || value === 'dark'
}

export function createThemeStyles(theme: Theme) {
  const colors = getThemeColors(theme)

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
      container: `background-color: ${theme === 'dark' ? colors.surface0 : colors.mantle}; border-color: ${theme === 'dark' ? colors.surface2 : colors.surface1}`,
      link: `color: ${colors.blue} !important`,
      separator: `color: ${colors.overlay0}`,
      hover: theme === 'dark' ? colors.surface1 : latte.surface0,
      text: `color: ${colors.text}`,
      muted: `color: ${colors.subtext0}`,
      directory: `color: ${colors.blue}; font-weight: 500`,
      loading: `color: ${colors.subtext0}`,
      error: `background-color: ${theme === 'dark' ? colors.surface1 : latte.surface0}; color: ${colors.red}; border-color: ${theme === 'dark' ? colors.surface2 : latte.surface1}`,
      divider: `background-color: ${colors.surface1}`,
      button: `color: ${colors.text}`,
    }
  }
}

export type ThemeResult = ReturnType<typeof createThemeStyles>
export type ThemeClasses = ThemeResult['classes']
export type ThemeStyles = ThemeResult['styles']