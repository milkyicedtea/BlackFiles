import type { Permission } from '@local/types/auth'
import {
  Button,
  Checkbox,
  ColorInput,
  Divider,
  Group,
  NumberInput,
  Paper,
  Stack,
  Text,
  TextInput,
} from '@mantine/core'
import { useForm } from '@mantine/form'
import { modals } from '@mantine/modals'
import { useMemo, useState } from 'react'

const PERMISSION_GROUP_NAMES: Record<string, string> = {
  files: 'Files',
  users: 'Users',
  roles: 'Roles',
}

export interface RoleFormValues {
  name: string
  display_name: string
  hierarchy: number
  color: string
  permissionNames: Array<string>
}

interface RoleFormProps {
  editingRole: boolean
  initialValues?: Partial<RoleFormValues>
  permissions: Array<Permission>
  onSave: (values: RoleFormValues) => Promise<void>
}

function RoleForm({ editingRole, initialValues, permissions, onSave }: RoleFormProps) {
  const [loading, setLoading] = useState(false)
  const form = useForm<RoleFormValues>({
    initialValues: {
      name: initialValues?.name ?? '',
      display_name: initialValues?.display_name ?? '',
      hierarchy: initialValues?.hierarchy ?? 1,
      color: initialValues?.color ?? '#868e96',
      permissionNames: initialValues?.permissionNames ?? [],
    },
    validate: {
      name: (v) => (!v ? 'Required' : null),
      display_name: (v) => (!v ? 'Required' : null),
    },
  })

  const groupedPermissions = useMemo(() => {
    return permissions.reduce<Record<string, Array<Permission>>>((acc, p) => {
      acc[p.group_name].push(p)
      return acc
    }, {})
  }, [permissions])

  function togglePermission(permName: string) {
    const current = form.values.permissionNames
    if (current.includes(permName)) {
      form.setFieldValue(
        'permissionNames',
        current.filter((p) => p !== permName)
      )
    } else {
      form.setFieldValue('permissionNames', [...current, permName])
    }
  }

  async function handleSave() {
    setLoading(true)
    try {
      await onSave(form.values)
      modals.closeAll()
    } catch {
      setLoading(false)
    }
  }

  return (
    <Stack gap="md">
      <TextInput
        label="Name"
        placeholder="e.g. custom_role"
        disabled={editingRole}
        required
        {...form.getInputProps('name')}
      />
      <TextInput
        label="Display Name"
        placeholder="e.g. Custom Role"
        required
        {...form.getInputProps('display_name')}
      />
      <Group grow>
        <NumberInput
          label="Hierarchy"
          description="Higher = more privileges (admin=100)"
          min={1}
          max={999}
          {...form.getInputProps('hierarchy')}
        />
        <ColorInput label="Color" {...form.getInputProps('color')} />
      </Group>

      <Divider label="Permissions" labelPosition="center" />

      <Stack gap="xs">
        {Object.entries(groupedPermissions).map(([groupName, perms]) => (
          <Paper key={groupName} p="sm" withBorder>
            <Text fw={500} size="sm" mb="xs">
              {PERMISSION_GROUP_NAMES[groupName] || groupName}
            </Text>
            <Group gap="sm">
              {perms.map((perm) => (
                <Checkbox
                  key={perm.name}
                  label={perm.display_name}
                  checked={form.values.permissionNames.includes(perm.name)}
                  onChange={() => togglePermission(perm.name)}
                  size="xs"
                />
              ))}
            </Group>
          </Paper>
        ))}
      </Stack>

      <Group justify="flex-end" mt="sm">
        <Button variant="default" onClick={() => modals.closeAll()}>
          Cancel
        </Button>
        <Button onClick={handleSave} loading={loading}>
          {editingRole ? 'Update' : 'Create'}
        </Button>
      </Group>
    </Stack>
  )
}

export function openRoleFormModal(
  permissions: Array<Permission>,
  onSave: (values: RoleFormValues) => Promise<void>,
  editingRole?: {
    name: string
    display_name: string
    hierarchy: number
    color: string
    permissionNames: Array<string>
  }
) {
  modals.open({
    title: editingRole ? 'Edit Role' : 'New Role',
    size: 'lg',
    children: (
      <RoleForm
        editingRole={!!editingRole}
        initialValues={editingRole}
        permissions={permissions}
        onSave={onSave}
      />
    ),
  })
}
