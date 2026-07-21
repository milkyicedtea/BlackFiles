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
  IconPlayerPlay,
  IconPlayerStop,
  IconUpload,
  IconX,
} from '@tabler/icons-react'
import { useRef, useState } from 'react'

export function UploadPanel() {
  const { items, hasActive, hasAny, activeCount, cancel, remove, resume, clearCompleted } =
    useUploader()
  const canUpload = usePermission('upload_files')
  const resumeInputRef = useRef<HTMLInputElement>(null)
  const [resumingId, setResumingId] = useState<string | null>(null)

  function selectFileToResume(id: string) {
    setResumingId(id)
    resumeInputRef.current?.click()
  }

  function resumeSelectedFile(file: File | null) {
    if (file && resumingId) resume(resumingId, file)
    setResumingId(null)
  }

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
        <input
          ref={resumeInputRef}
          type="file"
          hidden
          onChange={(event) => {
            resumeSelectedFile(event.currentTarget.files?.[0] ?? null)
            event.currentTarget.value = ''
          }}
        />
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

              {item.status === 'resumable' && (
                <>
                  <Text size="xs" c="dimmed">
                    Ready to resume
                  </Text>
                  {item.error && (
                    <Tooltip label={item.error}>
                      <IconAlertCircle size={16} color="var(--mantine-color-red-6)" />
                    </Tooltip>
                  )}
                  <Tooltip label="Select the original file to resume">
                    <ActionIcon variant="subtle" size="sm" onClick={() => selectFileToResume(item.id)}>
                      <IconPlayerPlay size={14} />
                    </ActionIcon>
                  </Tooltip>
                  <ActionIcon variant="subtle" size="sm" onClick={() => remove(item.id)}>
                    <IconX size={14} />
                  </ActionIcon>
                </>
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
                <ActionIcon variant="subtle" size="sm" onClick={() => remove(item.id)}>
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
