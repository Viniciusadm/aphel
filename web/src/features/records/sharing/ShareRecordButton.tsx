import { Share2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { ShareRecordDialog } from './ShareRecordDialog'

export interface ShareRecordButtonProps {
  recordType: string
  recordId: string
}

export function ShareRecordButton({ recordType, recordId }: ShareRecordButtonProps) {
  const { t } = useTranslation('sharing')
  const [open, setOpen] = useState(false)
  return (
    <>
      <Button variant="outline" size="sm" onClick={() => setOpen(true)}>
        <Share2 aria-hidden />
        {t('button.share')}
      </Button>
      {open && <ShareRecordDialog recordType={recordType} recordId={recordId} open={open} onOpenChange={setOpen} />}
    </>
  )
}
