import type { LucideIcon } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { EmptyState } from '@/components/ui/empty-state'

export interface RecordTabPlaceholderProps {
  description: string
  icon?: LucideIcon
  title?: string
}

export function RecordTabPlaceholder({ description, icon, title }: RecordTabPlaceholderProps) {
  const { t } = useTranslation('records-detail')
  return <EmptyState icon={icon} title={title ?? t('placeholders.soon')} description={description} />
}
