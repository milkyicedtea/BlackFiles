import iconMappings from '@client/core/icon-mappings.json'

export interface IconMapping {
  languageIds?: Array<string>
  fileExtensions?: Array<string>
  fileNames?: Array<string>
}

export type IconMappings = Record<string, IconMapping>

const mappings = iconMappings as IconMappings

export const DEFAULT_FILE_ICON = '_file'
export const DEFAULT_FOLDER_ICON = '_folder'

export function getFileIcon(fileName: string, isDirectory: boolean): string {
  if (isDirectory) {
    return DEFAULT_FOLDER_ICON
  }

  const lowerFileName = fileName.toLowerCase()

  for (const [iconName, mapping] of Object.entries(mappings)) {
    if (mapping.fileNames) {
      for (const name of mapping.fileNames) {
        if (lowerFileName === name.toLowerCase()) {
          return iconName
        }
      }
    }
  }

  const extension = getFileExtension(lowerFileName)
  if (extension) {
    for (const [iconName, mapping] of Object.entries(mappings)) {
      if (mapping.fileExtensions) {
        if (mapping.fileExtensions.includes(extension)) {
          return iconName
        }
      }
    }
  }

  return DEFAULT_FILE_ICON
}

function getFileExtension(fileName: string): string | null {
  const lastDot = fileName.lastIndexOf('.')
  if (lastDot === -1 || lastDot === 0) {
    return null
  }
  return fileName.substring(lastDot + 1)
}

export function iconExists(iconName: string): boolean {
  return iconName in mappings
}

export function getIconMapping(iconName: string): IconMapping | undefined {
  return mappings[iconName]
}