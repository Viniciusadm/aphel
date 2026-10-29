import { useMatches } from 'react-router'
import type { SettingsArea } from '@/kernel/modules/settings'

export interface SettingsAreaHandle {
  settingsArea: SettingsArea
}

function isSettingsAreaHandle(handle: unknown): handle is SettingsAreaHandle {
  return typeof handle === 'object' && handle !== null && 'settingsArea' in handle
}

export function useSettingsArea(): SettingsArea | null {
  const matches = useMatches()
  for (let index = matches.length - 1; index >= 0; index -= 1) {
    const handle = matches[index].handle
    if (isSettingsAreaHandle(handle)) return handle.settingsArea
  }
  return null
}
