import type { PasswordFormValues } from '@local/hooks/useUsers'
import { Button, Group, PasswordInput, Stack } from '@mantine/core'
import { useForm } from '@mantine/form'
import { modals } from '@mantine/modals'
import { useState } from 'react'

interface PasswordFormProps {
  username: string
  userId: string
  onUpdatePassword: (userId: string, password: string) => Promise<void>
}

function PasswordForm({ userId, onUpdatePassword }: PasswordFormProps) {
  const [loading, setLoading] = useState(false)
  const form = useForm<PasswordFormValues>({
    initialValues: { password: '', confirmPassword: '' },
    validate: {
      password: (v) => (v.length < 4 ? 'Min 4 characters' : null),
      confirmPassword: (v, values) => (v !== values.password ? 'Passwords do not match' : null),
    },
  })

  async function handleSubmit(values: PasswordFormValues) {
    setLoading(true)
    try {
      await onUpdatePassword(userId, values.password)
      modals.closeAll()
    } catch {
      setLoading(false)
    }
  }

  return (
    <form onSubmit={form.onSubmit(handleSubmit)}>
      <Stack gap="sm">
        <PasswordInput label="New Password" required {...form.getInputProps('password')} />
        <PasswordInput
          label="Confirm Password"
          required
          {...form.getInputProps('confirmPassword')}
        />
        <Group justify="flex-end" mt="sm">
          <Button variant="default" onClick={() => modals.closeAll()}>
            Cancel
          </Button>
          <Button type="submit" loading={loading}>
            Update
          </Button>
        </Group>
      </Stack>
    </form>
  )
}

export function openPasswordModal(
  username: string,
  userId: string,
  onUpdatePassword: (userId: string, password: string) => Promise<void>
) {
  modals.open({
    title: `Change Password \u2014 ${username}`,
    children: (
      <PasswordForm username={username} userId={userId} onUpdatePassword={onUpdatePassword} />
    ),
  })
}
