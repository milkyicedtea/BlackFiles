import { api } from '@local/hooks/api'
import { isAdmin, useAuth, usePermission } from '@local/hooks/authContext'
import { queryKeys } from '@local/hooks/queryKeys'
import type { CreatedUploadLink, UploadLink } from '@local/types/auth'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

export function useUploadLinks() {
  const { user } = useAuth()
  const admin = isAdmin(user)
  const hasCreatePermission = usePermission('create_upload_links')
  const hasViewPermission = usePermission('view_upload_links')
  const canCreate = admin || hasCreatePermission
  const canView = admin || hasViewPermission
  const queryClient = useQueryClient()

  const linksQuery = useQuery({
    queryKey: queryKeys.uploadLinks.all,
    enabled: canCreate || canView,
    queryFn: async () => {
      const { data } = await api.get<Array<UploadLink>>('/upload-links')
      return data
    },
  })

  const createMutation = useMutation({
    mutationFn: async (targetPath: string) => {
      const { data } = await api.post<CreatedUploadLink>(
        '/upload-links',
        { target_path: targetPath },
        { _successMessage: 'Upload link created' }
      )
      return data
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.uploadLinks.all })
    },
  })

  const deleteMutation = useMutation({
    mutationFn: (id: string) =>
      api.delete<void>(`/upload-links/${id}`, { _successMessage: 'Upload link deleted' }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.uploadLinks.all })
    },
  })

  return {
    links: linksQuery.data ?? [],
    loading: linksQuery.isLoading,
    canCreate,
    canView,
    createLink: createMutation.mutateAsync,
    deleteLink: deleteMutation.mutateAsync,
    creating: createMutation.isPending,
    deletingId: deleteMutation.isPending ? deleteMutation.variables : null,
  }
}
