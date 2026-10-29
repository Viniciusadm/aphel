import { useSyncExternalStore } from 'react'
import { NETWORK_ERROR_CODE, isApiError } from '@/api'

export function isOnline(): boolean {
  return typeof navigator === 'undefined' || navigator.onLine !== false
}

export function isNetworkFailure(error: unknown): boolean {
  return isApiError(error) && (error.status === 0 || error.code === NETWORK_ERROR_CODE)
}

function subscribe(listener: () => void): () => void {
  window.addEventListener('online', listener)
  window.addEventListener('offline', listener)
  return () => {
    window.removeEventListener('online', listener)
    window.removeEventListener('offline', listener)
  }
}

export function useOnline(): boolean {
  return useSyncExternalStore(subscribe, isOnline, () => true)
}
