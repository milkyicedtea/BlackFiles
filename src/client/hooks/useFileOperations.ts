import { api } from '@local/hooks/api'
import { queryKeys } from '@local/hooks/queryKeys'
import { useQueryClient } from '@tanstack/react-query'
import { useCallback } from 'react'

export function useFileOperations() {
  const queryClient = useQueryClient()

  const deleteFile = useCallback(
    async (filePath: string) => {
      await api.delete(`/files/${filePath}`, { _successMessage: 'File deleted' })
      queryClient.invalidateQueries({ queryKey: queryKeys.directory.all })
    },
    [queryClient]
  )

  return { deleteFile }
}
