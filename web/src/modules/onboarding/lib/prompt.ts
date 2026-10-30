const PROMPT_KEY = 'aphel.onboarding.prompted'

function key(userId?: string | null): string {
  return userId ? `${PROMPT_KEY}:${userId}` : PROMPT_KEY
}

export function wasPrompted(userId?: string | null): boolean {
  try {
    return window.sessionStorage.getItem(key(userId)) === 'true'
  } catch {
    return false
  }
}

export function markPrompted(userId?: string | null): void {
  try {
    window.sessionStorage.setItem(key(userId), 'true')
  } catch {
    return
  }
}
