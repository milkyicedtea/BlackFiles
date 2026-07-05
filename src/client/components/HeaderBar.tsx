import logoSrc from '@local/assets/icons8-folder-color-120.png'
import { UploadPanel } from '@local/components/UploadPanel'
import { useAuth } from '@local/hooks/authContext'
import { Badge, Burger, Group, Text } from '@mantine/core'
import { Link, useRouterState } from '@tanstack/react-router'

interface HeaderBarProps {
  mobileOpened?: boolean
  onToggle?: () => void
}

export function HeaderBar({ mobileOpened, onToggle }: HeaderBarProps) {
  const { user } = useAuth()
  const pathname = useRouterState({ select: (s) => s.location.pathname })
  const isLoginRoute = pathname === '/login'
  const isSettingsPage = pathname.startsWith('/settings')

  return (
    <Group h="100%" px="md" justify="space-between">
      <Group gap="xs">
        {!isLoginRoute && user && (
          <Burger
            opened={mobileOpened}
            onClick={onToggle}
            hiddenFrom="sm"
            aria-label="Toggle navigation"
            size="sm"
          />
        )}
        <Link to="/browse" style={{ textDecoration: 'none', color: 'inherit' }}>
          <Group gap={8}>
            <img
              src={logoSrc}
              height={28}
              width={28}
              alt="BlackFiles"
              style={{ display: 'block' }}
            />
            <Text fw={700} size="sm">
              BlackFiles
            </Text>
          </Group>
        </Link>
        {isSettingsPage && (
          <Badge size="xs" variant="light" color="blue">
            Settings
          </Badge>
        )}
      </Group>

      <Group gap="xs">{!isLoginRoute && user && <UploadPanel />}</Group>
    </Group>
  )
}
