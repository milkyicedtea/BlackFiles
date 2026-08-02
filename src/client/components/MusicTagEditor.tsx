import type { MusicSong, MusicTagFormValues, MusicTagUpdate } from '@local/types/music'
import { Button, Grid, Group, Modal, NumberInput, Stack, TextInput } from '@mantine/core'
import { useForm } from '@mantine/form'

interface MusicTagEditorProps {
  song: MusicSong
  opened: boolean
  saving: boolean
  onClose: () => void
  onSave: (values: MusicTagUpdate) => Promise<void>
}

const OPTIONAL_NUMBER_ERROR = 'Enter a whole number or leave this blank'

function validateOptionalInteger(value: number | string, minimum: number, maximum: number) {
  if (value === '') return null
  if (typeof value !== 'number' || !Number.isInteger(value)) return OPTIONAL_NUMBER_ERROR
  if (value < minimum || value > maximum) return `Enter a number from ${minimum} to ${maximum}`
  return null
}

export function MusicTagEditor({ song, opened, saving, onClose, onSave }: MusicTagEditorProps) {
  const form = useForm<MusicTagFormValues>({
    mode: 'uncontrolled',
    initialValues: {
      title: song.title,
      artist: song.artist,
      album: song.album,
      album_artist: song.album_artist ?? '',
      genre: song.genre ?? '',
      year: song.year ?? '',
      track_number: song.track_number ?? '',
      disc_number: song.disc_number ?? '',
    },
    validateInputOnBlur: true,
    validate: {
      title: (value) => (value.trim() ? null : 'Title is required'),
      artist: (value) => (value.trim() ? null : 'Artist is required'),
      album: (value) => (value.trim() ? null : 'Album is required'),
      year: (value) => validateOptionalInteger(value, 0, 9999),
      track_number: (value) => validateOptionalInteger(value, 1, 32767),
      disc_number: (value) => validateOptionalInteger(value, 1, 32767),
    },
  })

  return (
    <Modal opened={opened} onClose={onClose} title={`Edit tags — ${song.title}`} size="lg">
      <form
        onSubmit={form.onSubmit(async (values) => {
          await onSave({
            title: values.title.trim(),
            artist: values.artist.trim(),
            album: values.album.trim(),
            album_artist: values.album_artist.trim(),
            genre: values.genre.trim(),
            year: values.year === '' ? null : Number(values.year),
            track_number: values.track_number === '' ? null : Number(values.track_number),
            disc_number: values.disc_number === '' ? null : Number(values.disc_number),
          })
        })}
      >
        <Stack gap="sm">
          <TextInput
            key={form.key('title')}
            label="Title"
            required
            size="sm"
            {...form.getInputProps('title')}
          />
          <Grid gutter="sm">
            <Grid.Col span={{ base: 12, sm: 6 }}>
              <TextInput
                key={form.key('artist')}
                label="Artist"
                required
                size="sm"
                {...form.getInputProps('artist')}
              />
            </Grid.Col>
            <Grid.Col span={{ base: 12, sm: 6 }}>
              <TextInput
                key={form.key('album_artist')}
                label="Album artist"
                size="sm"
                {...form.getInputProps('album_artist')}
              />
            </Grid.Col>
          </Grid>
          <Grid gutter="sm">
            <Grid.Col span={{ base: 12, sm: 6 }}>
              <TextInput
                key={form.key('album')}
                label="Album"
                required
                size="sm"
                {...form.getInputProps('album')}
              />
            </Grid.Col>
            <Grid.Col span={{ base: 12, sm: 6 }}>
              <TextInput
                key={form.key('genre')}
                label="Genre"
                size="sm"
                {...form.getInputProps('genre')}
              />
            </Grid.Col>
          </Grid>
          <Grid gutter="sm">
            <Grid.Col span={{ base: 12, xs: 4 }}>
              <NumberInput
                key={form.key('year')}
                label="Year"
                min={0}
                max={9999}
                allowDecimal={false}
                allowNegative={false}
                size="sm"
                {...form.getInputProps('year')}
              />
            </Grid.Col>
            <Grid.Col span={{ base: 6, xs: 4 }}>
              <NumberInput
                key={form.key('track_number')}
                label="Track"
                min={1}
                max={32767}
                allowDecimal={false}
                allowNegative={false}
                size="sm"
                {...form.getInputProps('track_number')}
              />
            </Grid.Col>
            <Grid.Col span={{ base: 6, xs: 4 }}>
              <NumberInput
                key={form.key('disc_number')}
                label="Disc"
                min={1}
                max={32767}
                allowDecimal={false}
                allowNegative={false}
                size="sm"
                {...form.getInputProps('disc_number')}
              />
            </Grid.Col>
          </Grid>
          <Group justify="flex-end" mt="xs">
            <Button type="button" variant="default" onClick={onClose} disabled={saving}>
              Cancel
            </Button>
            <Button type="submit" loading={saving}>
              Save changes
            </Button>
          </Group>
        </Stack>
      </form>
    </Modal>
  )
}
