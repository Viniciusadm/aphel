import { useEffect, useRef } from 'react'
import { API_BASE_URL } from '@/api'

export interface PluginChange {
  seq: number
  event_type: string
  record_type: string | null
  record_id: string | null
  fields: string[]
  occurred_at: string
}

export interface ChangeFeedOptions {
  enabled?: boolean
  since?: number
  onReset?: () => void
}

export function useChangeFeed(
  pluginKey: string,
  onChange: (change: PluginChange) => void,
  { enabled = true, since = 0, onReset }: ChangeFeedOptions = {},
) {
  const handlers = useRef({ onChange, onReset })
  useEffect(() => {
    handlers.current = { onChange, onReset }
  })
  useEffect(() => {
    if (!enabled || typeof EventSource === 'undefined') return
    const url = `${API_BASE_URL}/v1/wasm-runtime/${encodeURIComponent(pluginKey)}/changes/stream?since=${since}`
    const source = new EventSource(url, { withCredentials: true })
    const handleChange = (event: MessageEvent<string>) => {
      try {
        handlers.current.onChange(JSON.parse(event.data) as PluginChange)
      } catch {
        handlers.current.onReset?.()
      }
    }
    const handleReset = () => handlers.current.onReset?.()
    source.addEventListener('change', handleChange)
    source.addEventListener('reset', handleReset)
    return () => {
      source.removeEventListener('change', handleChange)
      source.removeEventListener('reset', handleReset)
      source.close()
    }
  }, [pluginKey, enabled, since])
}
