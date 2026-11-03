import { useTranslation } from 'react-i18next'
import { useNavigate, useParams } from 'react-router'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { PageLoader } from '@/components/ui/page-loader'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { ASSIGNMENT_MANAGE } from '@/features/settings/lib/permissions'
import { useDistributionRules } from './api'
import { RuleEditorDialog } from './RuleEditorDialog'

export function RuleEditorPage() {
  return <RequirePermission permission={ASSIGNMENT_MANAGE}><RuleEditor /></RequirePermission>
}

function RuleEditor() {
  const { id } = useParams()
  const { t } = useTranslation('assignment')
  const navigate = useNavigate()
  const rules = useDistributionRules()
  if (id && rules.isPending) return <PageLoader />
  if (id && rules.isError) return <ErrorState onRetry={() => void rules.refetch()} />
  const rule = id ? rules.data?.find((item) => item.id === id) : null
  if (id && !rule) return <EmptyState title={t('editor.notFound')} />
  return <RuleEditorDialog key={id ?? 'new'} presentation="page" rule={rule ?? null} onClose={() => void navigate('/settings/assignment')} />
}
