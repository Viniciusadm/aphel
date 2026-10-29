import { ShieldOff } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { EmptyState } from '@/components/ui/empty-state'

export function NoAccess() {
  const { t } = useTranslation('settings')
  return <EmptyState icon={ShieldOff} title={t('noAccess.title')} description={t('noAccess.description')} />
}
