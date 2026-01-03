export type Theme = 'light' | 'dark'

export const latte = {
  base: '#eff1f5',
  mantle: '#e6e9ef',
  surface0: '#ccd0da',
  surface1: '#bcc0cc',
  surface2: '#acb0be',
  overlay0: '#9ca0b0',
  overlay1: '#8c8fa1',
  text: '#4c4f69',
  subtext0: '#6c6f85',
  subtext1: '#5c5f77',
  blue: '#1e66f5',
  red: '#d20f39',
}

export const macchiato = {
  base: '#24273a',
  mantle: '#1e2030',
  surface0: '#363a4f',
  surface1: '#494d64',
  surface2: '#5b6078',
  overlay0: '#6e738d',
  overlay1: '#8087a2',
  text: '#cad3f5',
  subtext0: '#a5adcb',
  subtext1: '#b8c0e0',
  blue: '#8aadf4',
  red: '#ed8796',
}

// Helper function to get the current theme colors
export function getThemeColors(theme: Theme) {
  return theme === 'dark' ? macchiato : latte
}