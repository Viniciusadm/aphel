import { createContext, useContext } from 'react'

export const ImageUsageContext = createContext<ReadonlySet<string>>(new Set())

export function useImagesInUse(): ReadonlySet<string> {
  return useContext(ImageUsageContext)
}

export function collectImageIds(value: unknown, found: Set<string> = new Set()): Set<string> {
  if (Array.isArray(value)) {
    for (const item of value) collectImageIds(item, found)
  } else if (value && typeof value === 'object') {
    for (const [key, field] of Object.entries(value)) {
      if (key === 'image' && typeof field === 'string' && field) found.add(field)
      else collectImageIds(field, found)
    }
  }
  return found
}

export function imageIdsOfJson(text: string | null | undefined, found: Set<string>): Set<string> {
  if (!text) return found
  try {
    return collectImageIds(JSON.parse(text), found)
  } catch {
    return found
  }
}
