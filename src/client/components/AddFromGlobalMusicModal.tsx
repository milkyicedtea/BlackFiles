import { MusicSongTable } from '@local/components/MusicSongTable'
import { useAddPersonalMusicSong, useGlobalMusicSongs } from '@local/hooks/useMusic'
import { ActionIcon, Group, Modal, Stack, Text, TextInput, Tooltip } from '@mantine/core'
import { useDebouncedValue } from '@mantine/hooks'
import { IconCheck, IconPlus, IconSearch } from '@tabler/icons-react'
import { useEffect, useState } from 'react'

interface AddFromGlobalMusicModalProps {
  opened: boolean
  knownPersonalSongIds: Set<string>
  onClose: () => void
}

export function AddFromGlobalMusicModal({
  opened,
  knownPersonalSongIds,
  onClose,
}: AddFromGlobalMusicModalProps) {
  const [search, setSearch] = useState('')
  const [debouncedSearch] = useDebouncedValue(search, 300)
  const [page, setPage] = useState(1)
  const [limit, setLimit] = useState(10)
  const [addedSongIds, setAddedSongIds] = useState<Set<string>>(new Set())
  const songsQuery = useGlobalMusicSongs({ page, limit, search: debouncedSearch }, opened)
  const addMutation = useAddPersonalMusicSong()

  useEffect(() => {
    if (!opened) return
    setSearch('')
    setPage(1)
    setAddedSongIds(new Set())
  }, [opened])

  const alreadyAdded = (songId: string) =>
    knownPersonalSongIds.has(songId) || addedSongIds.has(songId)

  return (
    <Modal opened={opened} onClose={onClose} title="Add from Global library" size="xl">
      <Stack gap="sm">
        <Text size="sm" c="dimmed">
          Add a global song to your library without creating another copy of its audio file.
        </Text>
        <TextInput
          value={search}
          onChange={(event) => {
            setSearch(event.currentTarget.value)
            setPage(1)
          }}
          leftSection={<IconSearch size={16} />}
          label="Search Global library"
          placeholder="Title, artist, or album"
          size="sm"
        />
        <MusicSongTable
          songs={songsQuery.data?.songs ?? []}
          total={songsQuery.data?.total ?? 0}
          page={page}
          limit={limit}
          loading={songsQuery.isFetching}
          error={songsQuery.error}
          emptyText={
            debouncedSearch.trim()
              ? 'No global songs match this search.'
              : 'The Global library is empty. Ask someone with upload permission to add music.'
          }
          onPageChange={setPage}
          onLimitChange={setLimit}
          renderActions={(song) => {
            const added = alreadyAdded(song.id)
            const adding = addMutation.isPending && addMutation.variables?.id === song.id
            return (
              <Tooltip label={added ? 'Already in My library' : 'Add to My library'}>
                <ActionIcon
                  variant={added ? 'subtle' : 'light'}
                  color={added ? 'teal' : 'blue'}
                  disabled={added}
                  loading={adding}
                  aria-label={
                    added ? `${song.title} is already in My library` : `Add ${song.title}`
                  }
                  onClick={async () => {
                    await addMutation.mutateAsync(song)
                    setAddedSongIds((current) => new Set(current).add(song.id))
                  }}
                >
                  {added ? <IconCheck size={16} /> : <IconPlus size={16} />}
                </ActionIcon>
              </Tooltip>
            )
          }}
        />
        <Group justify="flex-end">
          <Text size="xs" c="dimmed">
            Added songs stay in the Global library.
          </Text>
        </Group>
      </Stack>
    </Modal>
  )
}
