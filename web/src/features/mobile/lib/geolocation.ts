export interface CapturedLocation {
  latitude: number
  longitude: number
  accuracy_m: number | null
  captured_at: string
}

export type LocationFailure = 'unsupported' | 'denied' | 'unavailable' | 'timeout'

export type LocationResult = { status: 'ok'; location: CapturedLocation } | { status: 'failed'; reason: LocationFailure }

export const LOCATION_TIMEOUT_MS = 15_000
export const LOCATION_MAX_AGE_MS = 30_000

function failureOf(error: GeolocationPositionError): LocationFailure {
  if (error.code === error.PERMISSION_DENIED) return 'denied'
  if (error.code === error.TIMEOUT) return 'timeout'
  return 'unavailable'
}

export function isGeolocationSupported(): boolean {
  return typeof navigator !== 'undefined' && 'geolocation' in navigator
}

export function captureLocation(): Promise<LocationResult> {
  if (!isGeolocationSupported()) return Promise.resolve({ status: 'failed', reason: 'unsupported' })
  return new Promise((resolve) => {
    navigator.geolocation.getCurrentPosition(
      (position) =>
        resolve({
          status: 'ok',
          location: {
            latitude: position.coords.latitude,
            longitude: position.coords.longitude,
            accuracy_m: Number.isFinite(position.coords.accuracy) ? Math.round(position.coords.accuracy) : null,
            captured_at: new Date(position.timestamp).toISOString(),
          },
        }),
      (error) => resolve({ status: 'failed', reason: failureOf(error) }),
      { enableHighAccuracy: true, timeout: LOCATION_TIMEOUT_MS, maximumAge: LOCATION_MAX_AGE_MS },
    )
  })
}

export function mapUrl(latitude: number | string, longitude: number | string): string {
  const lat = encodeURIComponent(String(latitude))
  const lng = encodeURIComponent(String(longitude))
  return `https://www.openstreetmap.org/?mlat=${lat}&mlon=${lng}#map=17/${lat}/${lng}`
}

export interface MapLinkLocation {
  latitude: number | string
  longitude: number | string
  map_url?: string | null
}

export function timelineLocation(value: unknown): MapLinkLocation | null {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) return null
  const location = value as Record<string, unknown>
  const coordinate = (raw: unknown) => (typeof raw === 'string' || typeof raw === 'number' ? raw : null)
  const latitude = coordinate(location.latitude)
  const longitude = coordinate(location.longitude)
  if (latitude === null || longitude === null) return null
  return { latitude, longitude, map_url: typeof location.map_url === 'string' ? location.map_url : null }
}
