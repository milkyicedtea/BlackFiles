import type { CreatedApiKey } from '@local/types/settings'
import { Alert, Button, CopyButton, Group, Stack, Text, TextInput } from '@mantine/core'
import { modals } from '@mantine/modals'
import { IconAlertTriangle, IconCheck, IconCopy } from '@tabler/icons-react'

const MODAL_ID = 'created-api-key'

function ApiKeyResult({ createdKey }: { createdKey: CreatedApiKey }) {
  return (
    <Stack gap="md">
      <Alert color="yellow" icon={<IconAlertTriangle size={18} />} title="Copy this key now">
        This is the only time the full key will be shown. Store it in your OpenSubsonic client or
        password manager before closing this window.
      </Alert>

      <TextInput label="API key" value={createdKey.key} readOnly />

      <CopyButton value={createdKey.key} timeout={2_000}>
        {({ copied, copy }) => (
          <Button
            variant="light"
            color={copied ? 'green' : 'blue'}
            leftSection={copied ? <IconCheck size={16} /> : <IconCopy size={16} />}
            onClick={copy}
          >
            {copied ? 'Copied' : 'Copy API key'}
          </Button>
        )}
      </CopyButton>

      <Text size="sm" c="dimmed">
        Closing this window does not revoke the key. You can revoke it later from API keys.
      </Text>

      <Group justify="flex-end">
        <Button onClick={() => modals.close(MODAL_ID)}>I saved this key</Button>
      </Group>
    </Stack>
  )
}

export function openApiKeyResultModal(createdKey: CreatedApiKey) {
  modals.open({
    modalId: MODAL_ID,
    title: 'Save your API key',
    closeOnClickOutside: false,
    closeOnEscape: false,
    withCloseButton: false,
    children: <ApiKeyResult createdKey={createdKey} />,
  })
}
