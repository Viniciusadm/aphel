import { FileDown } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import type { SlotProps } from '@/kernel/modules/slots'
import { exportPermission } from './api'
import { ExportDialog } from './ExportDialog'

export function ExportAction({ recordType }: SlotProps<'record.list.actions'>) {
  const { t } = useTranslation('exports')
  const { hasPermission } = useSession()
  const [exporting, setExporting] = useState(false)
  const permission = exportPermission(recordType)
  if (permission === null || !hasPermission(permission)) return null
  return (
    <>
      <Button size="sm" variant="outline" onClick={() => setExporting(true)}>
        <FileDown aria-hidden />
        {t('action')}
      </Button>
      <ExportDialog recordType={recordType} open={exporting} onOpenChange={setExporting} />
    </>
  )
}
