import { isAdmin, useAuth, usePermission } from '@local/hooks/authContext'
import { MUSIC_UPLOAD_FILE_TYPES, useUploader } from '@local/hooks/UploadContext'
import { Button } from '@mantine/core'
import { IconMusicPlus } from '@tabler/icons-react'
import { useRef } from 'react'

export function MusicUploadButton() {
  const inputRef = useRef<HTMLInputElement>(null)
  const { user } = useAuth()
  const hasMusicUploadPermission = usePermission('music_upload')
  const canUploadMusic = isAdmin(user) || hasMusicUploadPermission
  const { addMusicFiles } = useUploader()

  if (!canUploadMusic) return null

  return (
    <>
      <input
        ref={inputRef}
        type="file"
        accept={MUSIC_UPLOAD_FILE_TYPES.join(',')}
        multiple
        hidden
        onChange={(event) => {
          if (event.currentTarget.files) addMusicFiles(event.currentTarget.files)
          event.currentTarget.value = ''
        }}
      />
      <Button
        size="sm"
        variant="light"
        leftSection={<IconMusicPlus size={16} />}
        onClick={() => inputRef.current?.click()}
      >
        Upload audio
      </Button>
    </>
  )
}
