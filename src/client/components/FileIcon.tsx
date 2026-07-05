import iconMappings from '@local/icon-mappings.json'
import { useMantineColorScheme } from '@mantine/core'

interface IconMapping {
  languageIds?: Array<string>
  fileExtensions?: Array<string>
  fileNames?: Array<string>
}

type IconMappings = Record<string, IconMapping>

const mappings = iconMappings as IconMappings

const fileNameToIcon: Record<string, string> = {}
const extensionToIcon: Record<string, string> = {}

const DEFAULT_FILE_ICON = '_file'
const DEFAULT_FOLDER_ICON = '_folder'

for (const [iconName, mapping] of Object.entries(mappings)) {
  mapping.fileNames?.forEach((name) => {
    fileNameToIcon[name.toLowerCase()] = iconName
  })
  mapping.fileExtensions?.forEach((ext) => {
    extensionToIcon[ext.toLowerCase()] = iconName
  })
}

export function getIconName(fileName: string, isDirectory: boolean): string {
  if (isDirectory) return DEFAULT_FOLDER_ICON

  const lowerFileName = fileName.toLowerCase()
  if (lowerFileName in fileNameToIcon) return fileNameToIcon[lowerFileName]

  const lastDot = lowerFileName.lastIndexOf('.')
  if (lastDot > 0) {
    const ext = lowerFileName.substring(lastDot + 1)
    if (ext in extensionToIcon) return extensionToIcon[ext]
  }

  return DEFAULT_FILE_ICON
}

export interface FileIconProps {
  fileName: string
  isDirectory: boolean
  size?: number
}

export function FileIcon({ fileName, isDirectory, size = 22 }: FileIconProps) {
  const { colorScheme } = useMantineColorScheme()
  const theme = colorScheme === 'dark' ? 'macchiato' : 'latte'
  const iconName = getIconName(fileName, isDirectory)

  const iconUrl = `/icons/${theme}/${iconName}.svg`

  return (
    <img
      src={iconUrl}
      alt={iconName}
      width={size}
      height={size}
      style={{ display: 'block', flexShrink: 0 }}
      draggable={false}
    />
  )
}
