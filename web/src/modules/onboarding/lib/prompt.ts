const PROMPT_KEY = 'aphel.onboarding.prompted'

export function wasPrompted(): boolean {
  try {
    return window.sessionStorage.getItem(PROMPT_KEY) === 'true'
  } catch {
    return false
  }
}

export function markPrompted(): void {
  try {
    window.sessionStorage.setItem(PROMPT_KEY, 'true')
  } catch {
    return
  }
}
