import { BrowsePathBar } from '@local/components/BrowsePathBar'
import { FileIcon } from '@local/components/FileIcon'
import { usePermission } from '@local/hooks/authContext'
import { useUploader } from '@local/hooks/UploadContext'
import { useDirectory } from '@local/hooks/useDirectory'
import { useFileOperations } from '@local/hooks/useFileOperations'
import { formatDate, formatSize } from '@local/lib/format'
import type { FileEntry } from '@local/types/auth'
import { ActionIcon, Button, Group, Stack, Text, Tooltip } from '@mantine/core'
import { modals } from '@mantine/modals'
import { IconDownload, IconTrash, IconUpload } from '@tabler/icons-react'
import { createFileRoute } from '@tanstack/react-router'
import type { DataTableColumn } from 'mantine-datatable'
import { DataTable } from 'mantine-datatable'
import { useRef } from 'react'

interface BrowseSearch {
  path?: string
}

export const Route = createFileRoute('/browse')({
  validateSearch: (search: Record<string, unknown>): BrowseSearch => ({
    path: typeof search.path === 'string' ? search.path : undefined,
  }),
  component: BrowsePage,
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

  const { deleteFile } = useFileOperations(currentPath)
  const { addFiles } = useUploader()

  function goUp() {
    if (pathParts.length === 0) return
    const parent = pathParts.slice(0, -1).join('/')
    navigateToDir(parent)
  }

  function handleFileSelect(e: React.ChangeEvent<HTMLInputElement>) {
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

  const columns: Array<DataTableColumn<FileEntry>> = [
    {
      accessor: 'name',
      title: 'Name',
      render: (file) => (
        <Group gap="xs" wrap="nowrap">
          <FileIcon fileName={file.name} isDirectory={file.is_dir} />
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
        </Group>
      ),
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
      width: canDelete ? 120 : 60,
      render: (file) => (
        <Group justify="center" gap={4} wrap="nowrap">
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
    <div style={{ padding: '1rem' }}>
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

        <DataTable<FileEntry>
          withTableBorder
          withColumnBorders
          borderRadius="sm"
          highlightOnHover
          verticalSpacing="sm"
          horizontalSpacing="md"
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
