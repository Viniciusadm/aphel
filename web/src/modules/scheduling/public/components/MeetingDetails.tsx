import { PageTitle } from '@/components/ui/page-title'
import { Clock, MapPin, Phone, User, Video } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { MeetingLocationKind } from '@/modules/scheduling/types'

interface MeetingDetailsProps {
  organizationName: string
  meetingName: string
  durationMinutes: number
  hostName: string | null
  locationKind: MeetingLocationKind
  description?: string | null
}

const LOCATION_ICONS = { video: Video, address: MapPin, phone: Phone } as const

export function MeetingDetails({
  organizationName,
  meetingName,
  durationMinutes,
  hostName,
  locationKind,
  description,
}: MeetingDetailsProps) {
  const { t } = useTranslation('scheduling', { keyPrefix: 'public' })
  const LocationIcon = locationKind === 'none' ? null : LOCATION_ICONS[locationKind]
  return (
    <div className="grid gap-3">
      <p className="text-sm font-medium text-muted-foreground">{organizationName}</p>
      <PageTitle>{meetingName}</PageTitle>
      <ul className="grid gap-2 text-sm text-muted-foreground">
        {hostName && (
          <li className="flex items-center gap-2">
            <User className="size-4" aria-hidden />
            {hostName}
          </li>
        )}
        <li className="flex items-center gap-2">
          <Clock className="size-4" aria-hidden />
          {t('details.duration', { count: durationMinutes })}
        </li>
        {LocationIcon && (
          <li className="flex items-center gap-2">
            <LocationIcon className="size-4" aria-hidden />
            {t(`locationKinds.${locationKind}`)}
          </li>
        )}
      </ul>
      {description && <p className="text-sm whitespace-pre-line">{description}</p>}
    </div>
  )
}
