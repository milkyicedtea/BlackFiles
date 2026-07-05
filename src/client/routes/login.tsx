import { api } from '@local/hooks/api'
import { useAuth } from '@local/hooks/authContext'
import type { LoginRequest, LoginResponse } from '@local/types/auth.ts'
import { Button, Container, Paper, Stack, TextInput, Title } from '@mantine/core'
import { useForm } from '@mantine/form'
import { notifications } from '@mantine/notifications'
import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useEffect } from 'react'

export const Route = createFileRoute('/login')({
  component: LoginPage,
})

function LoginPage() {
  const { user } = useAuth()
  const navigate = useNavigate()

  useEffect(() => {
    if (user) navigate({ to: '/browse' })
  }, [user, navigate])

  const form = useForm<LoginRequest>({
    initialValues: { username: '', password: '' },
    validate: {
      username: (v) => (!v ? 'Required' : null),
      password: (v) => (!v ? 'Required' : null),
    },
  })

  async function handleSubmit(values: LoginRequest) {
    try {
      await api.post<LoginResponse>('/auth/login', values, { _silent: true })
      notifications.show({ title: 'Welcome back', message: 'Signed in', color: 'green' })
      navigate({ to: '/browse' })
    } catch {
      notifications.show({
        title: 'Login failed',
        message: 'Invalid username or password',
        color: 'red',
      })
    }
  }

  return (
    <Container size="xs" mt={100}>
      <Paper withBorder shadow="md" p="xl">
        <Title order={2} ta="center" mb="lg">
          BlackFiles
        </Title>
        <form onSubmit={form.onSubmit(handleSubmit)}>
          <Stack gap="sm">
            <TextInput
              label="Username"
              placeholder="Enter your username"
              autoFocus
              {...form.getInputProps('username')}
            />
            <TextInput
              label="Password"
              placeholder="Enter your password"
              type="password"
              {...form.getInputProps('password')}
            />
            <Button type="submit" fullWidth mt="sm">
              Sign in
            </Button>
          </Stack>
        </form>
      </Paper>
    </Container>
  )
}
