import { ProtectedPage } from '@local/components/ProtectedPage'
import { isAdmin, useAuth, usePermission } from '@local/hooks/authContext'
import { Badge, Box, Group, Paper, SimpleGrid, Text, Title } from '@mantine/core'
import { IconKey, IconUser } from '@tabler/icons-react'
import { createFileRoute, Link } from '@tanstack/react-router'

export const Route = createFileRoute('/settings/')({
  component: () => (
    <ProtectedPage>
      <SettingsPage />
    </ProtectedPage>
  ),
})

function SettingsPage() {
  const { user } = useAuth()
  const hasApiKeyPermission = usePermission('music_manage_api_keys')
  const canManageApiKeys = isAdmin(user) || hasApiKeyPermission

  return (
    <Box>
      <Title order={3} mb="lg">
        Settings
      </Title>
      <SimpleGrid cols={{ base: 1, sm: 2 }} spacing="md">
        <Paper
          component={Link}
          to="/settings/general"
          withBorder
          p="lg"
          style={{ textDecoration: 'none', color: 'inherit' }}
        >
          <IconUser size={28} stroke={1.5} />
          <Title order={5} mt="sm">
            General
          </Title>
          <Text size="sm" c="dimmed" mt="xs">
            Review your profile and change your password.
          </Text>
        </Paper>

        {canManageApiKeys ? (
          <Paper
            component={Link}
            to="/settings/api-keys"
            withBorder
            p="lg"
            style={{ textDecoration: 'none', color: 'inherit' }}
          >
            <IconKey size={28} stroke={1.5} />
            <Title order={5} mt="sm">
              API keys
            </Title>
            <Text size="sm" c="dimmed" mt="xs">
              Connect OpenSubsonic apps without sharing your password.
            </Text>
          </Paper>
        ) : (
          <Paper withBorder p="lg">
            <Group justify="space-between" align="flex-start">
              <IconKey size={28} stroke={1.5} />
              <Badge color="gray">Permission required</Badge>
            </Group>
            <Title order={5} mt="sm">
              API keys
            </Title>
            <Text size="sm" c="dimmed" mt="xs">
              Your role does not allow OpenSubsonic API key management.
            </Text>
          </Paper>
        )}
      </SimpleGrid>
    </Box>
  )
}
