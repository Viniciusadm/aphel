import { Check, Copy, Download } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { notifyError } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { recoveryCodesFile } from './api'

const FILE_NAME = 'aphel-recovery-codes.txt'

export function RecoveryCodesView({ codes }: { codes: string[] }) {
  const { t } = useTranslation('two-factor')
  const { me } = useSession()
  const [copied, setCopied] = useState(false)
  const content = recoveryCodesFile(t('recoveryCodes.fileHeading', { email: me.user.email }), codes)

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(codes.join('\n'))
      setCopied(true)
    } catch {
      notifyError(new Error(t('recoveryCodes.copyFailed')), { title: t('recoveryCodes.copyFailed') })
    }
  }

  const download = () => {
    const url = URL.createObjectURL(new Blob([content], { type: 'text/plain;charset=utf-8' }))
    const link = document.createElement('a')
    link.href = url
    link.download = FILE_NAME
    link.click()
    URL.revokeObjectURL(url)
  }

  return (
    <div className="grid gap-3">
      <p className="text-sm text-muted-foreground">{t('recoveryCodes.description')}</p>
      <ul
        aria-label={t('recoveryCodes.listLabel')}
        className="grid grid-cols-1 gap-2 rounded-md border bg-muted p-3 font-mono text-sm min-[360px]:grid-cols-2"
      >
        {codes.map((code) => (
          <li key={code} className="text-center tracking-wider">
            {code}
          </li>
        ))}
      </ul>
      <div className="flex flex-wrap gap-2">
        <Button type="button" variant="outline" size="sm" onClick={() => void copy()}>
          {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
          {copied ? t('recoveryCodes.copied') : t('recoveryCodes.copy')}
        </Button>
        <Button type="button" variant="outline" size="sm" onClick={download}>
          <Download aria-hidden="true" />
          {t('recoveryCodes.download')}
        </Button>
      </div>
    </div>
  )
}
