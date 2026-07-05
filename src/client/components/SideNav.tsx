import { api } from '@local/hooks/api'
import { isAdmin, useAuth } from '@local/hooks/authContext'
import {
  ActionIcon,
  Button,
  Divider,
  Flex,
  NavLink,
  ScrollArea,
  Stack,
  useComputedColorScheme,
  useMantineColorScheme,
} from '@mantine/core'
import {
  IconFolder,
  IconMoon,
  IconSettings,
  IconShieldLock,
  IconSun,
  IconUsers,
} from '@tabler/icons-react'
import { useQueryClient } from '@tanstack/react-query'
import { Link, useNavigate, useRouter } from '@tanstack/react-router'

const compact = { root: { paddingTop: 1, paddingBottom: 1 } }

export function SideNav() {
  const { user } = useAuth()
  const { setColorScheme } = useMantineColorScheme()
  const colorScheme = useComputedColorScheme()
  const navigate = useNavigate()
  const router = useRouter()
  const queryClient = useQueryClient()
  const admin = isAdmin(user)

  function toggleColorScheme() {
    setColorScheme(colorScheme === 'dark' ? 'light' : 'dark')
  }

  async function handleLogout() {
    try {
      await api.post<void>('/auth/logout', undefined, { _silent: true })
    } catch {
      /* ignore */
    }
    queryClient.clear()
    await router.invalidate()
    navigate({ to: '/login' })
  }

  return (
    <Flex h="100%" direction="column" p="xs">
      <ScrollArea flex={1}>
        <Stack gap={0}>
          <NavLink
            component={Link}
            preload="intent"
            to="/browse"
            label="Browse"
            leftSection={<IconFolder size={16} />}
            styles={compact}
            fz="xs"
          />

          {admin && (
            <NavLink
              label="Settings"
              leftSection={<IconSettings size={16} />}
              defaultOpened
              childrenOffset={0}
              styles={compact}
              fz="xs"
            >
              <NavLink
                component={Link}
                preload="intent"
                to="/settings"
                activeOptions={{ exact: true }}
                label="Overview"
                leftSection={<IconSettings size={14} />}
                pl="xl"
                styles={compact}
                fz="xs"
              />
              <NavLink
                component={Link}
                preload="intent"
                to="/settings/users"
                label="Users"
                leftSection={<IconUsers size={14} />}
                pl="xl"
                styles={compact}
                fz="xs"
              />
              <NavLink
                component={Link}
                preload="intent"
                to="/settings/roles"
                label="Roles"
                leftSection={<IconShieldLock size={14} />}
                pl="xl"
                styles={compact}
                fz="xs"
              />
            </NavLink>
          )}
        </Stack>
      </ScrollArea>

      <Divider mt={2} />
      <Flex align="center" justify="space-between" px="sm" pt="xs">
        <ActionIcon variant="default" onClick={toggleColorScheme} size="md">
          {colorScheme === 'dark' ? <IconMoon size={16} /> : <IconSun size={16}/>}
        </ActionIcon>
        {user && (
          <Button size="xs" variant="subtle" color="red" onClick={handleLogout}>
            Logout
          </Button>
        )}
      </Flex>
    </Flex>
  )
}
