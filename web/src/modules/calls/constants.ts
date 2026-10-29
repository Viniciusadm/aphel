export const CALLS_PATH = '/calls'

export function callSessionPath(sessionId: string): string {
  return `${CALLS_PATH}/sessions/${encodeURIComponent(sessionId)}`
}
