import { AddFromGlobalMusicModal } from '@local/components/AddFromGlobalMusicModal'
import { MusicSongTable } from '@local/components/MusicSongTable'
import { MusicTagEditor } from '@local/components/MusicTagEditor'
import { MusicUploadButton } from '@local/components/MusicUploadButton'
import { ProtectedPage } from '@local/components/ProtectedPage'
import {
  useAddPersonalMusicSong,
  useDeleteGlobalMusicSong,
  useGlobalMusicSelection,
  useGlobalMusicSongs,
  useKnownPersonalSongIds,
  useMusicPermissions,
  usePersonalMusicSongs,
  useRemovePersonalMusicSong,
  useSetPersonalMusicSongs,
  useUpdateMusicSong,
} from '@local/hooks/useMusic'
import {
  mergeVisibleMusicSelection,
  musicSongIdsForMembership,
  withoutMusicSongs,
} from '@local/lib/musicSelection'
import type { MusicSong, MusicTagUpdate } from '@local/types/music'
import {
  ActionIcon,
  Button,
  Group,
  Paper,
  Stack,
  Tabs,
  Text,
  TextInput,
  Title,
  Tooltip,
} from '@mantine/core'
import { useDebouncedValue } from '@mantine/hooks'
import { modals } from '@mantine/modals'
import {
  IconEdit,
  IconHeadphones,
  IconMinus,
  IconPlus,
  IconSearch,
  IconTrash,
  IconWorld,
} from '@tabler/icons-react'
import { createFileRoute } from '@tanstack/react-router'
import { useState } from 'react'

export const Route = createFileRoute('/music')({
  component: () => (
    <ProtectedPage>
      <MusicPage />
    </ProtectedPage>
  ),
})

type MusicTab = 'global' | 'personal'

function MusicPage() {
  const [tab, setTab] = useState<MusicTab>('global')
  const [globalSearch, setGlobalSearch] = useState('')
  const [personalSearch, setPersonalSearch] = useState('')
  const [debouncedGlobalSearch] = useDebouncedValue(globalSearch, 300)
  const [debouncedPersonalSearch] = useDebouncedValue(personalSearch, 300)
  const [globalPage, setGlobalPage] = useState(1)
  const [personalPage, setPersonalPage] = useState(1)
  const [globalLimit, setGlobalLimit] = useState(50)
  const [personalLimit, setPersonalLimit] = useState(50)
  const [editingSong, setEditingSong] = useState<MusicSong | null>(null)
  const [addModalOpen, setAddModalOpen] = useState(false)
  const [selectedGlobalMembership, setSelectedGlobalMembership] = useState<Map<string, boolean>>(
    () => new Map()
  )
  const [allMatchingSongsSelected, setAllMatchingSongsSelected] = useState(false)

  const permissions = useMusicPermissions()
  const globalQuery = useGlobalMusicSongs({
    page: globalPage,
    limit: globalLimit,
    search: debouncedGlobalSearch,
  })
  const personalQuery = usePersonalMusicSongs({
    page: personalPage,
    limit: personalLimit,
    search: debouncedPersonalSearch,
  })
  const globalSongs = globalQuery.data?.songs ?? []
  const globalTotal = globalQuery.data?.total ?? 0
  const selectedGlobalSongs = globalSongs.filter((song) => selectedGlobalMembership.has(song.id))
  let selectedSongsToAddCount = 0
  let selectedSongsToRemoveCount = 0
  for (const inLibrary of selectedGlobalMembership.values()) {
    if (inLibrary) selectedSongsToRemoveCount += 1
    else selectedSongsToAddCount += 1
  }
  const allVisibleSongsSelected =
    globalSongs.length > 0 && globalSongs.every((song) => selectedGlobalMembership.has(song.id))
  const knownPersonalSongIds = useKnownPersonalSongIds(personalQuery.data?.songs ?? [])
  const updateSong = useUpdateMusicSong()
  const deleteSong = useDeleteGlobalMusicSong()
  const addSong = useAddPersonalMusicSong()
  const removeSong = useRemovePersonalMusicSong()
  const setSongsMembership = useSetPersonalMusicSongs()
  const globalSelection = useGlobalMusicSelection()

  const clearGlobalSelection = () => {
    setSelectedGlobalMembership(new Map())
    setAllMatchingSongsSelected(false)
  }

  const removeSongsFromGlobalSelection = (songIds: Iterable<string>) => {
    setSelectedGlobalMembership((current) => withoutMusicSongs(current, songIds))
  }

  const updateSelectedSongMembership = (songId: string, inLibrary: boolean) => {
    setSelectedGlobalMembership((current) => {
      if (!current.has(songId)) return current
      const next = new Map(current)
      next.set(songId, inLibrary)
      return next
    })
  }

  const selectAllGlobalSongs = async () => {
    const songs = await globalSelection.mutateAsync(debouncedGlobalSearch)
    setSelectedGlobalMembership(new Map(songs.map((song) => [song.id, song.in_library] as const)))
    setAllMatchingSongsSelected(true)
  }

  const updateSelectedMembership = async (inLibrary: boolean) => {
    const songIds = musicSongIdsForMembership(selectedGlobalMembership, inLibrary)
    if (songIds.length === 0) return

    await setSongsMembership.mutateAsync({ songIds, inLibrary })
    removeSongsFromGlobalSelection(songIds)
    setAllMatchingSongsSelected(false)
  }
  const confirmDelete = (song: MusicSong) => {
    modals.openConfirmModal({
      title: `Delete “${song.title}”?`,
      centered: true,
      confirmProps: { color: 'red' },
      labels: { confirm: 'Delete song', cancel: 'Cancel' },
      children: (
        <Text size="sm">
          This permanently removes the audio file for <strong>{song.title}</strong> and every
          user&apos;s personal-library reference. This action cannot be undone.
        </Text>
      ),
      onConfirm: () =>
        deleteSong.mutate(song, {
          onSuccess: () => {
            removeSongsFromGlobalSelection([song.id])
            setAllMatchingSongsSelected(false)
          },
        }),
    })
  }

  const saveSong = async (values: MusicTagUpdate, cover: File | null) => {
    if (!editingSong) return
    await updateSong.mutateAsync({ songId: editingSong.id, tags: values, cover })
    setEditingSong(null)
  }

  return (
    <Stack gap="md">
      <div>
        <Title order={3}>Music</Title>
        <Text size="sm" c="dimmed">
          Manage the shared catalog and choose what appears in your library.
        </Text>
      </div>

      <Tabs
        value={tab}
        color={tab === 'global' ? 'var(--mantine-color-blue-6)' : 'var(--mantine-color-grape-5)'}
        onChange={(value) => setTab(value === 'personal' ? 'personal' : 'global')}
      >
        <Tabs.List mb="md">
          <Tabs.Tab
            value="global"
            leftSection={<IconWorld size={16} />}
            style={{
              borderInlineStart:
                'calc(var(--mantine-spacing-xs) / 3) solid var(--mantine-color-blue-6)',
            }}
          >
            Global library
          </Tabs.Tab>
          <Tabs.Tab
            value="personal"
            leftSection={<IconHeadphones size={16} />}
            style={{
              borderInlineStart:
                'calc(var(--mantine-spacing-xs) / 3) solid var(--mantine-color-grape-5)',
            }}
          >
            My library
          </Tabs.Tab>
        </Tabs.List>

        <Tabs.Panel value="global">
          <Stack gap="sm">
            <Group justify="space-between" align="flex-end" wrap="wrap">
              <TextInput
                value={globalSearch}
                onChange={(event) => {
                  setGlobalSearch(event.currentTarget.value)
                  setGlobalPage(1)
                  clearGlobalSelection()
                }}
                label="Search Global library"
                placeholder="Title, artist, or album"
                leftSection={<IconSearch size={16} />}
                size="sm"
                w={{ base: '100%', sm: '50%' }}
              />
              {permissions.canUpload && <MusicUploadButton />}
            </Group>
            {selectedGlobalMembership.size > 0 && (
              <Paper withBorder p="xs" radius="sm">
                <Stack gap={4}>
                  <Group justify="space-between" gap="xs">
                    <Text size="sm" fw={500}>
                      {selectedGlobalMembership.size}{' '}
                      {selectedGlobalMembership.size === 1 ? 'song' : 'songs'} selected
                    </Text>
                    <Group gap="xs">
                      {selectedSongsToAddCount > 0 && (
                        <Button
                          size="xs"
                          variant="light"
                          leftSection={<IconPlus size={14} />}
                          loading={
                            setSongsMembership.isPending && setSongsMembership.variables?.inLibrary
                          }
                          disabled={setSongsMembership.isPending || globalSelection.isPending}
                          onClick={() => updateSelectedMembership(true)}
                        >
                          Add {selectedSongsToAddCount}
                        </Button>
                      )}
                      {selectedSongsToRemoveCount > 0 && (
                        <Button
                          size="xs"
                          variant="light"
                          color="grape"
                          leftSection={<IconMinus size={14} />}
                          loading={
                            setSongsMembership.isPending && !setSongsMembership.variables?.inLibrary
                          }
                          disabled={setSongsMembership.isPending || globalSelection.isPending}
                          onClick={() => updateSelectedMembership(false)}
                        >
                          Remove {selectedSongsToRemoveCount}
                        </Button>
                      )}
                      <Button
                        size="xs"
                        variant="subtle"
                        color="gray"
                        disabled={setSongsMembership.isPending || globalSelection.isPending}
                        onClick={clearGlobalSelection}
                      >
                        Clear
                      </Button>
                    </Group>
                  </Group>
                  {allMatchingSongsSelected ? (
                    <Text size="xs" c="dimmed">
                      All {selectedGlobalMembership.size}{' '}
                      {debouncedGlobalSearch.trim()
                        ? 'matching songs'
                        : 'songs in the Global library'}{' '}
                      are selected.
                    </Text>
                  ) : (
                    allVisibleSongsSelected &&
                    globalTotal > selectedGlobalMembership.size && (
                      <Group gap={4}>
                        <Text size="xs" c="dimmed">
                          All {globalSongs.length} songs on this page are selected.
                        </Text>
                        <Button
                          size="compact-xs"
                          variant="subtle"
                          loading={globalSelection.isPending}
                          disabled={setSongsMembership.isPending}
                          onClick={selectAllGlobalSongs}
                        >
                          Select all {globalTotal}{' '}
                          {debouncedGlobalSearch.trim() ? 'matching songs' : 'songs'}
                        </Button>
                      </Group>
                    )
                  )}
                </Stack>
              </Paper>
            )}
            <MusicSongTable
              songs={globalSongs}
              total={globalTotal}
              page={globalPage}
              limit={globalLimit}
              loading={globalQuery.isFetching}
              error={globalQuery.error}
              emptyText={
                debouncedGlobalSearch.trim()
                  ? 'No global songs match this search. Try another title, artist, or album.'
                  : permissions.canUpload
                    ? 'The Global library is empty. Upload audio to add the first song.'
                    : 'The Global library is empty. Ask someone with upload permission to add music.'
              }
              onPageChange={setGlobalPage}
              onLimitChange={setGlobalLimit}
              selectedSongs={selectedGlobalSongs}
              onSelectedSongsChange={(songs) => {
                setSelectedGlobalMembership((current) =>
                  mergeVisibleMusicSelection(current, globalSongs, songs)
                )
                setAllMatchingSongsSelected(false)
              }}
              renderActions={(song) => {
                const membershipPending =
                  (addSong.isPending && addSong.variables?.id === song.id) ||
                  (removeSong.isPending && removeSong.variables?.id === song.id)

                return (
                  <>
                    <Tooltip
                      label={song.in_library ? 'Remove from My library' : 'Add to My library'}
                    >
                      <ActionIcon
                        variant={song.in_library ? 'light' : 'subtle'}
                        color={song.in_library ? 'grape' : 'blue'}
                        size="sm"
                        loading={membershipPending}
                        disabled={setSongsMembership.isPending || globalSelection.isPending}
                        aria-label={
                          song.in_library
                            ? `Remove ${song.title} from My library`
                            : `Add ${song.title} to My library`
                        }
                        onClick={() => {
                          const inLibrary = !song.in_library
                          const mutation = inLibrary ? addSong : removeSong
                          mutation.mutate(song, {
                            onSuccess: () => updateSelectedSongMembership(song.id, inLibrary),
                          })
                        }}
                      >
                        {song.in_library ? <IconMinus size={16} /> : <IconPlus size={16} />}
                      </ActionIcon>
                    </Tooltip>
                    {permissions.canEditTags && (
                      <Tooltip label="Edit tags">
                        <ActionIcon
                          variant="subtle"
                          size="sm"
                          aria-label={`Edit tags for ${song.title}`}
                          onClick={() => setEditingSong(song)}
                        >
                          <IconEdit size={16} />
                        </ActionIcon>
                      </Tooltip>
                    )}
                    {permissions.canDeleteGlobal && (
                      <Tooltip label="Delete from Global library">
                        <ActionIcon
                          variant="subtle"
                          color="red"
                          size="sm"
                          loading={deleteSong.isPending && deleteSong.variables?.id === song.id}
                          aria-label={`Delete ${song.title} from Global library`}
                          onClick={() => confirmDelete(song)}
                        >
                          <IconTrash size={16} />
                        </ActionIcon>
                      </Tooltip>
                    )}
                  </>
                )
              }}
            />
          </Stack>
        </Tabs.Panel>

        <Tabs.Panel value="personal">
          <Stack gap="sm">
            <Group justify="space-between" align="flex-end" wrap="wrap">
              <TextInput
                value={personalSearch}
                onChange={(event) => {
                  setPersonalSearch(event.currentTarget.value)
                  setPersonalPage(1)
                }}
                label="Search My library"
                placeholder="Title, artist, or album"
                leftSection={<IconSearch size={16} />}
                size="sm"
                w={{ base: '100%', sm: '50%' }}
              />
              <Button
                size="sm"
                variant="light"
                leftSection={<IconPlus size={16} />}
                onClick={() => setAddModalOpen(true)}
              >
                Add from Global
              </Button>
            </Group>
            <MusicSongTable
              songs={personalQuery.data?.songs ?? []}
              total={personalQuery.data?.total ?? 0}
              page={personalPage}
              limit={personalLimit}
              loading={personalQuery.isFetching}
              error={personalQuery.error}
              emptyText={
                debouncedPersonalSearch.trim()
                  ? 'No songs in My library match this search.'
                  : 'Your library is empty. Use Add from Global to choose songs.'
              }
              onPageChange={setPersonalPage}
              onLimitChange={setPersonalLimit}
              renderActions={(song) => (
                <Tooltip label="Remove from My library">
                  <ActionIcon
                    variant="subtle"
                    color="red"
                    size="sm"
                    loading={removeSong.isPending && removeSong.variables?.id === song.id}
                    aria-label={`Remove ${song.title} from My library`}
                    onClick={() => removeSong.mutate(song)}
                  >
                    <IconTrash size={16} />
                  </ActionIcon>
                </Tooltip>
              )}
            />
          </Stack>
        </Tabs.Panel>
      </Tabs>

      {editingSong && (
        <MusicTagEditor
          key={editingSong.id}
          song={editingSong}
          opened
          saving={updateSong.isPending}
          onClose={() => setEditingSong(null)}
          onSave={saveSong}
        />
      )}
      <AddFromGlobalMusicModal
        opened={addModalOpen}
        knownPersonalSongIds={knownPersonalSongIds}
        onClose={() => setAddModalOpen(false)}
      />
    </Stack>
  )
}
