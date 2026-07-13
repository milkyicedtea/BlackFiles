import { ProtectedPage } from '@local/components/ProtectedPage'
import { isAdmin, useAuth } from '@local/hooks/authContext'
import { Box, Paper, SimpleGrid, Text, Title } from '@mantine/core'
import { IconShieldLock, IconUsers } from '@tabler/icons-react'
import { createFileRoute, Link } from '@tanstack/react-router'

export const Route = createFileRoute('/settings/')({
  component: () => (
    <ProtectedPage requireAdmin>
      <SettingsPage />
    </ProtectedPage>
  ),
})

function SettingsPage() {
  const { user } = useAuth()
  const canManageUsers = isAdmin(user)
  const canManageRoles = isAdmin(user)

  return (
    <Box size="sm">
      <Title order={3} mb="lg">
        Settings
      </Title>
      <SimpleGrid cols={{ base: 1, sm: 3, lg: 4 }} spacing="md">
        {canManageUsers && (
          <Paper
            component={Link}
            to="/settings/users"
            withBorder
            p="lg"
            style={{ textDecoration: 'none', color: 'inherit', cursor: 'pointer' }}
          >
            <IconUsers size={28} stroke={1.5} />
            <Title order={5} mt="sm">
              User Management
            </Title>
            <Text size="sm" c="dimmed" mt={4}>
              Create, edit, and delete users. Change roles and passwords.
            </Text>
          </Paper>
        )}
        {canManageRoles && (
          <Paper
            component={Link}
            to="/settings/roles"
            withBorder
            p="lg"
            style={{ textDecoration: 'none', color: 'inherit', cursor: 'pointer' }}
          >
            <IconShieldLock size={28} stroke={1.5} />
            <Title order={5} mt="sm">
              Role Management
            </Title>
            <Text size="sm" c="dimmed" mt={4}>
              Define roles and assign granular permissions.
            </Text>
          </Paper>
        )}
      </SimpleGrid>
      {!canManageUsers && !canManageRoles && (
        <Text c="dimmed" ta="center" py="xl">
          You don't have permission to manage settings.
        </Text>
      )}
    </Box>
  )
}
