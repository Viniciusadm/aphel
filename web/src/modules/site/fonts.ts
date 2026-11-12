import { useQuery } from '@tanstack/react-query'
import { designApi } from './designApi'

export interface FontFace {
  file: string
  weight: number
  style: 'normal' | 'italic'
  name?: string
}

export interface FontContent {
  family: string
  fallback: 'sans-serif' | 'serif' | 'monospace'
  faces: FontFace[]
}

export const fontKeys = { list: ['site', 'designs', 'fonts'] as const }

export function parseFont(text: string | null | undefined): FontContent {
  try {
    const parsed = JSON.parse(text ?? '{}') as Partial<FontContent>
    return { family: parsed.family ?? '', fallback: parsed.fallback ?? 'sans-serif', faces: parsed.faces ?? [] }
  } catch {
    return { family: '', fallback: 'sans-serif', faces: [] }
  }
}

export function useFonts() {
  return useQuery({
    queryKey: fontKeys.list,
    queryFn: async () => (await designApi.records()).filter((record) => record.kind === 'font'),
  })
}
