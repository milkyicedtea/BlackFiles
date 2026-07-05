import { ActionIcon, Group, TextInput } from '@mantine/core'
import { useForm } from '@mantine/form'
import { IconArrowLeft, IconFolder, IconSearch } from '@tabler/icons-react'
import { useState } from 'react'

interface BrowsePathBarProps {
  currentPath: string
  pathParts: Array<string>
  search: string
  onSearchChange: (v: string) => void
  onNavigateUp: () => void
  onNavigateToDir: (dirPath: string) => void
}

export function BrowsePathBar({
  currentPath,
  pathParts,
  search,
  onSearchChange,
  onNavigateUp,
  onNavigateToDir,
}: BrowsePathBarProps) {
  const [focused, setFocused] = useState(false)
  const pathForm = useForm<{ value: string }>({
    initialValues: { value: `/${currentPath}` },
  })

  // Sync form value when path changes externally and input is not focused
  // This runs on every render since we can't use useEffect here without the
  // setFieldValue becoming stale. We use a key trick: setFieldValue directly.
  const displayValue = `/${currentPath}`
  if (!focused && pathForm.values.value !== displayValue) {
    pathForm.setFieldValue('value', displayValue)
  }

  function handlePathCancel() {
    setFocused(false)
    pathForm.setFieldValue('value', `/${currentPath}`)
  }

  function handlePathSubmit(values: typeof pathForm.values) {
    const trimmed = values.value.replace(/^\/+|\/+$/g, '')
    onNavigateToDir(trimmed)
    setFocused(false)
  }

  return (
    <Group gap={4} wrap="nowrap">
      <ActionIcon
        variant="subtle"
        onClick={onNavigateUp}
        size="md"
        disabled={pathParts.length === 0}
      >
        <IconArrowLeft size={18} />
      </ActionIcon>
      <form onSubmit={pathForm.onSubmit(handlePathSubmit)} style={{ flex: 1 }}>
        <TextInput
          size="xs"
          leftSection={<IconFolder size={16} style={{ color: 'var(--mantine-color-blue-6)' }} />}
          {...pathForm.getInputProps('value')}
          onFocus={() => setFocused(true)}
          onBlur={handlePathCancel}
          onKeyDown={(e) => {
            if (e.key === 'Escape') handlePathCancel()
          }}
          placeholder="path/to/folder"
          styles={{
            wrapper: { width: '100%' },
            input: {
              fontFamily: 'monospace',
              cursor: 'text',
            },
          }}
        />
      </form>
      <TextInput
        size="xs"
        placeholder="Search files..."
        leftSection={<IconSearch size={16} />}
        value={search}
        onChange={(e) => {
          onSearchChange(e.currentTarget.value)
        }}
        style={{ maxWidth: 320 }}
      />
    </Group>
  )
}
