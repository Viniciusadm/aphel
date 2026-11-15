import { useTranslation } from 'react-i18next'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import type { SlotProps } from '@/kernel/modules/slots'
import { ApprovalRequestsPanel } from './ApprovalRequestsPanel'

export function QuoteApprovalsCard({ recordType, recordId, canRequest }: SlotProps<'quotes.editor.sidebar'>) {
  const { t } = useTranslation('approvals')
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('panel.tab')}</CardTitle>
      </CardHeader>
      <CardContent>
        <ApprovalRequestsPanel recordType={recordType} recordId={recordId} canRequest={canRequest} />
      </CardContent>
    </Card>
  )
}
