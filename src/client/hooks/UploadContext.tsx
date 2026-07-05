import { queryKeys } from '@local/hooks/queryKeys'
import { useQueryClient } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { createContext, useCallback, useContext, useEffect, useRef, useState } from 'react'

export interface UploadItem {
  id: string
  name: string
  progress: number
  status: 'uploading' | 'done' | 'error' | 'cancelled'
  error?: string
  size: number
  filePath: string
}

interface UploadContextValue {
  items: Array<UploadItem>
  addFiles: (files: FileList | Array<File>, targetPath: string) => void
  cancel: (id: string) => void
  clearCompleted: () => void
  hasActive: boolean
  hasAny: boolean
  activeCount: number
}

// Bytes per binary WebSocket frame. The server's tungstenite default
// max_message_size is 64 MiB; 8 MiB stays comfortably below it while keeping
// memory pressure low and ack round-trips cheap.
const WS_CHUNK_SIZE = 8 * 1024 * 1024

const UploadContext = createContext<UploadContextValue | null>(null)

export function UploadProvider({ children }: { children: ReactNode }) {
  const [items, setItems] = useState<Array<UploadItem>>([])
  const queryClient = useQueryClient()
  const socketsRef = useRef<Map<string, WebSocket>>(new Map())

  useEffect(
    () => () => {
      for (const s of socketsRef.current.values()) s.close()
    },
    []
  )

  const updateItem = useCallback((id: string, patch: Partial<UploadItem>) => {
    setItems((prev) => prev.map((i) => (i.id === id ? { ...i, ...patch } : i)))
  }, [])

  const cancelItem = useCallback((id: string) => {
    // Closing the socket makes the server drop the partial file (see upload_ws).
    socketsRef.current.get(id)?.close()
    socketsRef.current.delete(id)
    setItems((prev) => prev.map((i) => (i.id === id ? { ...i, status: 'cancelled' as const } : i)))
  }, [])

  const clearCompleted = useCallback(() => {
    setItems((prev) => prev.filter((i) => i.status === 'uploading'))
  }, [])

  const addFilesHandler = useCallback(
    async (files: FileList | Array<File>, targetPath: string) => {
      const fileArr = Array.from(files)
      const newItems: Array<UploadItem> = fileArr.map((f) => ({
        id: crypto.randomUUID(),
        name: f.name,
        progress: 0,
        status: 'uploading' as const,
        size: f.size,
        filePath: targetPath
          ? `${targetPath}/${encodeURIComponent(f.name)}`
          : encodeURIComponent(f.name),
      }))

      setItems((prev) => [...prev, ...newItems])

      for (let fi = 0; fi < newItems.length; fi++) {
        const entry = newItems[fi]
        const file = fileArr[fi]

        // WebSocket upload — see src/server/files.rs::upload_ws for the protocol.
        // Progress is driven by server "ack" messages (bytes persisted on disk),
        // so the bar cannot race ahead of the writer the way a monolithic PUT's
        // onUploadProgress did (it counted bytes handed to the network buffer).
        const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
        const wsUrl = `${wsProtocol}//${window.location.host}/api/upload?path=${entry.filePath}`
        const socket = new WebSocket(wsUrl)
        socketsRef.current.set(entry.id, socket)

        let offset = 0
        let resolved = false
        let ackResolver: ((done: boolean) => void) | null = null

        const awaitAck = () =>
          new Promise<boolean>((r) => {
            ackResolver = r
          })

        await new Promise<void>((resolve) => {
          const finish = () => {
            if (resolved) return
            resolved = true
            resolve()
          }

          socket.onopen = async () => {
            try {
              socket.send(JSON.stringify({ type: 'init', size: file.size, mime: file.type || '' }))
              while (offset < file.size && socket.readyState === WebSocket.OPEN) {
                const end = Math.min(offset + WS_CHUNK_SIZE, file.size)
                const buf = await file.slice(offset, end).arrayBuffer()
                socket.send(buf)
                const done = await awaitAck()
                if (done) break
                offset = end
                updateItem(entry.id, {
                  progress: file.size > 0 ? Math.round((offset * 100) / file.size) : 100,
                })
              }
              if (socket.readyState === WebSocket.OPEN) {
                socket.send(JSON.stringify({ type: 'end' }))
              }
            } catch {
              // The onerror/onclose handlers surface the failure; ensure we don't hang.
              ackResolver?.(true)
            }
          }

          socket.onmessage = (ev) => {
            if (typeof ev.data !== 'string') return
            let msg: { type: string; bytes?: number; message?: string }
            try {
              msg = JSON.parse(ev.data)
            } catch {
              return
            }
            if (msg.type === 'ack') {
              ackResolver?.(false)
              ackResolver = null
            } else if (msg.type === 'done') {
              ackResolver?.(true)
              updateItem(entry.id, { progress: 100, status: 'done' })
              queryClient.invalidateQueries({ queryKey: queryKeys.directory.all })
              finish()
            } else if (msg.type === 'error') {
              updateItem(entry.id, { status: 'error', error: msg.message ?? 'Upload failed' })
              ackResolver?.(true)
              finish()
            }
          }

          socket.onerror = () => {
            updateItem(entry.id, { status: 'error', error: 'Connection error' })
            ackResolver?.(true)
            finish()
          }

          socket.onclose = () => {
            // Release a pending chunk-ack so the send loop can't hang — some
            // browsers report an abnormal close via onclose only, no onerror.
            ackResolver?.(true)
            if (resolved) return
            // Unexpected close without a "done": if still uploading, mark errored.
            // (A user cancel already set 'cancelled' via cancelItem, which we leave alone.)
            setItems((prev) =>
              prev.map((i) =>
                i.id === entry.id && i.status === 'uploading'
                  ? { ...i, status: 'error', error: 'Connection closed' }
                  : i
              )
            )
            finish()
          }
        })

        socketsRef.current.delete(entry.id)
      }
    },
    [updateItem, queryClient]
  )

  const activeCount = items.filter((i) => i.status === 'uploading').length

  return (
    <UploadContext.Provider
      value={{
        items,
        addFiles: addFilesHandler,
        cancel: cancelItem,
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
