import { api } from '@/api'
import type { components } from '@/api/schema'

export type PushPublicKey = components['schemas']['PushPublicKeyResponse']
export type CreatePushSubscription = components['schemas']['CreatePushSubscriptionRequest']

export const pushApi = {
  publicKey: () => api.get<PushPublicKey>('/v1/notifications/push/public-key'),
  subscribe: (subscription: CreatePushSubscription) =>
    api.post<void>('/v1/notifications/push/subscriptions', subscription),
  unsubscribe: (endpoint: string) => api.post<void>('/v1/notifications/push/subscriptions/unsubscribe', { endpoint }),
}
