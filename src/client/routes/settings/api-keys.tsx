import { openApiKeyResultModal } from '@local/components/ApiKeyResultModal'
import { ProtectedPage } from '@local/components/ProtectedPage'
import { isAdmin, useAuth, usePermission } from '@local/hooks/authContext'
import { useApiKeys } from '@local/hooks/useApiKeys'
import type { ApiKey } from '@local/types/settings'
import {
  ActionIcon,
  Alert,
  Button,
  Group,
  Paper,
  Stack,
  Text,
  TextInput,
  Title,
  Tooltip,
} from '@mantine/core'
import { useForm } from '@mantine/form'
import { modals } from '@mantine/modals'
import { IconAlertCircle, IconKey, IconPlus, IconTrash } from '@tabler/icons-react'
import { createFileRoute } from '@tanstack/react-router'
import type { DataTableColumn } from 'mantine-datatable'
import { DataTable } from 'mantine-datatable'

export const Route = createFileRoute('/settings/api-keys')({
  component: () => (
    <ProtectedPage>
      <ApiKeysPage />
    </ProtectedPage>
  ),
})

const dateTimeFormatter = new Intl.DateTimeFormat(undefined, {
  dateStyle: 'medium',
  timeStyle: 'short',
})

function ApiKeysPage() {
  const { user } = useAuth()
  const hasPermission = usePermission('music_manage_api_keys')
  const canManage = isAdmin(user) || hasPermission
  const { keys, loading, error, createKey, creating, revokeKey, revokingId } = useApiKeys(canManage)
  const form = useForm({ initialValues: { label: '' } })

  async function handleCreate(values: { label: string }) {
    try {
      const label = values.label.trim()
      const createdKey = await createKey({ label: label || null })
      form.reset()
      openApiKeyResultModal(createdKey)
    } catch {
      // The API client displays the request error.
    }
  }

  function confirmRevoke(key: ApiKey) {
    const displayLabel = key.label || 'Unlabelled key'
    modals.openConfirmModal({
      title: 'Revoke API key',
      children: (
        <Text size="sm">
          Revoke <strong>{displayLabel}</strong>? Apps using this key will lose access immediately.
        </Text>
      ),
      labels: { confirm: 'Revoke key', cancel: 'Cancel' },
      confirmProps: { color: 'red' },
      onConfirm: () => revokeKey(key.id),
    })
  }

  const columns: Array<DataTableColumn<ApiKey>> = [
    {
      accessor: 'label',
      title: 'Label',
      render: (key) => key.label || <Text c="dimmed">Unlabelled</Text>,
    },
    {
      accessor: 'created_at',
      title: 'Created',
      render: (key) => dateTimeFormatter.format(new Date(key.created_at)),
    },
    {
      accessor: 'last_used_at',
      title: 'Last used',
      render: (key) =>
        key.last_used_at ? (
          dateTimeFormatter.format(new Date(key.last_used_at))
        ) : (
          <Text c="dimmed">Never</Text>
        ),
    },
    {
      accessor: 'actions',
      title: '',
      render: (key) => (
        <Tooltip label="Revoke key">
          <ActionIcon
            variant="subtle"
            color="red"
            size="sm"
            loading={revokingId === key.id}
            aria-label={`Revoke ${key.label || 'unlabelled API key'}`}
            onClick={() => confirmRevoke(key)}
          >
            <IconTrash size={15} />
          </ActionIcon>
        </Tooltip>
      ),
    },
  ]

  return (
    <Stack gap="lg">
      <div>
        <Title order={3}>API keys</Title>
        <Text size="sm" c="dimmed" mt="xs">
          Create keys for OpenSubsonic clients. Raw keys are shown only once.
        </Text>
      </div>

      {!canManage ? (
        <Alert icon={<IconAlertCircle size={18} />} color="red" title="Permission required">
          Your role does not allow API key management.
        </Alert>
      ) : (
        <>
          <Paper component="form" withBorder p="lg" onSubmit={form.onSubmit(handleCreate)}>
            <Group align="flex-end">
              <TextInput
                label="Label (optional)"
                placeholder="Phone, desktop app, or player"
                leftSection={<IconKey size={16} />}
                flex={1}
                {...form.getInputProps('label')}
              />
              <Button type="submit" loading={creating} leftSection={<IconPlus size={16} />}>
                Create key
              </Button>
            </Group>
          </Paper>

          {error && (
            <Alert icon={<IconAlertCircle size={18} />} color="red" title="Keys could not load">
              Refresh the page to try again.
            </Alert>
          )}

          <DataTable<ApiKey>
            withTableBorder
            borderRadius="sm"
            highlightOnHover
            verticalSpacing="sm"
            horizontalSpacing="md"
            fetching={loading}
            records={keys}
            columns={columns}
            noRecordsText="No API keys yet. Create one to connect an OpenSubsonic app."
          />
        </>
      )}
    </Stack>
  )
}
