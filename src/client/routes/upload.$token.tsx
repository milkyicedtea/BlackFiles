import { Alert, Button, FileInput, Paper, Progress, Stack, Text, Title } from '@mantine/core'
import { IconPlayerPlay, IconUpload, IconX } from '@tabler/icons-react'
import { createFileRoute } from '@tanstack/react-router'
import Uppy from '@uppy/core'
import Tus from '@uppy/tus'
import { useEffect, useState } from 'react'

const TUS_CHUNK_SIZE = 8 * 1024 * 1024

interface PublicTusUpload {
  id: string
  target_path: string
  upload_length: number
  upload_offset: number
}

interface PublicUploadLinkStatus {
  ready: boolean
  session: PublicTusUpload | null
}

interface UppyUploadFile {
  id: string
  tus?: {
    uploadUrl?: string | null
  }
}

export const Route = createFileRoute('/upload/$token')({
  component: PublicUploadPage,
})

function PublicUploadPage() {
  const { token } = Route.useParams()

  return <PublicUploadForm key={token} token={token} />
}

function PublicUploadForm({ token }: { token: string }) {
  const endpoint = `/api/public/upload-links/${encodeURIComponent(token)}/uploads`
  const [valid, setValid] = useState<boolean | null>(null)
  const [file, setFile] = useState<File | null>(null)
  const [pending, setPending] = useState<PublicTusUpload | null>(null)
  const [uploading, setUploading] = useState(false)
  const [progress, setProgress] = useState(0)
  const [uploadUrl, setUploadUrl] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [uploaded, setUploaded] = useState(false)
  const [uppy] = useState(() =>
    new Uppy({ autoProceed: false, restrictions: { maxNumberOfFiles: 1 } }).use(Tus, {
      endpoint,
      chunkSize: TUS_CHUNK_SIZE,
      limit: 1,
      retryDelays: [0, 1000, 3000, 5000],
      onShouldRetry: (uploadError, _retryAttempt, _options, defaultOnShouldRetry) => {
        const status = uploadError.originalResponse?.getStatus()
        if (status !== undefined && status >= 400 && status < 500) return false
        return defaultOnShouldRetry(uploadError)
      },
      removeFingerprintOnSuccess: true,
    })
  )

  useEffect(() => {
    let active = true

    async function validateLink() {
      try {
        const response = await fetch(`/api/public/upload-links/${encodeURIComponent(token)}`)
        if (!response.ok) throw new Error()
        const status = (await response.json()) as PublicUploadLinkStatus
        if (!active) return
        setValid(status.ready)
        setPending(status.session)
        if (status.session) {
          setProgress(
            status.session.upload_length > 0
              ? Math.round((status.session.upload_offset * 100) / status.session.upload_length)
              : 0
          )
        }
      } catch {
        if (active) setValid(false)
      }
    }

    void validateLink()
    return () => {
      active = false
    }
  }, [token])

  useEffect(() => {
    const handleProgress = (
      uppyFile: UppyUploadFile | undefined,
      uploadProgress: { bytesUploaded: number; bytesTotal: number | null }
    ) => {
      const total = uploadProgress.bytesTotal
      setProgress(total && total > 0 ? Math.round((uploadProgress.bytesUploaded * 100) / total) : 0)
      const url = uppyFile?.tus?.uploadUrl
      if (url) setUploadUrl(url)
    }
    const handleSuccess = () => {
      setProgress(100)
      setPending(null)
      setUploadUrl(null)
      setUploading(false)
      setUploaded(true)
      setFile(null)
    }
    const handleError = (
      _uppyFile: UppyUploadFile | undefined,
      uploadError: { message?: string }
    ) => {
      setUploading(false)
      setError(uploadError.message || 'Upload failed')
    }

    uppy.on('upload-progress', handleProgress)
    uppy.on('upload-success', handleSuccess)
    uppy.on('upload-error', handleError)
    return () => {
      uppy.off('upload-progress', handleProgress)
      uppy.off('upload-success', handleSuccess)
      uppy.off('upload-error', handleError)
      uppy.destroy()
    }
  }, [uppy])

  async function uploadFile() {
    if (!file) return
    if (
      pending &&
      (file.name !== pending.target_path.split('/').at(-1) || file.size !== pending.upload_length)
    ) {
      setError('Select the original file to resume this upload')
      return
    }

    setUploading(true)
    setError(null)
    try {
      for (const existingFile of uppy.getFiles()) uppy.removeFile(existingFile.id)
      const uppyFileId = uppy.addFile({
        name: file.name,
        type: file.type,
        data: file,
        meta: { filename: file.name },
      })
      if (pending) {
        const resumeUrl = `${endpoint}/${pending.id}`
        uppy.setFileState(uppyFileId, { tus: { uploadUrl: resumeUrl } })
        setUploadUrl(resumeUrl)
      }
      await uppy.upload()
    } catch (uploadError) {
      setUploading(false)
      setError(uploadError instanceof Error ? uploadError.message : 'Upload could not be started')
    }
  }

  async function cancelUpload() {
    uppy.cancelAll()
    const url = uploadUrl || (pending ? `${endpoint}/${pending.id}` : null)
    if (url) {
      await fetch(url, {
        method: 'DELETE',
        headers: { 'Tus-Resumable': '1.0.0' },
      }).catch(() => undefined)
    }
    setPending(null)
    setUploadUrl(null)
    setProgress(0)
    setUploading(false)
    setError(null)
  }

  return (
    <Paper withBorder radius="md" p="xl" maw={520} mx="auto" mt="xl">
      <Stack gap="lg">
        <div>
          <Title order={2}>Upload file</Title>
          <Text c="dimmed" size="sm" mt={4}>
            This link accepts one file. Interrupted uploads can be resumed with the original file.
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
            {pending && (
              <Alert color="blue" title="Upload ready to resume">
                Select <strong>{pending.target_path.split('/').at(-1)}</strong> again to continue
                from {progress}%.
              </Alert>
            )}
            <FileInput
              label="File"
              placeholder="Choose one file"
              value={file}
              onChange={(nextFile) => {
                setFile(nextFile)
                setError(null)
              }}
              clearable
              disabled={uploading}
            />
            {uploading && <Progress value={progress} aria-label="Upload progress" />}
            <Button
              leftSection={pending ? <IconPlayerPlay size={16} /> : <IconUpload size={16} />}
              disabled={!file}
              loading={uploading}
              onClick={uploadFile}
            >
              {pending ? 'Resume upload' : 'Upload file'}
            </Button>
            {pending && !uploading && (
              <Button
                variant="subtle"
                color="red"
                leftSection={<IconX size={16} />}
                onClick={cancelUpload}
              >
                Cancel upload
              </Button>
            )}
          </>
        )}
      </Stack>
    </Paper>
  )
}
