import { ProtectedPage } from '@local/components/ProtectedPage'
import { Box, Paper, SimpleGrid, Text, Title } from '@mantine/core'
import { IconKey, IconShieldLock, IconUsers } from '@tabler/icons-react'
import { createFileRoute, Link } from '@tanstack/react-router'

export const Route = createFileRoute('/admin/')({
  component: () => (
    <ProtectedPage requireAdmin>
      <AdminPage />
    </ProtectedPage>
  ),
})

function AdminPage() {
  return (
    <Box>
      <Title order={3} mb="lg">
        Admin
      </Title>
      <SimpleGrid cols={{ base: 1, sm: 2, lg: 3 }} spacing="md">
        <Paper
          component={Link}
          to="/admin/users"
          withBorder
          p="lg"
          style={{ textDecoration: 'none', color: 'inherit' }}
        >
          <IconUsers size={28} stroke={1.5} />
          <Title order={5} mt="sm">
            Users
          </Title>
          <Text size="sm" c="dimmed" mt="xs">
            Create accounts, assign roles, and reset passwords.
          </Text>
        </Paper>

        <Paper
          component={Link}
          to="/admin/roles"
          withBorder
          p="lg"
          style={{ textDecoration: 'none', color: 'inherit' }}
        >
          <IconShieldLock size={28} stroke={1.5} />
          <Title order={5} mt="sm">
            Roles
          </Title>
          <Text size="sm" c="dimmed" mt="xs">
            Define roles and assign granular permissions.
          </Text>
        </Paper>

        <Paper
          component={Link}
          to="/admin/api-keys"
          withBorder
          p="lg"
          style={{ textDecoration: 'none', color: 'inherit' }}
        >
          <IconKey size={28} stroke={1.5} />
          <Title order={5} mt="sm">
            API keys
          </Title>
          <Text size="sm" c="dimmed" mt="xs">
            Review and revoke OpenSubsonic keys across all users.
          </Text>
        </Paper>
      </SimpleGrid>
    </Box>
  )
}
