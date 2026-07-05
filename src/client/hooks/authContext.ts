import type { User } from '@local/types/auth'
import { useRouterState } from '@tanstack/react-router'

export interface AuthState {
  user: User | null
  loading: boolean
}

export const defaultAuthState: AuthState = {
  user: null,
  loading: true,
}

/**
 * Reads the current auth state from the router's context.
 * The root route's beforeLoad fetches the user and returns { auth }.
 */
export function useAuth(): AuthState {
  return useRouterState({
    select: (s) => {
      const ctx = s.matches[0]?.context as { auth?: AuthState } | undefined
      return ctx?.auth ?? defaultAuthState
    },
  })
}

export function isAdmin(user: User | null): boolean {
  return user?.role_name === 'admin'
}

export function usePermission(permission: string): boolean {
  const { user } = useAuth()
  if (!user?.permissions) return false
  return user.permissions.includes(permission)
}
