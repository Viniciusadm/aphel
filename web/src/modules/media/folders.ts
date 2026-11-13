import type { MediaFolder } from '@/sdk/media/api'

export interface FolderNode extends MediaFolder {
  depth: number
}

export function flattenFolders(folders: MediaFolder[], exclude?: string): FolderNode[] {
  const children = new Map<string | null, MediaFolder[]>()
  for (const folder of folders) {
    const key = folder.parent_id && folders.some((item) => item.id === folder.parent_id) ? folder.parent_id : null
    children.set(key, [...(children.get(key) ?? []), folder])
  }
  const out: FolderNode[] = []
  const walk = (parent: string | null, depth: number) => {
    for (const folder of (children.get(parent) ?? []).sort((a, b) => a.name.localeCompare(b.name))) {
      if (folder.id === exclude) continue
      out.push({ ...folder, depth })
      walk(folder.id, depth + 1)
    }
  }
  walk(null, 0)
  return out
}

export function folderPath(folders: MediaFolder[], id: string): string[] {
  const names: string[] = []
  let current = folders.find((folder) => folder.id === id)
  while (current && names.length < 20) {
    names.unshift(current.name)
    const parent: string | null = current.parent_id
    current = parent ? folders.find((folder) => folder.id === parent) : undefined
  }
  return names
}
