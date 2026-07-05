import { Button, Group, Text } from '@mantine/core'
import { modals } from '@mantine/modals'
import { useState } from 'react'

interface DeleteConfirmProps {
  username: string
  onDelete: () => Promise<void>
}

function DeleteConfirmContent({ username, onDelete }: DeleteConfirmProps) {
  const [loading, setLoading] = useState(false)

  async function handleDelete() {
    setLoading(true)
    try {
      await onDelete()
      modals.closeAll()
    } catch {
      setLoading(false)
    }
  }

  return (
    <>
      <Text mb="md">
        Are you sure you want to delete <strong>{username}</strong>? This cannot be undone.
      </Text>
      <Group justify="flex-end">
        <Button variant="default" onClick={() => modals.closeAll()}>
          Cancel
        </Button>
        <Button color="red" loading={loading} onClick={handleDelete}>
          Delete
        </Button>
      </Group>
    </>
  )
}

export function openDeleteUserModal(username: string, onDelete: () => Promise<void>) {
  modals.open({
    title: 'Confirm Deletion',
    children: <DeleteConfirmContent username={username} onDelete={onDelete} />,
  })
}
