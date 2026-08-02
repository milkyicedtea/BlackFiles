import { api } from '@local/hooks/api'
import { queryKeys } from '@local/hooks/queryKeys'
import type {
  AdminApiKeyListResponse,
  ApiKeyListResponse,
  CreateApiKeyRequest,
  CreatedApiKey,
} from '@local/types/settings'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

export function useApiKeys(enabled = true) {
  const queryClient = useQueryClient()
  const keysQuery = useQuery({
    queryKey: queryKeys.apiKeys.mine,
    enabled,
    queryFn: async () => {
      const { data } = await api.get<ApiKeyListResponse>('/music/api-keys')
      return data.keys
    },
  })

  const createMutation = useMutation({
    mutationFn: async (request: CreateApiKeyRequest) => {
      const { data } = await api.post<CreatedApiKey>('/music/api-keys', request, {
        _successMessage: 'API key created',
      })
      return data
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.apiKeys.mine })
    },
  })

  const revokeMutation = useMutation({
    mutationFn: (id: string) =>
      api.delete<void>(`/music/api-keys/${id}`, { _successMessage: 'API key revoked' }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.apiKeys.mine })
    },
  })

  return {
    keys: keysQuery.data ?? [],
    loading: keysQuery.isLoading,
    error: keysQuery.error,
    createKey: createMutation.mutateAsync,
    creating: createMutation.isPending,
    revokeKey: revokeMutation.mutateAsync,
    revokingId: revokeMutation.isPending ? revokeMutation.variables : null,
  }
}

export function useAdminApiKeys(enabled = true) {
  const queryClient = useQueryClient()
  const keysQuery = useQuery({
    queryKey: queryKeys.apiKeys.admin,
    enabled,
    queryFn: async () => {
      const { data } = await api.get<AdminApiKeyListResponse>('/admin/api-keys')
      return data.keys
    },
  })

  const revokeMutation = useMutation({
    mutationFn: (id: string) =>
      api.delete<void>(`/admin/api-keys/${id}`, { _successMessage: 'API key revoked' }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.apiKeys.admin })
      await queryClient.invalidateQueries({ queryKey: queryKeys.apiKeys.mine })
    },
  })

  return {
    keys: keysQuery.data ?? [],
    loading: keysQuery.isLoading,
    error: keysQuery.error,
    revokeKey: revokeMutation.mutateAsync,
    revokingId: revokeMutation.isPending ? revokeMutation.variables : null,
  }
}
