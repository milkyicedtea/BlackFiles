import { api } from '@local/hooks/api'
import { useAuth } from '@local/hooks/authContext'
import { queryKeys } from '@local/hooks/queryKeys'
import type { PermissionsCollectionResponse, RolesListResponse } from '@local/types/api'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { useCallback, useState } from 'react'

export interface RoleFormValues {
  name: string
  display_name: string
  color: string
  permissionNames: Array<string>
}

export function useRoles() {
  const { user: currentUser, loading: authLoading } = useAuth()
  const navigate = useNavigate()
  const queryClient = useQueryClient()

  const [nameFilter, setNameFilter] = useState('')
  const [displayNameFilter, setDisplayNameFilter] = useState('')
  const [page, setPage] = useState(1)
  const [limit, setLimit] = useState(25)
  // Queries
  const filterParams = {
    limit,
    offset: (page - 1) * limit,
    name: nameFilter || undefined,
    display_name: displayNameFilter || undefined,
  }

  const rolesQuery = useQuery({
    queryKey: queryKeys.roles.list(filterParams),
    queryFn: async () => {
      const { data } = await api.get<RolesListResponse>('/roles', { params: filterParams })
      return {
        roles: data.data,
        total: data.total,
      }
    },
  })

  const permissionsQuery = useQuery({
    queryKey: queryKeys.permissions.all,
    queryFn: async () => {
      const { data } = await api.get<PermissionsCollectionResponse>('/permissions')
      return data
    },
  })

  const roles = rolesQuery.data?.roles ?? []
  const permissions = permissionsQuery.data ?? []
  const total = rolesQuery.data?.total ?? 0

  // Mutations
  const saveMutation = useMutation({
    mutationFn: async (values: RoleFormValues & { roleId?: number }) => {
      const payload = {
        display_name: values.display_name,
        color: values.color,
        permissions: values.permissionNames,
      }

      if (values.roleId) {
        await api.put<void>(`/roles/${values.roleId}`, payload, {
          _successMessage: 'Role updated',
        })
      } else {
        await api.post<void>('/roles', { name: values.name, ...payload }, {
          _successMessage: 'Role created',
        })
      }
    },
    onSuccess: () => queryClient.invalidateQueries({ queryKey: queryKeys.roles.all }),
  })

  const moveMutation = useMutation({
    mutationFn: ({ roleId, direction }: { roleId: number; direction: 'up' | 'down' }) =>
      api.post<void>(`/roles/${roleId}/move`, { direction }, { _successMessage: 'Role moved' }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: queryKeys.roles.all }),
  })

  const deleteMutation = useMutation({
    mutationFn: (roleId: number) =>
      api.delete<void>(`/roles/${roleId}`, { _successMessage: 'Role deleted' }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.roles.all })
    },
  })

  const loadData = useCallback(async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: queryKeys.roles.all }),
      queryClient.invalidateQueries({ queryKey: queryKeys.permissions.all }),
    ])
  }, [queryClient])

  async function handleSave(values: RoleFormValues, roleId?: number) {
    await saveMutation.mutateAsync({ ...values, roleId })
  }

  async function handleMove(roleId: number, direction: 'up' | 'down') {
    await moveMutation.mutateAsync({ roleId, direction })
  }

  async function handleDelete(roleId: number) {
    await deleteMutation.mutateAsync(roleId)
  }

  return {
    currentUser,
    authLoading,
    roles,
    permissions,
    loading: rolesQuery.isLoading || permissionsQuery.isLoading,
    loadData,
    navigate,

    nameFilter,
    setNameFilter,
    displayNameFilter,
    setDisplayNameFilter,
    page,
    setPage,
    limit,
    setLimit,
    total,

    // Mutation handlers
    handleSave,
    handleMove,
    handleDelete,

    // Derived
    isAdminUser: currentUser ? currentUser.role_name === 'admin' : false,
    canManage: currentUser ? currentUser.role_name === 'admin' : false,
  }
}
