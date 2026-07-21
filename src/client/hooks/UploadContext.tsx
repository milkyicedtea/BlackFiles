import { queryKeys } from '@local/hooks/queryKeys'
import Uppy from '@uppy/core'
import Tus from '@uppy/tus'
import { useQueryClient } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { createContext, useCallback, useContext, useEffect, useRef, useState } from 'react'

interface PendingTusUpload {
  id: string
  target_path: string
  upload_length: number
  upload_offset: number
}

interface UppyUploadFile {
  meta: Record<string, unknown>
  tus?: {
    uploadUrl?: string | null
  }
}

export interface UploadItem {
  id: string
  name: string
  progress: number
  status: 'uploading' | 'resumable' | 'done' | 'error' | 'cancelled'
  error?: string
  size: number
  filePath: string
  uploadUrl?: string
}

interface UploadContextValue {
  items: Array<UploadItem>
  addFiles: (files: FileList | Array<File>, targetPath: string) => void
  cancel: (id: string) => void
  remove: (id: string) => void
  resume: (id: string, file: File) => void
  clearCompleted: () => void
  hasActive: boolean
  hasAny: boolean
  activeCount: number
}

const TUS_CHUNK_SIZE = 8 * 1024 * 1024

function uploadErrorMessage(error: { message?: string }): string {
  if (
    'originalResponse' in error &&
    error.originalResponse !== null &&
    typeof error.originalResponse === 'object' &&
    'getBody' in error.originalResponse &&
    typeof error.originalResponse.getBody === 'function'
  ) {
    const body = error.originalResponse.getBody()
    if (body) {
      try {
        const parsed: unknown = JSON.parse(body)
        if (
          typeof parsed === 'object' &&
          parsed !== null &&
          'error' in parsed &&
          typeof parsed.error === 'string'
        ) {
          return parsed.error
        }
      } catch {
        // Fall through to Uppy’s message when the server did not send JSON.
      }
    }
  }

  return error.message || 'Upload failed'
}

const UploadContext = createContext<UploadContextValue | null>(null)

function isPendingTusUpload(value: unknown): value is PendingTusUpload {
  return (
    typeof value === 'object' &&
    value !== null &&
    'id' in value &&
    typeof value.id === 'string' &&
    'target_path' in value &&
    typeof value.target_path === 'string' &&
    'upload_length' in value &&
    typeof value.upload_length === 'number' &&
    'upload_offset' in value &&
    typeof value.upload_offset === 'number'
  )
}

export function UploadProvider({ children }: { children: ReactNode }) {
  const [items, setItems] = useState<Array<UploadItem>>([])
  const queryClient = useQueryClient()
  const uploadFileIds = useRef<Map<string, string>>(new Map())
  const [uppy] = useState(
    () =>
      new Uppy({ autoProceed: true }).use(Tus, {
        endpoint: '/api/uploads',
        withCredentials: true,
        chunkSize: TUS_CHUNK_SIZE,
        limit: 3,
        retryDelays: [0, 1000, 3000, 5000],
        onShouldRetry: (error, _retryAttempt, _options, defaultOnShouldRetry) => {
          const status = error.originalResponse?.getStatus()
          if (status !== undefined && status >= 400 && status < 500) return false
          return defaultOnShouldRetry(error)
        },
        removeFingerprintOnSuccess: true,
      })
  )

  const updateItem = useCallback((id: string, patch: Partial<UploadItem>) => {
    setItems((prev) => prev.map((item) => (item.id === id ? { ...item, ...patch } : item)))
  }, [])

  useEffect(() => {
    let active = true

    void fetch('/api/uploads', { credentials: 'same-origin' })
      .then(async (response) => {
        if (!response.ok) return
        const body: unknown = await response.json()
        if (!Array.isArray(body) || !active) return

        const pending = body.filter(isPendingTusUpload)
        setItems((prev) => {
          const knownUrls = new Set(prev.flatMap((item) => (item.uploadUrl ? [item.uploadUrl] : [])))
          const restored = pending
            .map((session) => {
              const uploadUrl = `/api/uploads/${session.id}`
              const name = session.target_path.split('/').at(-1) || session.target_path
              return {
                id: session.id,
                name,
                progress:
                  session.upload_length > 0
                    ? Math.round((session.upload_offset * 100) / session.upload_length)
                    : 100,
                status: 'resumable' as const,
                size: session.upload_length,
                filePath: session.target_path,
                uploadUrl,
              }
            })
            .filter((item) => !knownUrls.has(item.uploadUrl))
          return [...prev, ...restored]
        })
      })
      .catch(() => undefined)

    return () => {
      active = false
    }
  }, [])
  useEffect(() => {
    const getClientUploadId = (file: UppyUploadFile | undefined) => {
      const id = file?.meta.clientUploadId
      return typeof id === 'string' ? id : null
    }

    const handleProgress = (
      file: UppyUploadFile | undefined,
      progress: { bytesUploaded: number; bytesTotal: number | null }
    ) => {
      const id = getClientUploadId(file)
      if (!id) return
      const total = progress.bytesTotal
      const uploadUrl = file?.tus?.uploadUrl
      updateItem(id, {
        progress: total && total > 0 ? Math.round((progress.bytesUploaded * 100) / total) : 100,
        ...(uploadUrl ? { uploadUrl } : {}),
      })
    }

    const handleSuccess = (file: UppyUploadFile | undefined) => {
      const id = getClientUploadId(file)
      if (!id) return
      updateItem(id, { progress: 100, status: 'done' })
      queryClient.invalidateQueries({ queryKey: queryKeys.directory.all })
    }

    const handleError = (
      file: UppyUploadFile | undefined,
      error: { message?: string }
    ) => {
      const id = getClientUploadId(file)
      if (!id) return
      setItems((prev) =>
        prev.map((item) =>
          item.id === id && item.status === 'uploading'
            ? { ...item, status: 'error', error: uploadErrorMessage(error) }
            : item
        )
      )
    }

    uppy.on('upload-progress', handleProgress)
    uppy.on('upload-success', handleSuccess)
    uppy.on('upload-error', handleError)

    return () => {
      uppy.off('upload-progress', handleProgress)
      uppy.off('upload-success', handleSuccess)
      uppy.off('upload-error', handleError)
    }
  }, [queryClient, updateItem, uppy])

  const cancelItem = useCallback(
    (id: string) => {
      const uppyFileId = uploadFileIds.current.get(id)
      if (uppyFileId && uppy.getFile(uppyFileId)) uppy.removeFile(uppyFileId)
      setItems((prev) =>
        prev.map((item) => (item.id === id ? { ...item, status: 'cancelled' as const } : item))
      )
    },
    [uppy]
  )

  const removeItem = useCallback(
    (id: string) => {
      const item = items.find((entry) => entry.id === id)
      const uppyFileId = uploadFileIds.current.get(id)
      if (uppyFileId && uppy.getFile(uppyFileId)) {
        uppy.removeFile(uppyFileId)
      } else if (item?.uploadUrl) {
        void fetch(item.uploadUrl, {
          method: 'DELETE',
          credentials: 'same-origin',
          headers: { 'Tus-Resumable': '1.0.0' },
        })
      }
      uploadFileIds.current.delete(id)
      setItems((prev) => prev.filter((entry) => entry.id !== id))
    },
    [items, uppy]
  )

  const clearCompleted = useCallback(() => {
    for (const item of items) {
      if (item.status === 'uploading' || item.status === 'resumable') continue
      const uppyFileId = uploadFileIds.current.get(item.id)
      if (uppyFileId && uppy.getFile(uppyFileId)) uppy.removeFile(uppyFileId)
      uploadFileIds.current.delete(item.id)
    }
    setItems((prev) =>
      prev.filter((item) => item.status === 'uploading' || item.status === 'resumable')
    )
  }, [items, uppy])

  const addFilesHandler = useCallback(
    (files: FileList | Array<File>, targetPath: string) => {
      for (const file of Array.from(files)) {
        const id = crypto.randomUUID()
        const entry: UploadItem = {
          id,
          name: file.name,
          progress: 0,
          status: 'uploading',
          size: file.size,
          filePath: targetPath ? `${targetPath}/${file.name}` : file.name,
        }

        setItems((prev) => [...prev, entry])
        try {
          const uppyFileId = uppy.addFile({
            name: file.name,
            type: file.type,
            data: file,
            meta: {
              clientUploadId: id,
              filename: file.name,
              targetPath,
              relativePath: targetPath ? `${targetPath}/${file.name}` : file.name,
            },
          })
          uploadFileIds.current.set(id, uppyFileId)
        } catch (error) {
          updateItem(id, {
            status: 'error',
            error: error instanceof Error ? error.message : 'Upload could not be queued',
          })
        }
      }
    },
    [updateItem, uppy]
  )

  const resumeItem = useCallback(
    (id: string, file: File) => {
      const item = items.find((entry) => entry.id === id)
      if (!item || item.status !== 'resumable' || !item.uploadUrl) return
      if (file.name !== item.name || file.size !== item.size) {
        updateItem(id, { error: 'Select the original file to resume this upload' })
        return
      }

      const separator = item.filePath.lastIndexOf('/')
      const targetPath = separator === -1 ? '' : item.filePath.slice(0, separator)
      updateItem(id, { status: 'uploading', error: undefined })
      try {
        const uppyFileId = uppy.addFile({
          name: file.name,
          type: file.type,
          data: file,
          meta: {
            clientUploadId: id,
            filename: file.name,
            targetPath,
            relativePath: item.filePath,
          },
        })
        uppy.setFileState(uppyFileId, { tus: { uploadUrl: item.uploadUrl } })
        uploadFileIds.current.set(id, uppyFileId)
      } catch (error) {
        updateItem(id, {
          status: 'resumable',
          error: error instanceof Error ? error.message : 'Upload could not be resumed',
        })
      }
    },
    [items, updateItem, uppy]
  )

  const activeCount = items.filter((item) => item.status === 'uploading').length

  return (
    <UploadContext.Provider
      value={{
        items,
        addFiles: addFilesHandler,
        cancel: cancelItem,
        remove: removeItem,
        resume: resumeItem,
        clearCompleted,
        hasActive: activeCount > 0,
        hasAny: items.length > 0,
        activeCount,
      }}
    >
      {children}
    </UploadContext.Provider>
  )
}

export function useUploader(): UploadContextValue {
  const ctx = useContext(UploadContext)
  if (!ctx) throw new Error('useUploader must be used within UploadProvider')
  return ctx
}
