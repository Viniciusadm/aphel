import { Flame } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { LEAD_PRO_KEY, temperatureBand } from './api'

const TONES = { cold: 'neutral', warm: 'warning', hot: 'destructive' } as const

export const DEFAULT_HOT_THRESHOLD = 70

export function TemperatureBadge({ temperature, hotThreshold }: { temperature: number; hotThreshold: number }) {
  const t = useModuleTranslation(LEAD_PRO_KEY)
  const band = temperatureBand(temperature, hotThreshold)
  return (
    <Badge tone={TONES[band]} icon={<Flame aria-hidden="true" />} className="tabular-nums">
      {temperature}
      <span className="sr-only">{t(`bands.${band}`)}</span>
    </Badge>
  )
}
