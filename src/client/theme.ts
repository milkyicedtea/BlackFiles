import { createTheme, rem } from '@mantine/core'

interface ThemeValues {
  base: string
  mantle: string
  surface0: string
  surface1: string
  surface2: string
  overlay0: string
  overlay1: string
  text: string
  subtext0: string
  subtext1: string
  blue: string
  red: string
}

export const latte: ThemeValues = {
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

export const macchiato: ThemeValues = {
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

export const themes = {
  light: latte,
  dark: macchiato,
}

export type ThemeName = keyof typeof themes

export const theme = createTheme({
  primaryColor: 'blue',
  defaultRadius: 'md',

  fontSizes: {
    xs: rem(13),
    sm: rem(15),
    md: rem(16),
    lg: rem(18),
    xl: rem(20),
  },

  headings: {
    fontWeight: '700',
  },

  components: {
    NavLink: {
      styles: {
        root: {
          minHeight: rem(36),
          padding: `${rem(4)} ${rem(12)}`,
          borderRadius: rem(8),
        },
        label: {
          fontSize: rem(14),
          fontWeight: 500,
        },
      },
    },
    ActionIcon: {
      defaultProps: {
        size: 'lg',
        radius: 'md',
        variant: 'light',
      },
    },
    Drawer: {
      defaultProps: {
        radius: 'md',
      },
    },
    Paper: {
      defaultProps: {
        radius: 'md',
      },
    },
    Modal: {
      defaultProps: {
        radius: 'lg',
        overlayProps: { backgroundOpacity: 0.4, blur: 3 },
      },
    },
    Badge: {
      defaultProps: {
        variant: 'light',
        radius: 'sm',
      },
    },
    Alert: {
      defaultProps: {
        radius: 'md',
        variant: 'light',
      },
    },
    Notification: {
      defaultProps: {
        radius: 'md',
      },
    },
    Button: {
      defaultProps: {
        radius: 'xl',
      },
    },
    Input: {
      defaultProps: {
        size: 'md',
      },
    },
    TextInput: {
      defaultProps: {
        size: 'md',
      },
    },
    NumberInput: {
      defaultProps: {
        size: 'md',
      },
    },
    Select: {
      defaultProps: {
        size: 'md',
      },
    },
  },
})
