import { api, setLogoutCallback } from '@local/hooks/api'
import { defaultAuthState } from '@local/hooks/authContext'
import { queryClient } from '@local/queryClient'
import { routeTree } from '@local/routeTree.gen'
import { createRouter, RouterProvider } from '@tanstack/react-router'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'

import '@mantine/core/styles.css'
import '@mantine/notifications/styles.css'
import 'mantine-datatable/styles.css'

const router = createRouter({
  routeTree,
  context: {
    queryClient,
    auth: defaultAuthState,
  },
  defaultErrorComponent: ({ error }) => {
    console.error('Router error:', error)

    const message = error instanceof Error ? error.message : 'Something went wrong!'

    return (
      <div style={{ padding: '1rem' }}>
        <h2> Unhandled error </h2>
        <pre style={{ whiteSpace: 'pre-wrap' }}>{message}</pre>
      </div>
    )
  },
})

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}

// Auto-logout on session expiry (401 refresh failure)
setLogoutCallback(async () => {
  await api
    .post('/auth/logout', undefined, { _skipAuthRefresh: true, _silent: true })
    .catch((err) => {
      console.log('setLogoutCallback error:', err)
    })
  queryClient.clear()
  await router.invalidate()
})

const rootElement = document.getElementById('root')
if (rootElement && !rootElement.innerHTML) {
  const root = createRoot(rootElement)
  root.render(
    <StrictMode>
      <RouterProvider router={router} />
    </StrictMode>
  )
}
