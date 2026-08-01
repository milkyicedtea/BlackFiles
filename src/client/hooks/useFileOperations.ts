import { api } from '@local/hooks/api'
import { queryKeys } from '@local/hooks/queryKeys'
import { useQueryClient } from '@tanstack/react-query'
import { useCallback } from 'react'

export function useFileOperations() {
  const queryClient = useQueryClient()

  const deleteFile = useCallback(
    async (filePath: string) => {
      await api.delete(`/files/${filePath}`, { _successMessage: 'Deleted' })
      queryClient.invalidateQueries({ queryKey: queryKeys.directory.all })
    },
    [queryClient]
  )

  const createFolder = useCallback(
    async (parentPath: string, name: string) => {
      await api.post(
        '/folders',
        { parent_path: parentPath, name },
        { _successMessage: 'Folder created' }
      )
      queryClient.invalidateQueries({ queryKey: queryKeys.directory.all })
    },
    [queryClient]
  )

  const renameEntry = useCallback(
    async (filePath: string, newName: string) => {
      await api.put(
        '/rename',
        { path: filePath, new_name: newName },
        { _successMessage: 'Renamed' }
      )
      queryClient.invalidateQueries({ queryKey: queryKeys.directory.all })
    },
    [queryClient]
  )

  return { deleteFile, createFolder, renameEntry }
}
