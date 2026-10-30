import type { BusinessAnswers } from './business'
import { parseBusiness } from './business'
import type { ModuleChoice } from './modules'

const DRAFT_KEY = 'aphel.setup.draft'

export interface SetupDraft {
  step: string
  values: {
    organizationName: string
    timezone: string
    currency: string
    locale: string
    adminName: string
    adminEmail: string
  }
  business: BusinessAnswers
  choice: ModuleChoice
  preselectedFor: string | null
}

export function readDraft(): Partial<SetupDraft> | null {
  try {
    const raw = window.sessionStorage.getItem(DRAFT_KEY)
    if (!raw) return null
    const parsed = JSON.parse(raw) as Partial<SetupDraft>
    return { ...parsed, business: parseBusiness(parsed.business) }
  } catch {
    return null
  }
}

export function writeDraft(draft: SetupDraft): void {
  try {
    window.sessionStorage.setItem(DRAFT_KEY, JSON.stringify(draft))
  } catch {
    return
  }
}

export function clearDraft(): void {
  try {
    window.sessionStorage.removeItem(DRAFT_KEY)
  } catch {
    return
  }
}
