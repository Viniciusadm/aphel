export interface FontStack {
  families: string[]
  fallback: string
}

export type TokenValue = string | FontStack
export type TokenMap = Record<string, TokenValue>
export type TokenScope = 'shared' | 'light' | 'dark'

export interface TokenSets {
  shared: TokenMap
  light: TokenMap
  dark: TokenMap
}

export interface FontFile {
  family: string
  weight: number
  style: 'normal' | 'italic'
  format: 'woff2'
  data: string
}

export interface ThemePackage {
  format: 'aphel-theme'
  format_version: 1
  id: string
  name: string
  version: string
  author: string
  description?: string | null
  license: string
  min_aphel_version?: string | null
  tokens: TokenSets
  fonts: FontFile[]
}

export interface ThemeIssue {
  path: string
  message: string
}

export interface ThemeValidation {
  issues: ThemeIssue[]
  warnings: ThemeIssue[]
  token_count: number
  fonts: { family: string; weight: number; style: string; bytes: number }[]
  installed_version: string | null
}

export interface ThemeSummary {
  key: string
  name: string
  version: string
  author: string
  description: string | null
  license: string
  min_aphel_version: string | null
  source: string
  active: boolean
  font_count: number
  tokens: TokenSets
  checksum: string
  created_at: string
  updated_at: string
}

export interface ThemeList {
  active_key: string | null
  themes: ThemeSummary[]
}
