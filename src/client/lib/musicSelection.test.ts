import {
  mergeVisibleMusicSelection,
  musicSongIdsForMembership,
  withoutMusicSongs,
} from './musicSelection'

function expectEqual(actual: unknown, expected: unknown, message: string) {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(
      `${message}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`
    )
  }
}

const current = new Map([
  ['off-page', false],
  ['visible-added', true],
  ['visible-removed', false],
])
const visibleSongs = [
  { id: 'visible-added', in_library: true },
  { id: 'visible-removed', in_library: false },
]
const nextPageSelection = mergeVisibleMusicSelection(current, visibleSongs, [visibleSongs[0]])
expectEqual(
  [...nextPageSelection.entries()],
  [
    ['off-page', false],
    ['visible-added', true],
  ],
  'changing one page must preserve selections from other pages'
)

let bulkSelection = new Map([
  ['already-added-1', true],
  ['not-added-1', false],
  ['already-added-2', true],
  ['not-added-2', false],
])
const removedSongIds = musicSongIdsForMembership(bulkSelection, false)
bulkSelection = withoutMusicSongs(bulkSelection, removedSongIds)
expectEqual(
  removedSongIds,
  ['already-added-1', 'already-added-2'],
  'remove must target only songs already in the library'
)
expectEqual(
  [...bulkSelection.entries()],
  [
    ['not-added-1', false],
    ['not-added-2', false],
  ],
  'remove must leave addable songs selected'
)

const addedSongIds = musicSongIdsForMembership(bulkSelection, true)
bulkSelection = withoutMusicSongs(bulkSelection, addedSongIds)
expectEqual(
  addedSongIds,
  ['not-added-1', 'not-added-2'],
  'add must target only songs outside the library'
)
expectEqual(bulkSelection.size, 0, 'add must clear the processed remainder')
