import { isServiceWorkerEnabled } from '../support'
import { pushApi, type CreatePushSubscription } from './api'

export type PushSupport = 'supported' | 'unsupported' | 'ios-needs-install' | 'development'

export function isIos(): boolean {
  const { userAgent, maxTouchPoints } = navigator
  return /iPad|iPhone|iPod/.test(userAgent) || (userAgent.includes('Macintosh') && maxTouchPoints > 1)
}

export function isStandalone(): boolean {
  const iosStandalone = (navigator as Navigator & { standalone?: boolean }).standalone
  return window.matchMedia('(display-mode: standalone)').matches || iosStandalone === true
}

export function pushSupport(): PushSupport {
  if (!import.meta.env.PROD) return 'development'
  if (isIos() && !isStandalone()) return 'ios-needs-install'
  if (!isServiceWorkerEnabled() || !('PushManager' in window) || !('Notification' in window)) return 'unsupported'
  return 'supported'
}

function base64UrlToBytes(value: string): Uint8Array<ArrayBuffer> {
  const padded = `${value}${'='.repeat((4 - (value.length % 4)) % 4)}`.replace(/-/g, '+').replace(/_/g, '/')
  const binary = atob(padded)
  const bytes = new Uint8Array(new ArrayBuffer(binary.length))
  for (let index = 0; index < binary.length; index += 1) bytes[index] = binary.charCodeAt(index)
  return bytes
}

function bytesToBase64Url(buffer: ArrayBuffer | null): string {
  if (!buffer) return ''
  let binary = ''
  for (const byte of new Uint8Array(buffer)) binary += String.fromCharCode(byte)
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}

function toRequest(subscription: PushSubscription): CreatePushSubscription {
  return {
    endpoint: subscription.endpoint,
    keys: {
      p256dh: bytesToBase64Url(subscription.getKey('p256dh')),
      auth: bytesToBase64Url(subscription.getKey('auth')),
    },
  }
}

async function registration(): Promise<ServiceWorkerRegistration | null> {
  if (!isServiceWorkerEnabled()) return null
  return (await navigator.serviceWorker.getRegistration()) ?? null
}

export async function currentSubscription(): Promise<PushSubscription | null> {
  if (pushSupport() !== 'supported') return null
  const current = await registration()
  return (await current?.pushManager.getSubscription()) ?? null
}

function sameKey(subscription: PushSubscription, publicKey: Uint8Array): boolean {
  const current = subscription.options.applicationServerKey
  if (!current) return false
  const bytes = new Uint8Array(current)
  return bytes.length === publicKey.length && bytes.every((byte, index) => byte === publicKey[index])
}

export async function enablePush(): Promise<NotificationPermission> {
  const permission = await Notification.requestPermission()
  if (permission !== 'granted') return permission
  const ready = await navigator.serviceWorker.ready
  const { public_key } = await pushApi.publicKey()
  const applicationServerKey = base64UrlToBytes(public_key)
  let subscription = await ready.pushManager.getSubscription()
  if (subscription && !sameKey(subscription, applicationServerKey)) {
    await subscription.unsubscribe()
    subscription = null
  }
  subscription ??= await ready.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey })
  await pushApi.subscribe(toRequest(subscription))
  return permission
}

export async function disablePush(): Promise<void> {
  const subscription = await currentSubscription()
  if (!subscription) return
  await pushApi.unsubscribe(subscription.endpoint).catch(() => undefined)
  await subscription.unsubscribe()
}

export async function syncPushSubscription(): Promise<void> {
  if (pushSupport() !== 'supported' || Notification.permission !== 'granted') return
  const subscription = await currentSubscription()
  if (subscription) await pushApi.subscribe(toRequest(subscription))
}

export async function forgetPushOnThisDevice(): Promise<void> {
  const subscription = await currentSubscription()
  await subscription?.unsubscribe()
}
