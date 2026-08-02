import { isAdmin, useAuth, usePermission } from '@local/hooks/authContext'
import { type UploadItem, type UploadKind, useUploader } from '@local/hooks/UploadContext'
import {
  ActionIcon,
  Button,
  Group,
  Indicator,
  Popover,
  Progress,
  ScrollArea,
  Stack,
  Tabs,
  Text,
  Tooltip,
} from '@mantine/core'
import {
  IconAlertCircle,
  IconCircleCheck,
  IconMusic,
  IconPlayerPlay,
  IconPlayerStop,
  IconTrash,
  IconUpload,
  IconX,
} from '@tabler/icons-react'
import { useRef, useState } from 'react'

const FIVE_UPLOAD_ROWS_HEIGHT =
  'calc(var(--mantine-spacing-xl) + var(--mantine-spacing-xl) + var(--mantine-spacing-xl) + var(--mantine-spacing-xl) + var(--mantine-spacing-xl))'

interface UploadQueueProps {
  kind: UploadKind
  items: Array<UploadItem>
  onCancel: (id: string) => void
  onRemove: (id: string) => void
  onResume: (id: string) => void
  onClearAll: (kind: UploadKind) => void
}

function UploadQueue({ kind, items, onCancel, onRemove, onResume, onClearAll }: UploadQueueProps) {
  const emptyMessage =
    kind === 'file' ? 'File uploads will appear here.' : 'Audio uploads will appear here.'
  const queueLabel = kind === 'file' ? 'file' : 'music'

  return (
    <Stack gap={0}>
      {items.length === 0 ? (
        <Text size="xs" c="dimmed" ta="center" py="md">
          {emptyMessage}
        </Text>
      ) : (
        <ScrollArea.Autosize mah={FIVE_UPLOAD_ROWS_HEIGHT} type="auto" offsetScrollbars>
          <Stack gap={0}>
            {items.map((item) => (
              <Group
                key={item.id}
                gap="xs"
                px="xs"
                h="var(--mantine-spacing-xl)"
                wrap="nowrap"
                style={{
                  borderRadius: 'var(--mantine-radius-sm)',
                  background: 'var(--mantine-color-default-hover)',
                }}
              >
                <Text size="xs" truncate style={{ flex: 1, minWidth: 0 }}>
                  {item.name}
                </Text>

                {item.status === 'done' && (
                  <Tooltip label="Upload complete">
                    <IconCircleCheck
                      size={16}
                      color="var(--mantine-color-teal-6)"
                      aria-label="Upload complete"
                    />
                  </Tooltip>
                )}
                {item.status === 'error' && (
                  <Tooltip label={item.error ?? 'Upload failed'}>
                    <IconAlertCircle
                      size={16}
                      color="var(--mantine-color-red-6)"
                      aria-label="Upload failed"
                    />
                  </Tooltip>
                )}
                {item.status === 'cancelled' && (
                  <Text size="xs" c="dimmed">
                    Cancelled
                  </Text>
                )}

                {item.status === 'resumable' && kind === 'file' && (
                  <>
                    <Text size="xs" c="dimmed">
                      Ready to resume
                    </Text>
                    {item.error && (
                      <Tooltip label={item.error}>
                        <IconAlertCircle
                          size={16}
                          color="var(--mantine-color-red-6)"
                          aria-label="Resume error"
                        />
                      </Tooltip>
                    )}
                    <Tooltip label="Select the original file to resume">
                      <ActionIcon
                        variant="subtle"
                        size="sm"
                        aria-label={`Resume ${item.name}`}
                        onClick={() => onResume(item.id)}
                      >
                        <IconPlayerPlay size={14} />
                      </ActionIcon>
                    </Tooltip>
                    <ActionIcon
                      variant="subtle"
                      size="sm"
                      aria-label={`Remove ${item.name}`}
                      onClick={() => onRemove(item.id)}
                    >
                      <IconX size={14} />
                    </ActionIcon>
                  </>
                )}

                {item.status === 'uploading' && (
                  <>
                    <Progress
                      value={item.progress}
                      size="sm"
                      w="25%"
                      color="blue"
                      animated
                      aria-label={`${item.name} upload progress`}
                    />
                    <ActionIcon
                      variant="subtle"
                      size="sm"
                      color="red"
                      aria-label={`Cancel ${item.name}`}
                      onClick={() => onCancel(item.id)}
                    >
                      <IconPlayerStop size={14} />
                    </ActionIcon>
                  </>
                )}

                {(item.status === 'error' || item.status === 'cancelled') && (
                  <ActionIcon
                    variant="subtle"
                    size="sm"
                    aria-label={`Remove ${item.name}`}
                    onClick={() => onRemove(item.id)}
                  >
                    <IconX size={14} />
                  </ActionIcon>
                )}
              </Group>
            ))}
          </Stack>
        </ScrollArea.Autosize>
      )}

      <Group justify="flex-end" pt="xs">
        <Button
          variant="subtle"
          size="compact-xs"
          color="red"
          leftSection={<IconTrash size={14} />}
          disabled={items.length === 0}
          aria-label={`Clear all ${queueLabel} uploads`}
          onClick={() => onClearAll(kind)}
        >
          Clear all
        </Button>
      </Group>
    </Stack>
  )
}

export function UploadPanel() {
  const { items, hasActive, activeCount, cancel, remove, resume, clearAll } = useUploader()
  const { user } = useAuth()
  const canUploadFiles = usePermission('upload_files')
  const hasMusicUploadPermission = usePermission('music_upload')
  const canUploadMusic = isAdmin(user) || hasMusicUploadPermission
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

  if (!canUploadFiles && !canUploadMusic) return null

  const fileItems = items.filter((item) => item.kind === 'file')
  const musicItems = items.filter((item) => item.kind === 'music')

  return (
    <Popover
      width="min(20rem, calc(100vw - var(--mantine-spacing-md)))"
      position="bottom-end"
      withArrow
      shadow="md"
    >
      <Popover.Target>
        <Tooltip label="Uploads">
          <Indicator disabled={activeCount === 0} color="blue" size={10} offset={6} processing>
            <ActionIcon
              variant={hasActive ? 'filled' : 'light'}
              color={hasActive ? 'blue' : 'gray'}
              size="md"
              radius="xl"
              aria-label="Open uploads"
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
        <Tabs defaultValue={canUploadFiles ? 'file' : 'music'}>
          <Tabs.List grow>
            {canUploadFiles && (
              <Tabs.Tab value="file" leftSection={<IconUpload size={14} />}>
                Files
              </Tabs.Tab>
            )}
            {canUploadMusic && (
              <Tabs.Tab value="music" leftSection={<IconMusic size={14} />}>
                Music
              </Tabs.Tab>
            )}
          </Tabs.List>

          {canUploadFiles && (
            <Tabs.Panel value="file" pt="xs">
              <UploadQueue
                kind="file"
                items={fileItems}
                onCancel={cancel}
                onRemove={remove}
                onResume={selectFileToResume}
                onClearAll={clearAll}
              />
            </Tabs.Panel>
          )}
          {canUploadMusic && (
            <Tabs.Panel value="music" pt="xs">
              <UploadQueue
                kind="music"
                items={musicItems}
                onCancel={cancel}
                onRemove={remove}
                onResume={selectFileToResume}
                onClearAll={clearAll}
              />
            </Tabs.Panel>
          )}
        </Tabs>
      </Popover.Dropdown>
    </Popover>
  )
}
