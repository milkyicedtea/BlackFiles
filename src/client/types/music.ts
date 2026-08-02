export interface MusicSong {
  id: string
  file_path: string
  title: string
  artist: string
  album: string
  album_artist: string | null
  genre: string | null
  year: number | null
  track_number: number | null
  disc_number: number | null
  duration_seconds: number | null
  size_bytes: number
  format: string | null
  bitrate_kbps: number | null
  has_cover_art: boolean
  in_library: boolean
  created_at: string
  updated_at: string
}

export interface MusicSongListResponse {
  songs: Array<MusicSong>
  total: number
  page: number
  limit: number
}

export interface MusicSongSelection {
  id: string
  in_library: boolean
}

export interface MusicSongSelectionResponse {
  songs: Array<MusicSongSelection>
}

export interface MusicListParams {
  page: number
  limit: number
  search: string
}

export interface MusicTagUpdate {
  title: string
  artist: string
  album: string
  album_artist: string
  genre: string
  year: number | null
  track_number: number | null
  disc_number: number | null
}

export interface MusicTagFormValues
  extends Omit<MusicTagUpdate, 'year' | 'track_number' | 'disc_number'> {
  year: number | string
  track_number: number | string
  disc_number: number | string
}

export interface MusicTagMutation {
  songId: string
  tags: MusicTagUpdate
}
