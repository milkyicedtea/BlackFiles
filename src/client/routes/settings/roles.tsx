import { ProtectedPage } from '@local/components/ProtectedPage'
import { openRoleFormModal } from '@local/components/RoleFormModal'
import { useRoles } from '@local/hooks/useRoles'
import type { RoleWithPermissions } from '@local/types/auth'
import {
  ActionIcon,
  Alert,
  Badge,
  Button,
  Group,
  Text,
  TextInput,
  Title,
  Tooltip,
} from '@mantine/core'
import { modals } from '@mantine/modals'
import { IconAlertCircle, IconEdit, IconPlus, IconTrash } from '@tabler/icons-react'
import { createFileRoute } from '@tanstack/react-router'
import type { DataTableColumn } from 'mantine-datatable'
import { DataTable } from 'mantine-datatable'

export const Route = createFileRoute('/settings/roles')({
  component: () => (
    <ProtectedPage requireAdmin>
      <RolesPage />
    </ProtectedPage>
  ),
})

function RolesPage() {
  const {
    roles,
    permissions,
    loading,

    handleSave,
    handleDelete,
    canManage,

    nameFilter,
    setNameFilter,
    displayNameFilter,
    setDisplayNameFilter,
    page,
    setPage,
    limit,
    setLimit,
    total,
  } = useRoles()

  const columns: Array<DataTableColumn<RoleWithPermissions>> = [
    {
      accessor: 'name',
      title: 'Role',
      render: (r) => (
        <Badge color={r.color || 'gray'} variant="light">
          {r.name}
        </Badge>
      ),
      filter: (
        <TextInput
          placeholder="Search name..."
          value={nameFilter}
          onChange={(e) => setNameFilter(e.currentTarget.value)}
        />
      ),
      filtering: nameFilter !== '',
    },
    {
      accessor: 'display_name',
      title: 'Display Name',
      filter: (
        <TextInput
          placeholder="Search display name..."
          value={displayNameFilter}
          onChange={(e) => setDisplayNameFilter(e.currentTarget.value)}
        />
      ),
      filtering: displayNameFilter !== '',
    },
    {
      accessor: 'hierarchy',
      title: 'Hierarchy',
      textAlign: 'center',
    },
    {
      accessor: 'permissions',
      title: 'Permissions',
      textAlign: 'center',
      render: (r) => (
        <Text size="sm" c="dimmed">
          {r.permissions.length} permissions
        </Text>
      ),
    },
    {
      accessor: 'actions',
      title: '',
      width: 100,
      render: (r) => {
        if (r.name === 'admin') return null
        return (
          <Group justify="center" gap={4} wrap="nowrap">
            {canManage && (
              <Tooltip label="Edit role">
                <ActionIcon
                  variant="subtle"
                  size="sm"
                  onClick={() =>
                    openRoleFormModal(permissions, (values) => handleSave(values, r.id), {
                      name: r.name,
                      display_name: r.display_name,
                      hierarchy: r.hierarchy,
                      color: r.color,
                      permissionNames: [...r.permissions],
                    })
                  }
                >
                  <IconEdit size={15} />
                </ActionIcon>
              </Tooltip>
            )}
            {canManage && (
              <Tooltip label="Delete role">
                <ActionIcon
                  variant="subtle"
                  color="red"
                  size="sm"
                  onClick={() => {
                    modals.openConfirmModal({
                      title: 'Delete Role',
                      children: (
                        <Text size="sm">
                          Are you sure you want to delete <strong>{r.display_name}</strong>? This
                          cannot be undone.
                        </Text>
                      ),
                      labels: { confirm: 'Delete', cancel: 'Cancel' },
                      confirmProps: { color: 'red' },
                      onConfirm: () => handleDelete(r.id),
                    })
                  }}
                >
                  <IconTrash size={15} />
                </ActionIcon>
              </Tooltip>
            )}
          </Group>
        )
      },
    },
  ]

  return (
    <div>
      <Group justify="space-between" mb="md">
        <Title order={4}>Role Management</Title>
        {canManage && (
          <Button
            leftSection={<IconPlus size={16} />}
            variant="light"
            size="sm"
            onClick={() => openRoleFormModal(permissions, (values) => handleSave(values))}
          >
            New Role
          </Button>
        )}
      </Group>

      {!canManage && (
        <Alert icon={<IconAlertCircle size={16} />} color="red" mb="md">
          You don't have permission to manage roles.
        </Alert>
      )}

      <DataTable<RoleWithPermissions>
        withTableBorder
        withColumnBorders
        borderRadius="sm"
        highlightOnHover
        verticalSpacing="sm"
        horizontalSpacing="md"
        fetching={loading}
        columns={columns}
        records={roles}
        noRecordsText="No roles found"
        page={page}
        onPageChange={setPage}
        totalRecords={total}
        recordsPerPage={limit}
        recordsPerPageOptions={[10, 25, 50]}
        onRecordsPerPageChange={(v) => {
          setLimit(v)
          setPage(1)
        }}
      />
    </div>
  )
}
