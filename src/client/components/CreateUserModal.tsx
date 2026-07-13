import type { CreateFormValues } from '@local/hooks/useUsers'
import type { RoleWithPermissions } from '@local/types/auth'
import { Button, Group, PasswordInput, Select, Stack, TextInput } from '@mantine/core'
import { useForm } from '@mantine/form'
import { modals } from '@mantine/modals'
import { useState } from 'react'

interface CreateUserFormProps {
  roles: Array<Pick<RoleWithPermissions, 'name' | 'display_name'>>
  onCreateUser: (values: CreateFormValues) => Promise<void>
}

function CreateUserForm({ roles, onCreateUser }: CreateUserFormProps) {
  const [loading, setLoading] = useState(false)
  const form = useForm<CreateFormValues>({
    initialValues: { username: '', password: '', role_name: 'viewer' },
    validate: {
      username: (v) => (!v ? 'Required' : /[A-Z]/.test(v) ? 'Lowercase letters only' : null),
      password: (v) => (!v ? 'Required' : null),
    },
  })

  async function handleSubmit(values: CreateFormValues) {
    setLoading(true)
    try {
      await onCreateUser(values)
      modals.closeAll()
    } catch {
      setLoading(false)
    }
  }

  return (
    <form onSubmit={form.onSubmit(handleSubmit)}>
      <Stack gap="sm">
        <TextInput
          label="Username"
          required
          {...form.getInputProps('username')}
          onChange={(e) => {
            const lowerValue = e.currentTarget.value.toLowerCase()
            form.setFieldValue('username', lowerValue)
          }}
        />
        <PasswordInput label="Password" required {...form.getInputProps('password')} />
        <Select
          label="Role"
          data={roles.map((r) => ({ value: r.name, label: r.display_name }))}
          {...form.getInputProps('role_name')}
        />
        <Group justify="flex-end" mt="sm">
          <Button variant="default" onClick={() => modals.closeAll()}>
            Cancel
          </Button>
          <Button type="submit" loading={loading}>
            Create
          </Button>
        </Group>
      </Stack>
    </form>
  )
}

export function openCreateUserModal(
  roles: Array<Pick<RoleWithPermissions, 'name' | 'display_name'>>,
  onCreateUser: (values: CreateFormValues) => Promise<void>
) {
  modals.open({
    title: 'New User',
    children: <CreateUserForm roles={roles} onCreateUser={onCreateUser} />,
  })
}
