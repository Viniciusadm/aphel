import { useQuery } from '@tanstack/react-query'
import { FlaskConical } from 'lucide-react'
import { api, type components } from '@/api'
import { useModuleTranslation } from '@/kernel/modules/i18n'

type DemoStatus = components['schemas']['DemoStatusResponse']

export function DemoBanner() {
  const t = useModuleTranslation('demo')
  const status = useQuery({
    queryKey: ['demo', 'status'],
    queryFn: () => api.get<DemoStatus>('/v1/demo'),
    staleTime: Infinity,
    retry: false,
  })
  if (!status.data?.enabled) return null
  const { visitor, reset_time_utc: resetTime } = status.data

  return (
    <div
      role="note"
      aria-label={t('banner.label')}
      className="flex flex-wrap items-center justify-center gap-x-3 gap-y-1 border-b border-warning/40 bg-warning/15 px-4 py-2 text-center text-sm"
    >
      <span className="inline-flex items-center gap-1.5 font-medium">
        <FlaskConical className="size-4" aria-hidden="true" />
        {t('banner.label')}
      </span>
      <span>{t('banner.message', { time: resetTime ?? '' })}</span>
      {visitor && (
        <span className="break-words">
          {t('banner.credentials', { email: visitor.email, password: visitor.password })}
        </span>
      )}
    </div>
  )
}
