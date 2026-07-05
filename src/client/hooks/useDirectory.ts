import { api } from '@local/hooks/api'
import { queryKeys } from '@local/hooks/queryKeys'
import type { DirectoryListResponse } from '@local/types/api'
import type { FileEntry } from '@local/types/auth'
import { useQuery } from '@tanstack/react-query'
import { useNavigate, useSearch } from '@tanstack/react-router'
import type { DataTableSortStatus } from 'mantine-datatable'
import { useCallback, useMemo, useState } from 'react'

interface UseDirectoryReturn {
  files: Array<FileEntry>
  loading: boolean
  error: string | null
  sortedRecords: Array<FileEntry>
  sortStatus: DataTableSortStatus<FileEntry>
  onSortStatusChange: (status: DataTableSortStatus<FileEntry>) => void
  currentPath: string
  pathParts: Array<string>
  pathInput: string
  setPathInput: (v: string) => void
  editingPath: boolean
  setEditingPath: (v: boolean) => void
  navigateToDir: (dirPath: string) => void
  loadDirectory: (dirPath: string) => Promise<void>
  search: string
  setSearch: (v: string) => void
  page: number
  setPage: (v: number) => void
  limit: number
  setLimit: (v: number) => void
  total: number
}

export function useDirectory(): UseDirectoryReturn {
  const { path } = useSearch({ from: '/browse' })
  const navigate = useNavigate()
  const [sortStatus, setSortStatus] = useState<DataTableSortStatus<FileEntry>>({
    columnAccessor: 'name',
    direction: 'asc',
  })
  const [pathInput, setPathInput] = useState('')
  const [editingPath, setEditingPath] = useState(false)

  // Pagination & Search
  const [search, setSearch] = useState('')
  const [page, setPage] = useState(1)
  const [limit, setLimit] = useState(25)

  const currentPath = path || ''
  const pathParts = currentPath ? currentPath.split('/').filter(Boolean) : []

  const { data, isLoading, error, refetch } = useQuery({
    queryKey: queryKeys.directory.list({
      path: currentPath,
      limit,
      offset: (page - 1) * limit,
      search: search || undefined,
    }),
    queryFn: async () => {
      const params = {
        limit,
        offset: (page - 1) * limit,
        ...(search ? { search } : {}),
      }

      const url = currentPath ? `/list/${currentPath}` : '/list'
      const { data: directoryList } = await api.get<DirectoryListResponse>(url, { params })
      return {
        files: directoryList.data,
        total: directoryList.total,
      }
    },
  })

  const files = data?.files ?? []
  const total = data?.total ?? 0

  const loadDirectory = useCallback(
    async (_dirPath: string) => {
      await refetch()
    },
    [refetch]
  )

  function navigateToDir(dirPath: string) {
    navigate({ to: '/browse', search: { path: dirPath } })
  }

  const sortedRecords = useMemo(() => {
    const sorted = [...files]
    const { columnAccessor, direction } = sortStatus
    const dir = direction === 'asc' ? 1 : -1

    sorted.sort((a, b) => {
      if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1

      let cmp = 0
      if (columnAccessor === 'name') cmp = a.name.localeCompare(b.name)
      else if (columnAccessor === 'size') cmp = a.size - b.size
      else if (columnAccessor === 'modified') cmp = a.modified - b.modified

      return cmp * dir
    })

    return sorted
  }, [files, sortStatus])

  return {
    files,
    loading: isLoading,
    error: error?.message ?? null,
    sortedRecords,
    sortStatus,
    onSortStatusChange: setSortStatus,
    currentPath,
    pathParts,
    pathInput,
    setPathInput,
    editingPath,
    setEditingPath,
    navigateToDir,
    loadDirectory,
    search,
    setSearch,
    page,
    setPage,
    limit,
    setLimit,
    total,
  }
}
