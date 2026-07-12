import { openCreateUserModal } from '@local/components/CreateUserModal'
import { openDeleteUserModal } from '@local/components/DeleteUserModal'
import { openPasswordModal } from '@local/components/PasswordModal'
import { ProtectedPage } from '@local/components/ProtectedPage'
import { useUsers } from '@local/hooks/useUsers'
import type { User } from '@local/types/auth'
import {
  ActionIcon,
  Alert,
  Badge,
  Button,
  Group,
  Select,
  Text,
  TextInput,
  Title,
  Tooltip,
} from '@mantine/core'
import { IconAlertCircle, IconEdit, IconKey, IconTrash, IconUserPlus } from '@tabler/icons-react'
import { createFileRoute } from '@tanstack/react-router'
import type { DataTableColumn } from 'mantine-datatable'
import { DataTable } from 'mantine-datatable'

export const Route = createFileRoute('/settings/users')({
  component: () => (
    <ProtectedPage requireAdmin>
      <UsersPage />
    </ProtectedPage>
  ),
})

function UsersPage() {
  const {
    currentUser,
    users,
    roles,
    loading,

    editingUserId,
    setEditingUserId,
    handleRoleUpdate,

    handleCreate,
    handlePasswordUpdate,
    handleDelete,

    isAdminUser,
    canCreate,
    canEdit,
    canDelete,

    usernameFilter,
    setUsernameFilter,
    roleNameFilter,
    setRoleNameFilter,
    page,
    setPage,
    limit,
    setLimit,
    total,
  } = useUsers()

  // TypeScript narrowing — root beforeLoad guarantees auth
  if (!currentUser) return null

  const columns: Array<DataTableColumn<User>> = [
    {
      accessor: 'username',
      title: 'Username',
      render: (u) => (
        <Group gap="xs">
          <Text size="sm">{u.username}</Text>
          {u.id === currentUser.id && (
            <Badge size="xs" variant="light">
              you
            </Badge>
          )}
        </Group>
      ),
      filter: (
        <TextInput
          placeholder="Search username..."
          value={usernameFilter}
          onChange={(e) => setUsernameFilter(e.currentTarget.value)}
        />
      ),
      filtering: usernameFilter !== '',
    },
    {
      accessor: 'role_name',
      title: 'Role',
      render: (u) => {
        const isEditing = editingUserId === u.id
        const role = roles.find((r) => r.name === u.role_name)
        if (isEditing) {
          return (
            <Group gap="xs">
              <Select
                defaultValue={u.role_name}
                onChange={(value) => handleRoleUpdate(u.id, value ?? u.role_name)}
                data={roles.map((r) => ({ value: r.name, label: r.display_name }))}
                size="xs"
              />
              <Button size="xs" variant="light" onClick={() => setEditingUserId(null)}>
                Cancel
              </Button>
            </Group>
          )
        }
        return (
          <Badge color={role?.color || 'gray'} variant="light" size="sm">
            {role?.display_name || u.role_name}
          </Badge>
        )
      },
      filter: (
        <TextInput
          placeholder="Search role..."
          value={roleNameFilter}
          onChange={(e) => setRoleNameFilter(e.currentTarget.value)}
        />
      ),
      filtering: roleNameFilter !== '',
    },
    {
      accessor: 'actions',
      title: '',
      width: 100,
      render: (u) => {
        if (u.id === currentUser.id || u.username === 'admin') return null
        return (
          <Group justify="center" gap={4} wrap="nowrap">
            {canEdit && (
              <>
                <Tooltip label="Change role">
                  <ActionIcon variant="subtle" size="sm" onClick={() => setEditingUserId(u.id)}>
                    <IconEdit size={15} />
                  </ActionIcon>
                </Tooltip>
                <Tooltip label="Change password">
                  <ActionIcon
                    variant="subtle"
                    size="sm"
                    onClick={() => openPasswordModal(u.username, u.id, handlePasswordUpdate)}
                  >
                    <IconKey size={15} />
                  </ActionIcon>
                </Tooltip>
              </>
            )}
            {canDelete && (
              <Tooltip label="Delete user">
                <ActionIcon
                  variant="subtle"
                  color="red"
                  size="sm"
                  onClick={() => openDeleteUserModal(u.username, () => handleDelete(u.id))}
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
        <Title order={4}>User Management</Title>
        {canCreate && (
          <Button
            leftSection={<IconUserPlus size={16} />}
            variant="light"
            size="sm"
            onClick={() => openCreateUserModal(roles, handleCreate)}
          >
            New User
          </Button>
        )}
      </Group>

      {!isAdminUser && (
        <Alert icon={<IconAlertCircle size={16} />} color="red" mb="md">
          You don't have permission to manage users.
        </Alert>
      )}

      <DataTable<User>
        withTableBorder
        withColumnBorders
        borderRadius="sm"
        highlightOnHover
        verticalSpacing="sm"
        horizontalSpacing="md"
        fetching={loading}
        columns={columns}
        records={users}
        noRecordsText="No users found"
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
