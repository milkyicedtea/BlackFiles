import { ProtectedPage } from '@local/components/ProtectedPage'
import { useUploadLinks } from '@local/hooks/useUploadLinks'
import type { UploadLink } from '@local/types/auth'
import {
  ActionIcon,
  Alert,
  Badge,
  Button,
  Group,
  Paper,
  Stack,
  Text,
  TextInput,
  Title,
  Tooltip,
} from '@mantine/core'
import { modals } from '@mantine/modals'
import { IconCopy, IconPlus, IconTrash } from '@tabler/icons-react'
import { createFileRoute } from '@tanstack/react-router'
import type { DataTableColumn } from 'mantine-datatable'
import { DataTable } from 'mantine-datatable'
import { useState } from 'react'

export const Route = createFileRoute('/upload-links')({
  component: () => (
    <ProtectedPage>
      <UploadLinksPage />
    </ProtectedPage>
  ),
})

function UploadLinksPage() {
  const { links, loading, canCreate, canView, createLink, deleteLink, creating, deletingId } =
    useUploadLinks()
  const [targetPath, setTargetPath] = useState('')
  const [createdUrl, setCreatedUrl] = useState<string | null>(null)

  async function handleCreate() {
    const created = await createLink(targetPath)
    const url = `${window.location.origin}/upload/${created.token}`
    setCreatedUrl(url)
    setTargetPath('')
    await navigator.clipboard?.writeText(url).catch(() => undefined)
  }

  function copyLink() {
    if (!createdUrl) return
    void navigator.clipboard?.writeText(createdUrl)
  }

  function confirmDelete(link: UploadLink) {
    modals.openConfirmModal({
      title: 'Delete upload link',
      children: (
        <Text size="sm">
          Delete one-time link for <strong>{link.target_path || '/'}</strong>?
        </Text>
      ),
      labels: { confirm: 'Delete', cancel: 'Cancel' },
      confirmProps: { color: 'red' },
      onConfirm: () => deleteLink(link.id),
    })
  }

  const columns: Array<DataTableColumn<UploadLink>> = [
    {
      accessor: 'target_path',
      title: 'Destination',
      render: (link) => <Text size="sm">{link.target_path || '/'}</Text>,
    },
    {
      accessor: 'created_by_username',
      title: 'Created by',
      render: (link) => <Text size="sm">{link.created_by_username}</Text>,
    },
    {
      accessor: 'used_at',
      title: 'Status',
      render: (link) => (
        <Badge color={link.used_at ? 'gray' : 'green'} variant="light">
          {link.used_at ? 'Used' : 'Ready'}
        </Badge>
      ),
    },
    {
      accessor: 'created_at',
      title: 'Created',
      render: (link) => (
        <Text size="sm" c="dimmed">
          {new Date(link.created_at).toLocaleString()}
        </Text>
      ),
    },
    {
      accessor: 'actions',
      title: '',
      textAlign: 'center',
      width: 56,
      render: (link) =>
        link.can_delete ? (
          <Tooltip label="Delete upload link">
            <ActionIcon
              variant="subtle"
              color="red"
              loading={deletingId === link.id}
              onClick={() => confirmDelete(link)}
              aria-label="Delete upload link"
            >
              <IconTrash size={16} />
            </ActionIcon>
          </Tooltip>
        ) : null,
    },
  ]

  if (!canCreate && !canView) {
    return (
      <Alert color="red" title="Access denied">
        You do not have permission to manage upload links.
      </Alert>
    )
  }

  return (
    <Stack gap="lg">
      <div>
        <Title order={2}>Upload links</Title>
        <Text c="dimmed" size="sm" mt={4}>
          Give someone one file upload to selected destination folder.
        </Text>
      </div>

      {canCreate && (
        <Paper withBorder p="md">
          <Stack gap="sm">
            <Text fw={500}>Create one-time link</Text>
            <Group align="flex-end">
              <TextInput
                label="Destination folder"
                placeholder="incoming/client-a"
                value={targetPath}
                onChange={(event) => setTargetPath(event.currentTarget.value)}
                description="Leave blank for storage root. Link accepts one file."
                style={{ flex: 1 }}
              />
              <Button
                leftSection={<IconPlus size={16} />}
                loading={creating}
                onClick={handleCreate}
              >
                Create link
              </Button>
            </Group>
          </Stack>
        </Paper>
      )}

      {createdUrl && (
        <Paper withBorder p="md">
          <Stack gap="xs">
            <Text fw={500}>Link copied</Text>
            <Group wrap="nowrap">
              <TextInput value={createdUrl} readOnly style={{ flex: 1 }} aria-label="Upload link" />
              <Tooltip label="Copy link">
                <ActionIcon variant="default" onClick={copyLink} aria-label="Copy upload link">
                  <IconCopy size={16} />
                </ActionIcon>
              </Tooltip>
            </Group>
          </Stack>
        </Paper>
      )}

      <DataTable<UploadLink>
        withTableBorder
        withColumnBorders
        borderRadius="sm"
        highlightOnHover
        verticalSpacing="sm"
        horizontalSpacing="md"
        minHeight="10rem"
        fetching={loading}
        columns={columns}
        records={links}
        noRecordsText={loading ? 'Loading…' : 'No upload links yet'}
      />
    </Stack>
  )
}
