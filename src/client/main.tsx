import { setLogoutCallback } from '@local/hooks/api'
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
})

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}

// Auto-logout on session expiry (401 refresh failure)
setLogoutCallback(() => {
  queryClient.clear()
  router.invalidate()
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
