import { useAuth } from '@local/hooks/authContext'
import { AppShell, MantineProvider } from '@mantine/core'
import { ModalsProvider } from '@mantine/modals'
import { Notifications } from '@mantine/notifications'
import {createRootRouteWithContext, HeadContent, Outlet, redirect, Scripts} from '@tanstack/react-router'
import { Suspense } from 'react'
import '@mantine/core/styles.css'
import '@mantine/notifications/styles.css'
import 'mantine-datatable/styles.css'
import { HeaderBar } from '@local/components/HeaderBar'
import { LoadingScreen } from '@local/components/LoadingScreen'
import { SideNav } from '@local/components/SideNav'
import { api, setActiveSession } from '@local/hooks/api'
import type { AuthState } from '@local/hooks/authContext'
import { UploadProvider } from '@local/hooks/UploadContext'
import { queryClient } from '@local/queryClient.ts'
import { theme } from '@local/theme'
import type { CheckResponse } from '@local/types/api'
import { useDisclosure } from '@mantine/hooks'
import { type QueryClient, QueryClientProvider } from '@tanstack/react-query'

const PUBLIC_ROUTES = ['/login']

export const Route = createRootRouteWithContext<{ queryClient: QueryClient; auth: AuthState }>()({
  beforeLoad: async ({ location }) => {
    let user = null
    try {
      const { data } = await api.get<CheckResponse>('/check')
      setActiveSession(true)
      user = data.user
    } catch {
      setActiveSession(false)
      return { auth: { user: null, loading: false}}
    }

    const isPublicRoute = PUBLIC_ROUTES.some(
      (route) => location.pathname === route || location.pathname.startsWith(route)
    )

    if (!user && !isPublicRoute) {
      throw redirect({ to: '/login' })
    }

    return { auth: { user, loading: false } }
  },
  head: () => ({
    meta: [
      { title: 'BlackFiles' },
      { name: 'description', content: 'File browser and management' },
    ],
    links: [
      {
        rel: 'icon',
        href: '/favicon.ico',
      },
    ],
  }),

  component: RootLayout,
})

function RootLayout() {
  const { user } = useAuth()
  const [mobileOpened, { toggle: toggleMobile }] = useDisclosure()

  return (
    <QueryClientProvider client={queryClient}>
      <HeadContent/>
      <MantineProvider defaultColorScheme="auto" theme={theme}>
        <ModalsProvider>
          <Notifications />
          <UploadProvider>
            <AppShell
              header={{ height: 54 }}
              navbar={
                user
                  ? { width: 220, breakpoint: 'sm', collapsed: { mobile: !mobileOpened } }
                  : undefined
              }
              padding="md"
            >
              {user && (
                <AppShell.Navbar>
                  <SideNav />
                </AppShell.Navbar>
              )}
              <AppShell.Header>
                <HeaderBar mobileOpened={mobileOpened} onToggle={toggleMobile} />
              </AppShell.Header>
              <AppShell.Main>
                <Suspense fallback={<LoadingScreen />}>
                  <Outlet />
                </Suspense>
              </AppShell.Main>
            </AppShell>
          </UploadProvider>
        </ModalsProvider>
      </MantineProvider>
    </QueryClientProvider>
  )
}
