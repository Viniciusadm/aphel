import { Eye, EyeOff, FileText } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import type { EmailVisibility } from '../types'

const ICONS = { private: EyeOff, metadata: FileText, shared: Eye } as const

export function VisibilityBadge({ visibility }: { visibility: EmailVisibility }) {
  const { t } = useTranslation('email')
  const Icon = ICONS[visibility]
  return (
    <Badge variant="outline" title={t(`visibility.${visibility}.description`)}>
      <Icon className="size-3" aria-hidden="true" />
      {t(`visibility.${visibility}.label`)}
    </Badge>
  )
}
