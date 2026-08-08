import type { MusicSong } from '@local/types/music'

export function musicCoverUrl(song: MusicSong, size: number): string | undefined {
  if (!song.has_cover_art) return undefined

  const params = new URLSearchParams({
    size: String(size),
    v: song.updated_at,
  })
  return `/api/music/songs/${song.id}/cover?${params}`
}
