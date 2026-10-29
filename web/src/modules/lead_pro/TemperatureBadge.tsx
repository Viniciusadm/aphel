import { Flame } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { LEAD_PRO_KEY, temperatureBand } from './api'

const VARIANTS = { cold: 'secondary', warm: 'warning', hot: 'destructive' } as const

export const DEFAULT_HOT_THRESHOLD = 70

export function TemperatureBadge({ temperature, hotThreshold }: { temperature: number; hotThreshold: number }) {
  const t = useModuleTranslation(LEAD_PRO_KEY)
  const band = temperatureBand(temperature, hotThreshold)
  return (
    <Badge variant={VARIANTS[band]} className="gap-1 tabular-nums">
      <Flame className="size-3" aria-hidden="true" />
      {temperature}
      <span className="sr-only">{t(`bands.${band}`)}</span>
    </Badge>
  )
}
