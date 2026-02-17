import iconMappings from '@client/core/icon-mappings.json'

export interface IconMapping {
  languageIds?: Array<string>
  fileExtensions?: Array<string>
  fileNames?: Array<string>
}

export type IconMappings = Record<string, IconMapping>

const mappings = iconMappings as IconMappings

const fileNameToIcon: Record<string, string> = {}
const extensionToIcon: Record<string, string> = {}

export const DEFAULT_FILE_ICON = '_file'
export const DEFAULT_FOLDER_ICON = '_folder'

for (const [iconName, mapping] of Object.entries(mappings)) {
  mapping.fileNames?.forEach(name => {
    fileNameToIcon[name.toLowerCase()] = iconName
  })
  mapping.fileExtensions?.forEach(ext => {
    extensionToIcon[ext.toLowerCase()] = iconName
  })
}

export function getFileIcon(fileName: string, isDirectory: boolean): string {
  if (isDirectory) return DEFAULT_FOLDER_ICON


  const lowerFileName = fileName.toLowerCase()

  if (lowerFileName in fileNameToIcon) return fileNameToIcon[lowerFileName]

  const extension = getFileExtension(lowerFileName)
  if (extension && extension in extensionToIcon) return extensionToIcon[extension]

  return DEFAULT_FILE_ICON
}

function getFileExtension(fileName: string): string | null {
  const lastDot = fileName.lastIndexOf('.')
  if (lastDot <= 0) return null
  return fileName.substring(lastDot + 1)
}

export function iconExists(iconName: string): boolean {
  return iconName in mappings
}

export function getIconMapping(iconName: string): IconMapping | undefined {
  return mappings[iconName]
}