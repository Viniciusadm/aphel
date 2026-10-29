import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'

type Schemas = components['schemas']

export type WebhookEndpoint = Schemas['WebhookEndpointResponse']
export type WebhookEndpointWithSecret = Schemas['WebhookEndpointWithSecretResponse']
export type WebhookEndpointsPage = Schemas['Page_WebhookEndpointResponse']
export type CreateWebhookEndpointBody = Schemas['CreateWebhookEndpointRequest']
export type UpdateWebhookEndpointBody = Schemas['UpdateWebhookEndpointRequest']
export type WebhookDelivery = Schemas['WebhookDeliveryResponse']
export type WebhookDeliveryDetail = Schemas['WebhookDeliveryDetailResponse']
export type WebhookDeliveriesPage = Schemas['Page_WebhookDeliveryResponse']
export type PublicEventType = Schemas['PublicEventTypeResponse']

export const ALL_EVENTS = '*'
export const ENDPOINTS_PER_PAGE = 100
export const DELIVERIES_PER_PAGE = 20
export const MAX_URL_LENGTH = 2048
export const MAX_DESCRIPTION_LENGTH = 255
export const DELIVERY_STATUSES = ['pending', 'succeeded', 'failed'] as const
export type DeliveryStatus = (typeof DELIVERY_STATUSES)[number]

const PENDING_REFRESH_MS = 3000

export const webhooksKeys = {
  all: ['webhooks'] as const,
  endpoints: () => [...webhooksKeys.all, 'endpoints'] as const,
  catalog: () => [...webhooksKeys.all, 'catalog'] as const,
  deliveries: (endpointId: string) => [...webhooksKeys.all, 'deliveries', endpointId] as const,
  deliveryList: (endpointId: string, status: DeliveryStatus | null, page: number) =>
    [...webhooksKeys.deliveries(endpointId), 'list', status, page] as const,
  delivery: (endpointId: string, deliveryId: string) =>
    [...webhooksKeys.deliveries(endpointId), 'detail', deliveryId] as const,
}

function endpointPath(id: string): string {
  return `/v1/webhook-endpoints/${encodeURIComponent(id)}`
}

export const webhooksApi = {
  list: () => api.get<WebhookEndpointsPage>('/v1/webhook-endpoints', { query: { per_page: ENDPOINTS_PER_PAGE } }),
  catalog: () => api.get<PublicEventType[]>('/v1/events/catalog'),
  create: (body: CreateWebhookEndpointBody) => api.post<WebhookEndpointWithSecret>('/v1/webhook-endpoints', body),
  update: (endpoint: WebhookEndpoint, body: UpdateWebhookEndpointBody) =>
    api.patch<WebhookEndpoint>(endpointPath(endpoint.id), body, { headers: { 'If-Match': endpoint.updated_at } }),
  remove: (id: string) => api.delete<void>(endpointPath(id)),
  rotateSecret: (id: string) => api.post<WebhookEndpointWithSecret>(`${endpointPath(id)}/rotate-secret`),
  test: (id: string) => api.post<WebhookDelivery>(`${endpointPath(id)}/test`),
  deliveries: (endpointId: string, status: DeliveryStatus | null, page: number) =>
    api.get<WebhookDeliveriesPage>(`${endpointPath(endpointId)}/deliveries`, {
      query: { status: status ?? undefined, page, per_page: DELIVERIES_PER_PAGE },
    }),
  delivery: (endpointId: string, deliveryId: string) =>
    api.get<WebhookDeliveryDetail>(`${endpointPath(endpointId)}/deliveries/${encodeURIComponent(deliveryId)}`),
  redeliver: (endpointId: string, deliveryId: string) =>
    api.post<WebhookDelivery>(`${endpointPath(endpointId)}/deliveries/${encodeURIComponent(deliveryId)}/redeliver`),
}

export function useWebhookEndpoints() {
  return useQuery({
    queryKey: webhooksKeys.endpoints(),
    queryFn: webhooksApi.list,
    select: (page) => page.data,
  })
}

export function useEventCatalog() {
  return useQuery({ queryKey: webhooksKeys.catalog(), queryFn: webhooksApi.catalog, staleTime: Infinity })
}

export function useWebhookDeliveries(endpointId: string, status: DeliveryStatus | null, page: number) {
  return useQuery({
    queryKey: webhooksKeys.deliveryList(endpointId, status, page),
    queryFn: () => webhooksApi.deliveries(endpointId, status, page),
    refetchInterval: (query) =>
      query.state.data?.data.some((delivery) => delivery.status === 'pending') ? PENDING_REFRESH_MS : false,
  })
}

export function useWebhookDelivery(endpointId: string, deliveryId: string | null) {
  return useQuery({
    queryKey: webhooksKeys.delivery(endpointId, deliveryId ?? ''),
    queryFn: () => webhooksApi.delivery(endpointId, deliveryId ?? ''),
    enabled: deliveryId !== null,
  })
}

export function useInvalidateWebhooks() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: webhooksKeys.all })
}

export function subscribesToAll(endpoint: Pick<WebhookEndpoint, 'events'>): boolean {
  return endpoint.events.includes(ALL_EVENTS)
}

export interface EventGroup {
  resource: string
  events: PublicEventType[]
}

export function groupEvents(catalog: PublicEventType[]): EventGroup[] {
  const groups = new Map<string, PublicEventType[]>()
  for (const event of catalog) {
    groups.set(event.resource, [...(groups.get(event.resource) ?? []), event])
  }
  return [...groups.entries()].map(([resource, events]) => ({ resource, events }))
}

export function isDeliveryStatus(value: string): value is DeliveryStatus {
  return (DELIVERY_STATUSES as readonly string[]).includes(value)
}
