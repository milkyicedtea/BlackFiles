import { usePermission } from '@local/hooks/authContext'
import { useUploader } from '@local/hooks/UploadContext'
import {
  ActionIcon,
  Button,
  Group,
  Indicator,
  Popover,
  Progress,
  Stack,
  Text,
  Tooltip,
} from '@mantine/core'
import {
  IconAlertCircle,
  IconCircleCheck,
  IconPlayerStop,
  IconUpload,
  IconX,
} from '@tabler/icons-react'

export function UploadPanel() {
  const { items, hasActive, hasAny, activeCount, cancel, clearCompleted } = useUploader()
  const canUpload = usePermission('upload_files')

  if (!canUpload) return null

  return (
    <Popover width={320} position="bottom-end" withArrow shadow="md">
      <Popover.Target>
        <Tooltip label="Uploads">
          <Indicator disabled={activeCount === 0} color="blue" size={10} offset={6} processing>
            <ActionIcon
              variant={hasActive ? 'filled' : 'light'}
              color={hasActive ? 'blue' : 'gray'}
              size="md"
              radius="xl"
            >
              <IconUpload size={18} />
            </ActionIcon>
          </Indicator>
        </Tooltip>
      </Popover.Target>

      <Popover.Dropdown p="sm">
        <Stack gap={4}>
          {items.length === 0 && (
            <Text size="xs" c="dimmed" ta="center" py="sm">
              No uploads at the moment
            </Text>
          )}

          {items.map((item) => (
            <Group
              key={item.id}
              gap="sm"
              px="sm"
              py={4}
              style={{
                borderRadius: 'var(--mantine-radius-sm)',
                background: 'var(--mantine-color-default-hover)',
              }}
            >
              <Text size="xs" truncate style={{ flex: 1, minWidth: 0 }}>
                {item.name}
              </Text>

              {item.status === 'done' && (
                <IconCircleCheck size={16} color="var(--mantine-color-teal-6)" />
              )}
              {item.status === 'error' && (
                <Tooltip label={item.error ?? 'Upload failed'}>
                  <IconAlertCircle size={16} color="var(--mantine-color-red-6)" />
                </Tooltip>
              )}
              {item.status === 'cancelled' && (
                <Text size="xs" c="dimmed">
                  Cancelled
                </Text>
              )}

              {item.status === 'uploading' && (
                <>
                  <Progress value={item.progress} size="sm" w={80} color="blue" animated />
                  <ActionIcon
                    variant="subtle"
                    size="sm"
                    color="red"
                    onClick={() => cancel(item.id)}
                  >
                    <IconPlayerStop size={14} />
                  </ActionIcon>
                </>
              )}

              {(item.status === 'error' || item.status === 'cancelled') && (
                <ActionIcon variant="subtle" size="sm" onClick={() => cancel(item.id)}>
                  <IconX size={14} />
                </ActionIcon>
              )}
            </Group>
          ))}

          {hasAny && (
            <Group justify="space-between" mt={4}>
              <Button variant="subtle" size="compact-xs" color="gray" onClick={clearCompleted}>
                Clear completed
              </Button>
            </Group>
          )}
        </Stack>
      </Popover.Dropdown>
    </Popover>
  )
}
