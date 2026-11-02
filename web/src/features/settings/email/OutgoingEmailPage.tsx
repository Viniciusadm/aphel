import { useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { ConfigurationGate } from '../general/ConfigurationGate'
import { useSaveConfiguration, type OrganizationConfiguration } from '../general/api'
import { NativeSelect } from '../organization/components/NativeSelect'
import { MailNotConfiguredNotice } from './MailNotConfiguredNotice'

function MailForm({ configuration }: { configuration: OrganizationConfiguration }) {
  const { t } = useTranslation('settings')
  const save = useSaveConfiguration()
  const [mail, setMail] = useState(configuration.mail)
  const [password, setPassword] = useState('')
  const [clearPassword, setClearPassword] = useState(false)
  const [saving, setSaving] = useState(false)
  const submit = async (event: FormEvent) => {
    event.preventDefault()
    setSaving(true)
    try {
      await save({ mail, ...(clearPassword ? { mail_password: null } : password ? { mail_password: password } : {}) })
      setPassword('')
      setClearPassword(false)
      notifySuccess(t('configuration.saved'))
    } catch (error) { notifyError(error) } finally { setSaving(false) }
  }
  return (
    <form onSubmit={(event) => void submit(event)} className="grid max-w-2xl gap-4">
      <fieldset disabled={saving} className="grid gap-4">
        {(['host', 'from', 'username'] as const).map((key) => (
          <label key={key} className="grid gap-1 text-sm">
            {t(`configuration.mail.transport.${key}`)}
            <Input value={mail[key] ?? ''} type={key === 'from' ? 'email' : 'text'} onChange={(event) => setMail({ ...mail, [key]: event.target.value.trim() || null })} />
          </label>
        ))}
        <label className="grid gap-1 text-sm">{t('configuration.mail.transport.port')}
          <Input type="number" min={1} max={65535} required value={mail.port} onChange={(event) => setMail({ ...mail, port: Number(event.target.value) })} />
        </label>
        <label className="grid gap-1 text-sm">{t('configuration.mail.transport.security')}
          <NativeSelect value={mail.security} onChange={(event) => setMail({ ...mail, security: event.target.value })}>
            <option value="starttls">STARTTLS</option><option value="tls">TLS</option><option value="none">{t('configuration.mail.transport.none')}</option>
          </NativeSelect>
        </label>
        <label className="grid gap-1 text-sm">{t('configuration.mail.transport.password')}
          <Input type="password" autoComplete="new-password" value={password} onChange={(event) => setPassword(event.target.value)} />
        </label>
        {configuration.mail_secret_configured && <label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={clearPassword} onChange={(event) => setClearPassword(event.target.checked)} />{t('configuration.mail.transport.clearPassword')}</label>}
        <Button type="submit" disabled={saving}>{t(saving ? 'configuration.saving' : 'configuration.save')}</Button>
      </fieldset>
    </form>
  )
}

export function OutgoingEmailPage() {
  const { t } = useTranslation('settings')
  return <Page><PageHeader title={t('configuration.mail.title')} description={t('configuration.mail.description')} /><MailNotConfiguredNotice showLink={false} /><ConfigurationGate>{(configuration) => <MailForm configuration={configuration} />}</ConfigurationGate></Page>
}
