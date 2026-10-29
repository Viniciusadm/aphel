import { Lock } from 'lucide-react'
import { EmptyState } from '@/components/ui/empty-state'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { PROPOSALS_KEY } from './api'

export function NoAccess() {
  const t = useModuleTranslation(PROPOSALS_KEY)
  return <EmptyState icon={Lock} title={t('list.noAccessTitle')} description={t('list.noAccessDescription')} />
}
