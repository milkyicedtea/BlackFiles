import { api } from '@local/hooks/api'
import { queryKeys } from '@local/hooks/queryKeys'
import { useQueryClient } from '@tanstack/react-query'
import { useCallback, useRef, useState } from 'react'

export interface UploadEntry {
  id: string
  name: string
  progress: number
  status: 'pending' | 'uploading' | 'done' | 'error' | 'cancelled'
  error?: string
  cancel: () => void
}

export function useFileOperations(currentPath: string) {
  const queryClient = useQueryClient()
  const [uploads, setUploads] = useState<Array<UploadEntry>>([])
  const abortMap = useRef<Map<string, AbortController>>(new Map())

  const removeUpload = useCallback((id: string) => {
    abortMap.current.delete(id)
    setUploads((prev) => prev.filter((u) => u.id !== id))
  }, [])

  const uploadFiles = useCallback(
    async (files: FileList | Array<File>) => {
      const entries: Array<UploadEntry> = Array.from(files).map((file) => ({
        id: crypto.randomUUID(),
        name: file.name,
        progress: 0,
        status: 'pending' as const,
        cancel: () => {},
      }))

      setUploads((prev) => [...prev, ...entries])

      // Process each file in parallel (up to 3 at a time)
      const queue = [...entries]
      const CHUNK_SIZE = 5 * 1024 * 1024 // 5 MiB

      async function processNext() {
        if (queue.length === 0) return
        const entry = queue.shift()
        if (!entry) return

        const file = Array.from(files).find((f) => f.name === entry.name)
        if (!file) return

        const controller = new AbortController()
        abortMap.current.set(entry.id, controller)

        entry.cancel = () => {
          controller.abort()
          abortMap.current.delete(entry.id)
          setUploads((prev) =>
            prev.map((u) =>
              u.id === entry.id ? { ...u, status: 'cancelled', cancel: () => {} } : u
            )
          )
        }

        setUploads((prev) =>
          prev.map((u) =>
            u.id === entry.id ? { ...u, status: 'uploading', cancel: entry.cancel } : u
          )
        )

        try {
          const basePath = currentPath
            ? `${currentPath}/${encodeURIComponent(file.name)}`
            : encodeURIComponent(file.name)
          const totalChunks = Math.ceil(file.size / CHUNK_SIZE)
          let completedChunks = 0

          for (let i = 0; i < totalChunks; i++) {
            if (controller.signal.aborted) return

            const start = i * CHUNK_SIZE
            const end = Math.min(start + CHUNK_SIZE, file.size)
            const chunk = file.slice(start, end)
            const offset = start > 0 ? `?offset=${start}` : ''

            const { promise, resolve, reject } = Promise.withResolvers<void>()
            const xhr = new XMLHttpRequest()

            xhr.onload = () => {
              if (xhr.status >= 200 && xhr.status < 300) {
                resolve()
              } else {
                reject(new Error(`Upload failed (${xhr.status})`))
              }
            }

            xhr.onerror = () => reject(new Error('Network error'))
            xhr.onabort = () => reject(new DOMException('Aborted', 'AbortError'))

            const abortHandler = () => xhr.abort()
            controller.signal.addEventListener('abort', abortHandler, { once: true })

            xhr.open('PUT', `/api/files/${basePath}${offset}`)
            xhr.setRequestHeader('Content-Type', file.type || 'application/octet-stream')
            xhr.withCredentials = true
            xhr.send(chunk)

            await promise
            controller.signal.removeEventListener('abort', abortHandler)

            completedChunks++
            setUploads((prev) =>
              prev.map((u) =>
                u.id === entry.id
                  ? { ...u, progress: Math.round((completedChunks / totalChunks) * 100) }
                  : u
              )
            )
          }

          setUploads((prev) =>
            prev.map((u) =>
              u.id === entry.id ? { ...u, progress: 100, status: 'done', cancel: () => {} } : u
            )
          )
        } catch (err: unknown) {
          if (controller.signal.aborted) return
          const message = err instanceof Error ? err.message : 'Upload failed'
          setUploads((prev) =>
            prev.map((u) =>
              u.id === entry.id ? { ...u, status: 'error', error: message, cancel: () => {} } : u
            )
          )
        } finally {
          abortMap.current.delete(entry.id)
        }

        await processNext()
      }
      // Start up to 3 concurrent uploads
      const workers = Array.from({ length: Math.min(3, queue.length) }, () => processNext())
      await Promise.all(workers)

      // Refresh the file listing
      queryClient.invalidateQueries({ queryKey: queryKeys.directory.all })
    },
    [currentPath, queryClient]
  )

  const deleteFile = useCallback(
    async (filePath: string) => {
      await api.delete(`/files/${filePath}`, { _successMessage: 'File deleted' })
      queryClient.invalidateQueries({ queryKey: queryKeys.directory.all })
    },
    [queryClient]
  )

  const clearCompleted = useCallback(() => {
    setUploads((prev) => prev.filter((u) => u.status === 'pending' || u.status === 'uploading'))
  }, [])

  return { uploads, uploadFiles, deleteFile, removeUpload, clearCompleted }
}
