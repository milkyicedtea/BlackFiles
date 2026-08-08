import { api } from '@local/hooks/api'
import { isAdmin, useAuth, usePermission } from '@local/hooks/authContext'
import type {
  MusicListParams,
  MusicSong,
  MusicSongListResponse,
  MusicSongMutation,
  MusicSongSelectionResponse,
} from '@local/types/music'
import { notifications } from '@mantine/notifications'
import {
  keepPreviousData,
  type QueryClient,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query'
import { useMemo } from 'react'

export const musicQueryKeys = {
  global: {
    all: ['music', 'global'] as const,
    list: (params: MusicListParams) => ['music', 'global', 'list', params] as const,
  },
  personal: {
    all: ['music', 'personal'] as const,
    list: (params: MusicListParams) => ['music', 'personal', 'list', params] as const,
  },
} as const

async function invalidateMusicQueries(queryClient: QueryClient): Promise<void> {
  await Promise.all([
    queryClient.invalidateQueries({ queryKey: musicQueryKeys.global.all }),
    queryClient.invalidateQueries({ queryKey: musicQueryKeys.personal.all }),
  ])
}

function cleanSearch(search: string): string | undefined {
  const value = search.trim()
  return value || undefined
}

interface SetPersonalMusicSongsMutation {
  songIds: Array<string>
  inLibrary: boolean
}

async function setPersonalMusicSong(
  song: MusicSong,
  inLibrary: boolean,
  silent = false
): Promise<void> {
  if (inLibrary) {
    await api.post<void>(`/music/library/${song.id}`, undefined, {
      _silent: silent,
      _successMessage: silent ? undefined : `Added “${song.title}” to your library`,
    })
    return
  }

  await api.delete<void>(`/music/library/${song.id}`, {
    _silent: silent,
    _successMessage: silent ? undefined : `Removed “${song.title}” from your library`,
  })
}

export function useGlobalMusicSongs(params: MusicListParams, enabled = true) {
  const search = cleanSearch(params.search)
  return useQuery({
    queryKey: musicQueryKeys.global.list(params),
    enabled,
    placeholderData: keepPreviousData,
    queryFn: async () => {
      const { data } = await api.get<MusicSongListResponse>('/music/songs', {
        params: {
          page: params.page,
          limit: params.limit,
          // The global endpoint passes this value directly to ILIKE.
          search: search ? `%${search}%` : undefined,
        },
      })
      return data
    },
  })
}

export function useGlobalMusicSelection() {
  return useMutation({
    mutationFn: async (search: string) => {
      const cleanedSearch = cleanSearch(search)
      const { data } = await api.get<MusicSongSelectionResponse>('/music/songs/selection', {
        params: {
          search: cleanedSearch ? `%${cleanedSearch}%` : undefined,
        },
      })
      return data.songs
    },
  })
}

export function usePersonalMusicSongs(params: MusicListParams) {
  const search = cleanSearch(params.search)
  return useQuery({
    queryKey: musicQueryKeys.personal.list(params),
    placeholderData: keepPreviousData,
    queryFn: async () => {
      const { data } = await api.get<MusicSongListResponse>('/music/library', {
        params: {
          page: params.page,
          limit: params.limit,
          // The personal endpoint adds its own ILIKE wildcards.
          search,
        },
      })
      return data
    },
  })
}

export function useMusicPermissions() {
  const { user } = useAuth()
  const hasUploadPermission = usePermission('music_upload')
  const hasEditPermission = usePermission('music_edit_tags')
  const hasDeletePermission = usePermission('music_delete')
  const admin = isAdmin(user)
  return {
    canUpload: admin || hasUploadPermission,
    canEditTags: admin || hasEditPermission,
    canDeleteGlobal: admin || hasDeletePermission,
  }
}

export function useKnownPersonalSongIds(currentSongs: Array<MusicSong>) {
  const queryClient = useQueryClient()

  return useMemo(() => {
    const ids = new Set(currentSongs.map((song) => song.id))
    const cachedPages = queryClient.getQueriesData<MusicSongListResponse>({
      queryKey: musicQueryKeys.personal.all,
    })
    for (const [, page] of cachedPages) {
      for (const song of page?.songs ?? []) ids.add(song.id)
    }
    return ids
  }, [currentSongs, queryClient])
}

export function useUpdateMusicSong() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: async ({ songId, tags, cover }: MusicSongMutation) => {
      const tagsResponse = await api.put<MusicSong>(`/music/songs/${songId}/tags`, tags)
      if (!cover) return tagsResponse.data

      const coverResponse = await api.put<MusicSong>(`/music/songs/${songId}/cover`, cover, {
        headers: { 'Content-Type': cover.type || 'application/octet-stream' },
      })
      return coverResponse.data
    },
    onSuccess: async () => {
      notifications.show({
        title: 'Success',
        message: 'Song updated',
        color: 'green',
      })
      await invalidateMusicQueries(queryClient)
    },
  })
}

export function useDeleteGlobalMusicSong() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (song: MusicSong) =>
      api.delete<void>(`/music/songs/${song.id}`, {
        _successMessage: `Deleted “${song.title}”`,
      }),
    onSuccess: () => invalidateMusicQueries(queryClient),
  })
}

export function useAddPersonalMusicSong() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (song: MusicSong) => setPersonalMusicSong(song, true),
    onSuccess: () => invalidateMusicQueries(queryClient),
  })
}

export function useRemovePersonalMusicSong() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (song: MusicSong) => setPersonalMusicSong(song, false),
    onSuccess: () => invalidateMusicQueries(queryClient),
  })
}

export function useSetPersonalMusicSongs() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({ songIds, inLibrary }: SetPersonalMusicSongsMutation) => {
      await api.put<void>('/music/library', {
        song_ids: songIds,
        in_library: inLibrary,
      })
      return { songIds, inLibrary }
    },
    onSuccess: ({ songIds, inLibrary }) => {
      notifications.show({
        title: 'Library updated',
        message: `${songIds.length} ${songIds.length === 1 ? 'song' : 'songs'} ${
          inLibrary ? 'added to' : 'removed from'
        } your library`,
        color: 'green',
      })
    },
    onSettled: () => invalidateMusicQueries(queryClient),
  })
}
