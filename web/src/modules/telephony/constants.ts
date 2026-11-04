export const CALLS_PATH = '/telephony'

export function callSessionPath(sessionId: string): string {
  return `${CALLS_PATH}/sessions/${encodeURIComponent(sessionId)}`
}
