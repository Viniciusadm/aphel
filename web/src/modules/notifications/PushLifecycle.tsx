import { useEffect } from 'react'
import { registerSignOutCleanup } from '@/pwa/sign-out'
import { forgetPushOnThisDevice, syncPushSubscription } from './push/push'
export function PushLifecycle() { useEffect(() => { const unregister = registerSignOutCleanup(forgetPushOnThisDevice); void syncPushSubscription().catch(() => undefined); return unregister }, []); return null }
