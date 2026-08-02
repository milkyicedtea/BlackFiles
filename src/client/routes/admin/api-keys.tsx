import { ProtectedPage } from '@local/components/ProtectedPage'
import { useAdminApiKeys } from '@local/hooks/useApiKeys'
import type { AdminApiKey } from '@local/types/settings'
import {
  ActionIcon,
  Alert,
  Badge,
  Button,
  Group,
  Loader,
  Paper,
  SimpleGrid,
  Stack,
  Text,
  TextInput,
  Title,
  Tooltip,
} from '@mantine/core'
import { modals } from '@mantine/modals'
import { IconAlertCircle, IconTrash } from '@tabler/icons-react'
import { createFileRoute } from '@tanstack/react-router'
import type { DataTableColumn } from 'mantine-datatable'
import { DataTable } from 'mantine-datatable'
import { useMemo, useState } from 'react'

export const Route = createFileRoute('/admin/api-keys')({
  component: () => (
    <ProtectedPage requireAdmin>
      <AdminApiKeysPage />
    </ProtectedPage>
  ),
})

const dateTimeFormatter = new Intl.DateTimeFormat(undefined, {
  dateStyle: 'medium',
  timeStyle: 'short',
})

function AdminApiKeysPage() {
  const { keys, loading, error, revokeKey, revokingId } = useAdminApiKeys()
  const [usernameFilter, setUsernameFilter] = useState('')
  const [labelFilter, setLabelFilter] = useState('')
  const [createdFilter, setCreatedFilter] = useState('')
  const [lastUsedFilter, setLastUsedFilter] = useState('')

  const groupedKeys = useMemo(() => {
    const usernameNeedle = usernameFilter.trim().toLocaleLowerCase()
    const labelNeedle = labelFilter.trim().toLocaleLowerCase()
    const createdNeedle = createdFilter.trim().toLocaleLowerCase()
    const lastUsedNeedle = lastUsedFilter.trim().toLocaleLowerCase()
    const groups = new Map<string, Array<AdminApiKey>>()

    for (const key of keys) {
      const created = dateTimeFormatter.format(new Date(key.created_at)).toLocaleLowerCase()
      const lastUsed = key.last_used_at
        ? dateTimeFormatter.format(new Date(key.last_used_at)).toLocaleLowerCase()
        : 'never'
      if (!key.username.toLocaleLowerCase().includes(usernameNeedle)) continue
      if (!(key.label ?? 'unlabelled').toLocaleLowerCase().includes(labelNeedle)) continue
      if (!created.includes(createdNeedle)) continue
      if (!lastUsed.includes(lastUsedNeedle)) continue

      const group = groups.get(key.username) ?? []
      group.push(key)
      groups.set(key.username, group)
    }

    return [...groups.entries()].sort(([left], [right]) => left.localeCompare(right))
  }, [keys, usernameFilter, labelFilter, createdFilter, lastUsedFilter])

  const hasFilters = Boolean(usernameFilter || labelFilter || createdFilter || lastUsedFilter)

  function confirmRevoke(key: AdminApiKey) {
    const displayLabel = key.label || 'Unlabelled key'
    modals.openConfirmModal({
      title: 'Revoke API key',
      children: (
        <Text size="sm">
          Revoke <strong>{displayLabel}</strong> for <strong>{key.username}</strong>? Their app will
          lose access immediately.
        </Text>
      ),
      labels: { confirm: 'Revoke key', cancel: 'Cancel' },
      confirmProps: { color: 'red' },
      onConfirm: () => revokeKey(key.id),
    })
  }

  const columns: Array<DataTableColumn<AdminApiKey>> = [
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
            aria-label={`Revoke ${key.label || 'unlabelled API key'} for ${key.username}`}
            onClick={() => confirmRevoke(key)}
          >
            <IconTrash size={15} />
          </ActionIcon>
        </Tooltip>
      ),
    },
  ]

  function clearFilters() {
    setUsernameFilter('')
    setLabelFilter('')
    setCreatedFilter('')
    setLastUsedFilter('')
  }

  return (
    <Stack gap="lg">
      <div>
        <Title order={3}>API keys</Title>
        <Text size="sm" c="dimmed" mt="xs">
          Review OpenSubsonic access by user. Raw key values are never available here.
        </Text>
      </div>

      <Paper withBorder p="md">
        <SimpleGrid cols={{ base: 1, sm: 2, lg: 4 }} spacing="md">
          <TextInput
            label="Username"
            placeholder="Filter usernames"
            value={usernameFilter}
            onChange={(event) => setUsernameFilter(event.currentTarget.value)}
          />
          <TextInput
            label="Key label"
            placeholder="Filter labels"
            value={labelFilter}
            onChange={(event) => setLabelFilter(event.currentTarget.value)}
          />
          <TextInput
            label="Created time"
            placeholder="Filter displayed time"
            value={createdFilter}
            onChange={(event) => setCreatedFilter(event.currentTarget.value)}
          />
          <TextInput
            label="Last-used time"
            placeholder="Filter displayed time or never"
            value={lastUsedFilter}
            onChange={(event) => setLastUsedFilter(event.currentTarget.value)}
          />
        </SimpleGrid>
      </Paper>

      {error && (
        <Alert icon={<IconAlertCircle size={18} />} color="red" title="Keys could not load">
          Refresh the page to try again.
        </Alert>
      )}

      {loading ? (
        <Group justify="center" py="xl">
          <Loader aria-label="Loading API keys" />
        </Group>
      ) : groupedKeys.length > 0 ? (
        groupedKeys.map(([username, userKeys]) => (
          <section key={username} aria-labelledby={`api-keys-${userKeys[0].user_id}`}>
            <Group gap="xs" mb="xs">
              <Title order={4} id={`api-keys-${userKeys[0].user_id}`}>
                {username}
              </Title>
              <Badge color="gray">
                {userKeys.length} {userKeys.length === 1 ? 'key' : 'keys'}
              </Badge>
            </Group>
            <DataTable<AdminApiKey>
              withTableBorder
              borderRadius="sm"
              highlightOnHover
              verticalSpacing="sm"
              horizontalSpacing="md"
              records={userKeys}
              columns={columns}
            />
          </section>
        ))
      ) : (
        <Paper withBorder p="xl">
          <Stack align="center" gap="sm">
            <Text c="dimmed">
              {hasFilters
                ? 'No API keys match these filters.'
                : 'No users have created an API key yet.'}
            </Text>
            {hasFilters && (
              <Button variant="default" onClick={clearFilters}>
                Clear filters
              </Button>
            )}
          </Stack>
        </Paper>
      )}
    </Stack>
  )
}
