import { MapPin } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { components } from '@/api/schema'
import { cn } from '@/lib/utils'
import { mapUrl, type MapLinkLocation } from '../lib/geolocation'

type ActivityLocation = components['schemas']['ActivityLocationResponse']

export interface ActivityMapLinkProps {
  location?: ActivityLocation | MapLinkLocation | null
  className?: string
}

export function ActivityMapLink({ location, className }: ActivityMapLinkProps) {
  const { t } = useTranslation('mobile')
  if (!location) return null
  const href = location.map_url || mapUrl(location.latitude, location.longitude)
  return (
    <a
      href={href}
      target="_blank"
      rel="noreferrer"
      className={cn('inline-flex w-fit items-center gap-1 text-xs text-primary underline-offset-4 hover:underline', className)}
    >
      <MapPin className="size-3.5" aria-hidden="true" />
      {t('checkIn.viewOnMap')}
    </a>
  )
}
