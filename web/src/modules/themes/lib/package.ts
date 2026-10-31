import { emptyTokenSets } from './tokens'
import type { FontFile, ThemePackage } from './types'

export const THEME_FORMAT = 'aphel-theme'
export const THEME_KEY_PATTERN = /^[a-z0-9]+(?:[._-][a-z0-9]+)*$/
export const MAX_FONT_BYTES = 400 * 1024
export const MAX_FONTS = 6

export function newPackage(): ThemePackage {
  return {
    format: THEME_FORMAT,
    format_version: 1,
    id: '',
    name: '',
    version: '1.0.0',
    author: '',
    description: null,
    license: 'MIT',
    min_aphel_version: null,
    tokens: emptyTokenSets(),
    fonts: [],
  }
}

export function slugify(value: string): string {
  return value
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 64)
}

export async function readPackageFile(file: File): Promise<ThemePackage> {
  const parsed: unknown = JSON.parse(await file.text())
  if (typeof parsed !== 'object' || parsed === null) throw new Error('not an object')
  return parsed as ThemePackage
}

export function downloadPackage(themePackage: ThemePackage): void {
  const blob = new Blob([JSON.stringify(themePackage, null, 2)], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = `${themePackage.id}-${themePackage.version}.json`
  link.click()
  URL.revokeObjectURL(url)
}

export async function fontFromFile(file: File, family: string, weight: number, style: FontFile['style']): Promise<FontFile> {
  const bytes = new Uint8Array(await file.arrayBuffer())
  let binary = ''
  for (const byte of bytes) binary += String.fromCharCode(byte)
  return { family, weight, style, format: 'woff2', data: btoa(binary) }
}

export function base64ToBuffer(data: string): ArrayBuffer {
  const binary = atob(data)
  const bytes = new Uint8Array(binary.length)
  for (let index = 0; index < binary.length; index += 1) bytes[index] = binary.charCodeAt(index)
  return bytes.buffer
}

export function normalizePackage(themePackage: ThemePackage): ThemePackage {
  return {
    ...themePackage,
    description: themePackage.description?.trim() || null,
    min_aphel_version: themePackage.min_aphel_version?.trim() || null,
  }
}

export const THEME_STYLESHEET_ID = 'aphel-theme'

export function reloadThemeStylesheet(): void {
  const link = document.getElementById(THEME_STYLESHEET_ID)
  if (!(link instanceof HTMLLinkElement)) return
  const url = new URL(link.href)
  url.searchParams.set('v', String(Date.now()))
  link.href = url.toString()
}
