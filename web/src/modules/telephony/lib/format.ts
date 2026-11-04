export function formatCallDuration(seconds: number | null | undefined): string | null {
  if (seconds === null || seconds === undefined) return null
  const total = Math.max(0, Math.round(seconds))
  const hours = Math.floor(total / 3600)
  const minutes = Math.floor((total % 3600) / 60)
  const rest = String(total % 60).padStart(2, '0')
  if (hours > 0) return `${hours}:${String(minutes).padStart(2, '0')}:${rest}`
  return `${minutes}:${rest}`
}

export function splitDuration(seconds: number | null | undefined): { minutes: string; seconds: string } {
  if (seconds === null || seconds === undefined) return { minutes: '', seconds: '' }
  return { minutes: String(Math.floor(seconds / 60)), seconds: String(seconds % 60) }
}

export function joinDuration(minutes: string, seconds: string): number | null {
  const parsedMinutes = minutes.trim() === '' ? 0 : Number(minutes)
  const parsedSeconds = seconds.trim() === '' ? 0 : Number(seconds)
  if (!Number.isFinite(parsedMinutes) || !Number.isFinite(parsedSeconds)) return null
  if (minutes.trim() === '' && seconds.trim() === '') return null
  return Math.max(0, Math.floor(parsedMinutes) * 60 + Math.floor(parsedSeconds))
}
