import type { MusicSong } from '@local/types/music'
import { Alert, Box, Group, Text, VisuallyHidden } from '@mantine/core'
import { IconAlertCircle } from '@tabler/icons-react'
import type { DataTableColumn } from 'mantine-datatable'
import { DataTable } from 'mantine-datatable'
import type { ReactNode } from 'react'

interface MusicSongTableProps {
  songs: Array<MusicSong>
  total: number
  page: number
  limit: number
  loading: boolean
  error: Error | null
  emptyText: string
  onPageChange: (page: number) => void
  onLimitChange: (limit: number) => void
  renderActions?: (song: MusicSong) => ReactNode
  selectedSongs?: Array<MusicSong>
  onSelectedSongsChange?: (songs: Array<MusicSong>) => void
}

export function formatMusicDuration(seconds: number | null): string {
  if (seconds === null || !Number.isFinite(seconds)) return '-'

  const wholeSeconds = Math.max(0, Math.round(seconds))
  const hours = Math.floor(wholeSeconds / 3600)
  const minutes = Math.floor((wholeSeconds % 3600) / 60)
  const remainder = wholeSeconds % 60
  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, '0')}:${String(remainder).padStart(2, '0')}`
  }
  return `${minutes}:${String(remainder).padStart(2, '0')}`
}

export function MusicSongTable({
  songs,
  total,
  page,
  limit,
  loading,
  error,
  emptyText,
  onPageChange,
  onLimitChange,
  renderActions,
  selectedSongs,
  onSelectedSongsChange,
}: MusicSongTableProps) {
  const columns: Array<DataTableColumn<MusicSong>> = [
    {
      accessor: 'title',
      title: 'Title',
      render: (song) => (
        <Text size="sm" fw={500} lineClamp={1} title={song.title}>
          {song.title}
        </Text>
      ),
    },
    {
      accessor: 'artist',
      title: 'Artist',
      render: (song) => (
        <Text size="sm" lineClamp={1} title={song.artist}>
          {song.artist}
        </Text>
      ),
    },
    {
      accessor: 'album',
      title: 'Album',
      visibleMediaQuery: (theme) => `(min-width: ${theme.breakpoints.sm})`,
      render: (song) => (
        <Text size="sm" lineClamp={1} title={song.album}>
          {song.album}
        </Text>
      ),
    },
    {
      accessor: 'genre',
      title: 'Genre',
      visibleMediaQuery: (theme) => `(min-width: ${theme.breakpoints.md})`,
      render: (song) => (
        <Text size="sm" c={song.genre ? undefined : 'dimmed'} lineClamp={1}>
          {song.genre || '-'}
        </Text>
      ),
    },
    {
      accessor: 'duration_seconds',
      title: 'Duration',
      textAlign: 'right',
      width: 96,
      visibleMediaQuery: (theme) => `(min-width: ${theme.breakpoints.xs})`,
      render: (song) => (
        <Text size="sm" ff="monospace" ta="right">
          {formatMusicDuration(song.duration_seconds)}
        </Text>
      ),
    },
  ]

  if (renderActions) {
    columns.push({
      accessor: 'actions',
      title: <VisuallyHidden>Actions</VisuallyHidden>,
      width: 120,
      render: (song) => (
        <Group justify="flex-end" gap={4} wrap="nowrap">
          {renderActions(song)}
        </Group>
      ),
    })
  }

  return (
    <Box>
      {error && (
        <Alert
          icon={<IconAlertCircle size={16} />}
          color="red"
          mb="sm"
          title="Songs could not load"
        >
          {error.message || 'Try again.'}
        </Alert>
      )}
      <DataTable<MusicSong>
        withTableBorder
        borderRadius="sm"
        highlightOnHover
        verticalSpacing="xs"
        horizontalSpacing="sm"
        fetching={loading}
        columns={columns}
        records={songs}
        selectedRecords={selectedSongs}
        onSelectedRecordsChange={onSelectedSongsChange}
        selectionCheckboxProps={{ size: 'xs' }}
        allRecordsSelectionCheckboxProps={{ 'aria-label': 'Select all visible songs' }}
        getRecordSelectionCheckboxProps={(song) => ({
          'aria-label': `Select ${song.title}`,
        })}
        noRecordsText={emptyText}
        page={page}
        onPageChange={onPageChange}
        totalRecords={total}
        recordsPerPage={limit}
        recordsPerPageOptions={[10, 25, 50, 100]}
        onRecordsPerPageChange={(value) => {
          onLimitChange(value)
          onPageChange(1)
        }}
      />
    </Box>
  )
}
