import { notifications } from '@mantine/notifications'
import axios from 'axios'

declare module 'axios' {
  interface AxiosRequestConfig {
    _retry?: boolean
    _skipAuthRefresh?: boolean
    // Suppress both success and error notifications for this request.
    _silent?: boolean
    // Success toast message. If omitted for mutating methods, no success
    // toast is shown — set this at call sites whose success should be surfaced.
    _successMessage?: string
    // Override the error toast message. Defaults to the server's `error` field
    // or the axios error message.
    _errorMessage?: string
  }
}

axios.defaults.withCredentials = true

export const api = axios.create({
  baseURL: `/api`,
  withCredentials: true,
  headers: { 'Content-Type': 'application/json' },
})

let logoutCallback: (() => void) | null = null
let activeSession = false

export function setLogoutCallback(cb: () => void) {
  logoutCallback = cb
}

export function setActiveSession(v: boolean) {
  activeSession = v
}

function extractErrorMessage(err: unknown): string {
  if (err && typeof err === 'object' && 'response' in err) {
    const e = err as { response?: { data?: { error?: string } }; message?: string }
    return e.response?.data?.error || e.message || 'Request failed'
  }
  if (err instanceof Error) return err.message
  return 'Request failed'
}

// Success: surface mutating calls that opt in via `_successMessage`.
api.interceptors.response.use((res) => {
  const cfg = res.config
  if (cfg?._silent || !cfg?._successMessage) return res
  const method = (cfg.method || 'get').toLowerCase()
  if (method === 'get') return res
  notifications.show({
    title: 'Success',
    message: cfg._successMessage,
    color: 'green',
  })
  return res
})

// Errors: surface every non-silent failure (network, 4xx, 5xx) unless the
// request is a 401 that we're about to refresh-and-retry.
api.interceptors.response.use(
  (res) => res,
  async (err) => {
    const cfg = err.config

    if (err.response?.status === 401 && !cfg?._retry && !cfg?._skipAuthRefresh) {
      cfg._retry = true
      try {
        await api.post('/auth/refresh', undefined, { _skipAuthRefresh: true, _silent: true })
        return api(cfg)
      } catch {
        if (activeSession) {
          activeSession = false
          if (logoutCallback) logoutCallback()
        }
      }
    }

    // The refresh path above rejects on final failure; the internal refresh
    // call is `_silent`, and the retried original request carries its own
    // `_silent` flag — so suppressed requests don't double-toast.
    if (!cfg?._silent) {
      notifications.show({
        title: 'Error',
        message: cfg?._errorMessage || extractErrorMessage(err),
        color: 'red',
      })
    }

    return Promise.reject(err)
  }
)
