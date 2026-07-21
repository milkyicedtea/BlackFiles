import { Alert, Button, FileInput, Paper, Stack, Text, Title } from '@mantine/core'
import { IconUpload } from '@tabler/icons-react'
import { createFileRoute } from '@tanstack/react-router'
import { useEffect, useState } from 'react'

interface PublicUploadLinkStatus {
  ready: boolean
}

export const Route = createFileRoute('/upload/$token')({
  component: PublicUploadPage,
})

function PublicUploadPage() {
  const { token } = Route.useParams()
  const [valid, setValid] = useState<boolean | null>(null)
  const [file, setFile] = useState<File | null>(null)
  const [uploading, setUploading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [uploaded, setUploaded] = useState(false)

  useEffect(() => {
    let active = true

    async function validateLink() {
      try {
        const response = await fetch(`/api/public/upload-links/${encodeURIComponent(token)}`)
        if (!response.ok) throw new Error()
        const status = (await response.json()) as PublicUploadLinkStatus
        if (active) setValid(status.ready)
      } catch {
        if (active) setValid(false)
      }
    }

    void validateLink()
    return () => {
      active = false
    }
  }, [token])

  async function uploadFile() {
    if (!file) return
    setUploading(true)
    setError(null)

    try {
      const response = await fetch(
        `/api/public/upload-links/${encodeURIComponent(token)}?filename=${encodeURIComponent(file.name)}`,
        {
          method: 'PUT',
          headers: { 'Content-Type': file.type || 'application/octet-stream' },
          body: file,
        }
      )
      if (!response.ok) {
        const body = (await response.json().catch(() => null)) as { error?: string } | null
        throw new Error(body?.error || 'Upload failed')
      }
      setUploaded(true)
      setFile(null)
    } catch (uploadError) {
      setError(uploadError instanceof Error ? uploadError.message : 'Upload failed')
    } finally {
      setUploading(false)
    }
  }

  return (
    <Paper withBorder radius="md" p="xl" maw={520} mx="auto" mt="xl">
      <Stack gap="lg">
        <div>
          <Title order={2}>Upload file</Title>
          <Text c="dimmed" size="sm" mt={4}>
            This link accepts one upload attempt.
          </Text>
        </div>

        {valid === null && <Text c="dimmed">Checking upload link…</Text>}
        {valid === false && (
          <Alert color="red" title="Upload link unavailable">
            This link is invalid or has already been used.
          </Alert>
        )}
        {uploaded && (
          <Alert color="green" title="Upload complete">
            File uploaded. This link cannot be used again.
          </Alert>
        )}
        {error && <Alert color="red">{error}</Alert>}

        {valid && !uploaded && (
          <>
            <FileInput
              label="File"
              placeholder="Choose one file"
              value={file}
              onChange={setFile}
              clearable
              disabled={uploading}
            />
            <Button
              leftSection={<IconUpload size={16} />}
              disabled={!file}
              loading={uploading}
              onClick={uploadFile}
            >
              Upload file
            </Button>
          </>
        )}
      </Stack>
    </Paper>
  )
}
