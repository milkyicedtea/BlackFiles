import { LoadingScreen } from '@local/components/LoadingScreen'
import { isAdmin, useAuth } from '@local/hooks/authContext'
import { useNavigate } from '@tanstack/react-router'
import type { ReactElement } from 'react'
import { useEffect } from 'react'

interface ProtectedPageProps {
  children: ReactElement
  requireAdmin?: boolean
}

export function ProtectedPage({ children, requireAdmin = false }: ProtectedPageProps) {
  const { user, loading } = useAuth()
  const navigate = useNavigate()
  const hasAccess = !!user && (!requireAdmin || isAdmin(user))

  useEffect(() => {
    if (loading) return

    if (!user) {
      navigate({ to: '/login' })
      return
    }

    if (requireAdmin && !isAdmin(user)) {
      navigate({ to: '/browse' })
      return
    }
  }, [user, loading, requireAdmin, navigate])

  if (loading || !hasAccess) return <LoadingScreen />

  return children
}
