import { BrowsePathBar } from '@local/components/BrowsePathBar'
import { FileIcon } from '@local/components/FileIcon'
import { ProtectedPage } from '@local/components/ProtectedPage'
import { usePermission } from '@local/hooks/authContext'
import { useUploader } from '@local/hooks/UploadContext'
import { useDirectory } from '@local/hooks/useDirectory'
import { useFileOperations } from '@local/hooks/useFileOperations'
import { formatDate, formatSize } from '@local/lib/format'
import type { FileEntry } from '@local/types/auth'
import { ActionIcon, Button, Group, Stack, Text, TextInput, Tooltip } from '@mantine/core'
import { modals } from '@mantine/modals'
import { IconDownload, IconEdit, IconFolderPlus, IconTrash, IconUpload } from '@tabler/icons-react'
import { createFileRoute } from '@tanstack/react-router'
import type { DataTableColumn } from 'mantine-datatable'
import { DataTable } from 'mantine-datatable'
import { type ChangeEvent, type DragEvent, useCallback, useRef, useState } from 'react'

interface BrowseSearch {
  path?: string
}

export const Route = createFileRoute('/browse')({
  validateSearch: (search: Record<string, unknown>): BrowseSearch => ({
    path: typeof search.path === 'string' ? search.path : undefined,
  }),
  component: () => (
    <ProtectedPage>
      <BrowsePage />
    </ProtectedPage>
  ),
})

const PREVIEWABLE_EXTENSIONS = new Set([
  'png',
  'jpg',
  'jpeg',
  'gif',
  'webp',
  'svg',
  'ico',
  'txt',
  'md',
  'json',
  'xml',
  'html',
  'css',
  'js',
  'ts',
  'pdf',
])

function handleFileDownload(file: FileEntry) {
  const a = document.createElement('a')
  a.href = `/api/files/${file.path}`
  a.rel = 'noopener'
  a.click()
}

function handleFileOpen(file: FileEntry, navigateToDir: (path: string) => void) {
  if (file.is_dir) {
    navigateToDir(file.path)
    return
  }

  const ext = file.name.includes('.') ? file.name.split('.').pop()?.toLowerCase() || '' : ''

  if (PREVIEWABLE_EXTENSIONS.has(ext)) {
    window.open(`/api/files/${file.path}`, '_blank')
  } else {
    handleFileDownload(file)
  }
}

function BrowsePage() {
  const canUpload = usePermission('upload_files')
  const canDelete = usePermission('delete_files')
  const fileInputRef = useRef<HTMLInputElement>(null)
  const {
    loading,
    error,
    sortedRecords,
    sortStatus,
    onSortStatusChange,
    currentPath,
    pathParts,
    navigateToDir,
    search,
    setSearch,
    page,
    setPage,
    limit,
    total,
    setLimit,
  } = useDirectory()

  const { deleteFile, createFolder, renameEntry } = useFileOperations()
  const { addFiles } = useUploader()
  const canRename = usePermission('rename_files')
  const canCreateFolder = usePermission('create_folders')

  const [editingPath, setEditingPath] = useState<string | null>(null)
  const [editingName, setEditingName] = useState('')

  const [creatingFolder, setCreatingFolder] = useState(false)
  const [newFolderName, setNewFolderName] = useState('')

  const [isDragOver, setIsDragOver] = useState(false)
  const dragCounter = useRef(0)

  function goUp() {
    if (pathParts.length === 0) return
    const parent = pathParts.slice(0, -1).join('/')
    navigateToDir(parent)
  }

  function handleFileSelect(e: ChangeEvent<HTMLInputElement>) {
    const files = e.target.files
    if (!files || files.length === 0) return
    addFiles(files, currentPath)
    e.target.value = ''
  }

  function confirmDelete(file: FileEntry) {
    modals.openConfirmModal({
      title: file.is_dir ? 'Delete directory' : 'Delete file',
      children: (
        <Text size="sm">
          Are you sure you want to delete <strong>{file.name}</strong>?
          {file.is_dir && ' This will delete all contents inside.'}
        </Text>
      ),
      labels: { confirm: 'Delete', cancel: 'Cancel' },
      confirmProps: { color: 'red' },
      onConfirm: () => deleteFile(file.path),
    })
  }

  const startRename = useCallback((file: FileEntry) => {
    setEditingPath(file.path)
    setEditingName(file.name)
  }, [])

  const submitRename = useCallback(() => {
    if (!editingPath) return
    const trimmed = editingName.trim()
    if (trimmed && trimmed !== editingPath.split('/').pop()) {
      renameEntry(editingPath, trimmed)
    }
    setEditingPath(null)
    setEditingName('')
  }, [editingPath, editingName, renameEntry])

  const cancelRename = useCallback(() => {
    setEditingPath(null)
    setEditingName('')
  }, [])

  const startCreateFolder = useCallback(() => {
    setCreatingFolder(true)
    setNewFolderName('')
  }, [])

  const submitCreateFolder = useCallback(() => {
    const trimmed = newFolderName.trim()
    if (trimmed) createFolder(currentPath, trimmed)
    setCreatingFolder(false)
    setNewFolderName('')
  }, [newFolderName, currentPath, createFolder])

  const handleDragEnter = useCallback((e: DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    dragCounter.current++
    if (e.dataTransfer.items && e.dataTransfer.items.length > 0) setIsDragOver(true)
  }, [])

  const handleDragLeave = useCallback((e: DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    dragCounter.current--
    if (dragCounter.current === 0) setIsDragOver(false)
  }, [])

  const handleDragOver = useCallback((e: DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
  }, [])

  const handleDrop = useCallback(
    (e: DragEvent) => {
      e.preventDefault()
      e.stopPropagation()
      setIsDragOver(false)
      dragCounter.current = 0
      if (!canUpload) return
      const files = e.dataTransfer.files
      if (files.length > 0) addFiles(files, currentPath)
    },
    [canUpload, addFiles, currentPath]
  )

  const columns: Array<DataTableColumn<FileEntry>> = [
    {
      accessor: 'name',
      title: 'Name',
      render: (file) => {
        const isEditing = editingPath === file.path
        return (
          <Group gap="xs" wrap="nowrap">
            <FileIcon fileName={file.name} isDirectory={file.is_dir} />
            {isEditing ? (
              <TextInput
                size="xs"
                value={editingName}
                autoFocus
                onChange={(e) => setEditingName(e.currentTarget.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') submitRename()
                  if (e.key === 'Escape') cancelRename()
                }}
                onBlur={cancelRename}
                styles={{ input: { minWidth: 200 } }}
              />
            ) : (
              <Text
                size="sm"
                style={{
                  fontWeight: file.is_dir ? 500 : undefined,
                  maxWidth: 300,
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                }}
              >
                {file.name}
              </Text>
            )}
          </Group>
        )
      },
      sortable: true,
    },
    {
      accessor: 'size',
      title: 'Size',
      textAlign: 'right',
      render: (file) => (
        <Text size="sm" c="dimmed">
          {file.is_dir ? '\u2014' : formatSize(file.size)}
        </Text>
      ),
      sortable: true,
    },
    {
      accessor: 'modified',
      title: 'Modified',
      textAlign: 'right',
      render: (file) => (
        <Text size="sm" c="dimmed">
          {formatDate(file.modified)}
        </Text>
      ),
      sortable: true,
    },
    {
      accessor: 'actions',
      title: '',
      textAlign: 'center',
      width: 160,
      render: (file) => (
        <Group justify="center" gap={4} wrap="nowrap">
          {canRename && (
            <Tooltip label="Rename">
              <ActionIcon
                variant="subtle"
                size="sm"
                onClick={(e) => {
                  e.stopPropagation()
                  startRename(file)
                }}
              >
                <IconEdit size={16} />
              </ActionIcon>
            </Tooltip>
          )}
          {!file.is_dir && (
            <Tooltip label="Download">
              <ActionIcon
                variant="subtle"
                size="sm"
                onClick={(e) => {
                  e.stopPropagation()
                  handleFileDownload(file)
                }}
              >
                <IconDownload size={16} />
              </ActionIcon>
            </Tooltip>
          )}
          {canDelete && (
            <Tooltip label={file.is_dir ? 'Delete directory' : 'Delete file'}>
              <ActionIcon
                variant="subtle"
                color="red"
                size="sm"
                onClick={(e) => {
                  e.stopPropagation()
                  confirmDelete(file)
                }}
              >
                <IconTrash size={16} />
              </ActionIcon>
            </Tooltip>
          )}
        </Group>
      ),
    },
  ]

  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: Drag-and-drop is pointer-only; Upload button provides accessible alternative
    <div
      onDragEnter={canUpload ? handleDragEnter : undefined}
      onDragLeave={canUpload ? handleDragLeave : undefined}
      onDragOver={canUpload ? handleDragOver : undefined}
      onDrop={canUpload ? handleDrop : undefined}
      style={{
        position: 'relative',
        outline: isDragOver ? '2px dashed var(--mantine-color-blue-5)' : undefined,
        outlineOffset: 0,
        borderRadius: 'var(--mantine-radius-sm)',
      }}
    >
      {isDragOver && (
        <div
          style={{
            position: 'absolute',
            inset: 0,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            background: 'color-mix(in srgb, var(--mantine-color-blue-5) 10%, transparent)',
            borderRadius: 'var(--mantine-radius-sm)',
            backdropFilter: 'blur(10px)',
            height: '100%',
            zIndex: 10,
            pointerEvents: 'none',
          }}
        >
          <Text size="xl" fw={700} c="blue">
            Drop files to upload
          </Text>
        </div>
      )}
      <Stack gap="sm">
        <Group justify="space-between" align="flex-end">
          <BrowsePathBar
            currentPath={currentPath}
            pathParts={pathParts}
            search={search}
            onSearchChange={(v: string) => {
              setSearch(v)
              setPage(1)
            }}
            onNavigateUp={goUp}
            onNavigateToDir={navigateToDir}
          />

          <Group gap="xs">
            {canCreateFolder &&
              (creatingFolder ? (
                <TextInput
                  size="xs"
                  placeholder="Folder name"
                  value={newFolderName}
                  autoFocus
                  onChange={(e) => setNewFolderName(e.currentTarget.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') submitCreateFolder()
                    if (e.key === 'Escape') {
                      setCreatingFolder(false)
                      setNewFolderName('')
                    }
                  }}
                  onBlur={() => {
                    setCreatingFolder(false)
                    setNewFolderName('')
                  }}
                  styles={{ input: { width: 160 } }}
                />
              ) : (
                <Button
                  leftSection={<IconFolderPlus size={16} />}
                  variant="light"
                  size="sm"
                  onClick={startCreateFolder}
                >
                  New Folder
                </Button>
              ))}
            {canUpload && (
              <>
                <input ref={fileInputRef} type="file" multiple hidden onChange={handleFileSelect} />
                <Button
                  leftSection={<IconUpload size={16} />}
                  variant="light"
                  size="sm"
                  onClick={() => fileInputRef.current?.click()}
                >
                  Upload
                </Button>
              </>
            )}
          </Group>
        </Group>

        <DataTable<FileEntry>
          idAccessor="path"
          withTableBorder
          withColumnBorders
          borderRadius="sm"
          highlightOnHover
          verticalSpacing="sm"
          horizontalSpacing="md"
          minHeight="10rem"
          fetching={loading}
          columns={columns}
          records={sortedRecords}
          onRowClick={({ record }) => handleFileOpen(record, navigateToDir)}
          noRecordsText={error || (loading ? 'Loading\u2026' : 'This folder is empty')}
          sortStatus={sortStatus}
          onSortStatusChange={onSortStatusChange}
          page={page}
          onPageChange={setPage}
          totalRecords={total}
          recordsPerPageOptions={[10, 25, 50]}
          recordsPerPage={limit}
          onRecordsPerPageChange={(v: number) => {
            setLimit(v)
            setPage(1)
          }}
        />
      </Stack>
    </div>
  )
}
