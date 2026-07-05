import { tanstackConfig } from '@tanstack/eslint-config'
import stylistic from '@stylistic/eslint-plugin'

export default [
  ...tanstackConfig,
  {
    plugins: {
      '@stylistic': stylistic
    },
    rules: {
      'node/prefer-node-protocol': 'off',
      '@stylistic/object-curly-spacing': ["error", "always"],
      '@stylistic/semi': ["error", "never"]
    },
  },
  {
    ignores: ['eslint.config.ts', 'prettier.config.ts'],
  },
]