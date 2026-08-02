import { ProtectedPage } from '@local/components/ProtectedPage'
import { api } from '@local/hooks/api'
import { useAuth } from '@local/hooks/authContext'
import type { PasswordFormValues } from '@local/types/settings'
import {
  Button,
  Paper,
  PasswordInput,
  SimpleGrid,
  Stack,
  Text,
  TextInput,
  Title,
} from '@mantine/core'
import { useForm } from '@mantine/form'
import { createFileRoute } from '@tanstack/react-router'
import { useState } from 'react'

export const Route = createFileRoute('/settings/general')({
  component: () => (
    <ProtectedPage>
      <GeneralSettingsPage />
    </ProtectedPage>
  ),
})

function GeneralSettingsPage() {
  const { user } = useAuth()
  const [saving, setSaving] = useState(false)
  const form = useForm<PasswordFormValues>({
    initialValues: { password: '', confirmPassword: '' },
    validateInputOnBlur: true,
    validate: {
      password: (value) => (value.length < 4 ? 'Password must be at least 4 characters' : null),
      confirmPassword: (value, values) =>
        value !== values.password ? 'Passwords do not match' : null,
    },
  })

  if (!user) return null
  const userId = user.id

  async function handlePasswordChange(values: PasswordFormValues) {
    setSaving(true)
    try {
      await api.put<void>(
        `/users/${userId}/password`,
        { password: values.password },
        { _successMessage: 'Password changed' }
      )
      form.reset()
    } finally {
      setSaving(false)
    }
  }

  return (
    <Stack gap="lg">
      <Title order={3}>General</Title>

      <Paper withBorder p="lg">
        <Title order={4} mb="xs">
          Profile
        </Title>
        <Text size="sm" c="dimmed" mb="md">
          Profile details are managed by your administrator.
        </Text>
        <SimpleGrid cols={{ base: 1, sm: 2 }} spacing="md">
          <TextInput label="Username" value={user.username} readOnly />
          <TextInput label="Role" value={user.role_name} readOnly />
          <TextInput
            label="Member since"
            value={new Date(user.created_at).toLocaleString()}
            readOnly
          />
          <TextInput
            label="Last updated"
            value={new Date(user.updated_at).toLocaleString()}
            readOnly
          />
        </SimpleGrid>
      </Paper>

      <Paper component="form" withBorder p="lg" onSubmit={form.onSubmit(handlePasswordChange)}>
        <Title order={4} mb="xs">
          Change password
        </Title>
        <Text size="sm" c="dimmed" mb="md">
          Use at least 4 characters. Your new password takes effect immediately.
        </Text>
        <Stack gap="md">
          <PasswordInput
            label="New password"
            autoComplete="new-password"
            required
            {...form.getInputProps('password')}
          />
          <PasswordInput
            label="Confirm password"
            autoComplete="new-password"
            required
            {...form.getInputProps('confirmPassword')}
          />
          <Button type="submit" loading={saving} style={{ alignSelf: 'flex-start' }}>
            Change password
          </Button>
        </Stack>
      </Paper>
    </Stack>
  )
}
