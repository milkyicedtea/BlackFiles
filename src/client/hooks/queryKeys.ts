export const queryKeys = {
  users: {
    all: ['users'] as const,
    list: (params?: {
      limit?: number
      offset?: number
      search?: string
      username?: string
      role_name?: string
    }) => ['users', 'list', params] as const,
  },
  roles: {
    all: ['roles'] as const,
    list: (params?: {
      limit?: number
      offset?: number
      search?: string
      name?: string
      display_name?: string
    }) => ['roles', 'list', params] as const,
  },
  permissions: {
    all: ['permissions'] as const,
  },
  uploadLinks: {
    all: ['upload-links'] as const,
  },
  directory: {
    all: ['directory'] as const,
    list: (params: { path: string; limit?: number; offset?: number; search?: string }) =>
      ['directory', 'list', params] as const,
  },
} as const
