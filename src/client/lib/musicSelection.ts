export interface SelectableMusicSong {
  id: string
  in_library: boolean
}

export function mergeVisibleMusicSelection(
  current: ReadonlyMap<string, boolean>,
  visibleSongs: ReadonlyArray<SelectableMusicSong>,
  selectedSongs: ReadonlyArray<SelectableMusicSong>
): Map<string, boolean> {
  const next = new Map(current)
  for (const song of visibleSongs) next.delete(song.id)
  for (const song of selectedSongs) next.set(song.id, song.in_library)
  return next
}

export function musicSongIdsForMembership(
  selection: ReadonlyMap<string, boolean>,
  inLibrary: boolean
): Array<string> {
  const songIds: Array<string> = []
  for (const [songId, currentMembership] of selection) {
    if (currentMembership !== inLibrary) songIds.push(songId)
  }
  return songIds
}

export function withoutMusicSongs(
  selection: ReadonlyMap<string, boolean>,
  songIds: Iterable<string>
): Map<string, boolean> {
  const next = new Map(selection)
  for (const songId of songIds) next.delete(songId)
  return next
}
