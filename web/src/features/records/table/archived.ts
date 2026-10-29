import type { ArchivedMode } from '../types'

export const ARCHIVED_PARAM = 'archived'

export function parseArchivedParam(value: string | null | undefined): ArchivedMode {
  return value === 'include' || value === 'only' ? value : 'exclude'
}
