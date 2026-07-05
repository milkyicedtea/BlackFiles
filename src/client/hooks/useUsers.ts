import { api } from '@local/hooks/api'
import { useAuth } from '@local/hooks/authContext'
import { queryKeys } from '@local/hooks/queryKeys'
import type { RolesListResponse, UserResponse, UsersListResponse } from '@local/types/api'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { useCallback, useState } from 'react'

export interface CreateFormValues {
  username: string
  password: string
  role_name: string
}

export interface PasswordFormValues {
  password: string
  confirmPassword: string
}

interface UpdateRoleVars {
  userId: string
  role: string
}

interface UpdatePasswordVars {
  userId: string
  password: string
}

export function useUsers() {
  const { user: currentUser, loading: authLoading } = useAuth()
  const navigate = useNavigate()
  const queryClient = useQueryClient()

  const [usernameFilter, setUsernameFilter] = useState('')
  const [roleNameFilter, setRoleNameFilter] = useState('')
  const [page, setPage] = useState(1)
  const [limit, setLimit] = useState(25)

  // Inline role editing state
  const [editingUserId, setEditingUserId] = useState<string | null>(null)

  // Queries
  const filterParams = {
    limit,
    offset: (page - 1) * limit,
    username: usernameFilter || undefined,
    role_name: roleNameFilter || undefined,
  }

  const usersQuery = useQuery({
    queryKey: queryKeys.users.list(filterParams),
    queryFn: async () => {
      const { data } = await api.get<UsersListResponse>('/users', { params: filterParams })
      return {
        users: data.data,
        total: data.total,
      }
    },
  })

  const rolesQuery = useQuery({
    queryKey: queryKeys.roles.all,
    queryFn: async () => {
      const { data } = await api.get<RolesListResponse>('/roles')
      return data.data
    },
  })

  const users = usersQuery.data?.users ?? []
  const roles = rolesQuery.data ?? []
  const total = usersQuery.data?.total ?? 0
  const totalPages = Math.ceil(total / (limit || 1))

  // Mutations
  const createMutation = useMutation({
    mutationFn: (values: CreateFormValues) =>
      api.post<UserResponse>('/users', values, { _successMessage: 'User created' }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.users.all })
    },
  })

  const updateRoleMutation = useMutation({
    mutationFn: ({ userId, role }: UpdateRoleVars) =>
      api.put<void>(`/users/${userId}/role`, { role }, { _successMessage: 'Role updated' }),
    onSuccess: () => {
      setEditingUserId(null)
      queryClient.invalidateQueries({ queryKey: queryKeys.users.all })
    },
  })

  const updatePasswordMutation = useMutation({
    mutationFn: ({ userId, password }: UpdatePasswordVars) =>
      api.put<void>(
        `/users/${userId}/password`,
        { password },
        { _successMessage: 'Password updated' }
      ),
  })

  const deleteMutation = useMutation({
    mutationFn: (userId: string) =>
      api.delete<void>(`/users/${userId}`, { _successMessage: 'User deleted' }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.users.all })
    },
  })

  const loadData = useCallback(async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: queryKeys.users.all }),
      queryClient.invalidateQueries({ queryKey: queryKeys.roles.all }),
    ])
  }, [queryClient])

  async function handleCreate(values: CreateFormValues) {
    await createMutation.mutateAsync(values)
  }

  async function handleRoleUpdate(userId: string, role: string) {
    await updateRoleMutation.mutateAsync({ userId, role })
  }

  async function handlePasswordUpdate(userId: string, password: string) {
    await updatePasswordMutation.mutateAsync({ userId, password })
  }

  async function handleDelete(userId: string) {
    await deleteMutation.mutateAsync(userId)
  }

  return {
    currentUser,
    authLoading,
    users,
    roles,
    loading: usersQuery.isLoading || rolesQuery.isLoading,
    loadData,
    navigate,

    // Filters
    usernameFilter,
    setUsernameFilter,
    roleNameFilter,
    setRoleNameFilter,
    page,
    setPage,
    limit,
    setLimit,
    total,
    totalPages,

    // Inline role edit
    editingUserId,
    setEditingUserId,
    handleRoleUpdate,

    // Mutation handlers
    handleCreate,
    handlePasswordUpdate,
    handleDelete,

    // Derived
    isAdminUser: currentUser ? currentUser.role_name === 'admin' : false,
    canCreate: currentUser ? currentUser.role_name === 'admin' : false,
    canEdit: currentUser ? currentUser.role_name === 'admin' : false,
    canDelete: currentUser ? currentUser.role_name === 'admin' : false,
  }
}
