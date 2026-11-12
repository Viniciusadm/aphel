import { useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { ToggleField } from '@/components/ui/toggle-field'
import { SettingsCard, WIDE_FIELD } from '../components/SettingsCard'
import { ConfigurationGate } from '../general/ConfigurationGate'
import { useSaveConfiguration, type OrganizationConfiguration } from '../general/api'
import { NativeSelect } from '../organization/components/NativeSelect'
import { MailNotConfiguredNotice } from './MailNotConfiguredNotice'

function useMailSettings(configuration: OrganizationConfiguration) {
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
    } catch (error) {
      notifyError(error)
    } finally {
      setSaving(false)
    }
  }
  return { mail, setMail, password, setPassword, clearPassword, setClearPassword, saving, submit }
}

type MailSettingsState = ReturnType<typeof useMailSettings>

function MailFields({ configuration, state }: { configuration: OrganizationConfiguration; state: MailSettingsState }) {
  const { t } = useTranslation('settings')
  const { mail, setMail, password, setPassword, clearPassword, setClearPassword, saving } = state
  return (
    <>
      <SettingsCard title={t('configuration.mail.sections.server')} disabled={saving}>
        <div className={WIDE_FIELD}>
          <Field id="mail-host" label={t('configuration.mail.transport.host')}>
            <Input
              id="mail-host"
              value={mail.host ?? ''}
              onChange={(event) => setMail({ ...mail, host: event.target.value.trim() || null })}
            />
          </Field>
        </div>
        <Field id="mail-port" label={t('configuration.mail.transport.port')} required>
          <Input
            id="mail-port"
            type="number"
            min={1}
            max={65535}
            required
            value={mail.port}
            onChange={(event) => setMail({ ...mail, port: Number(event.target.value) })}
          />
        </Field>
        <Field id="mail-security" label={t('configuration.mail.transport.security')}>
          <NativeSelect id="mail-security" value={mail.security} onChange={(event) => setMail({ ...mail, security: event.target.value })}>
            <option value="starttls">STARTTLS</option>
            <option value="tls">TLS</option>
            <option value="none">{t('configuration.mail.transport.none')}</option>
          </NativeSelect>
        </Field>
      </SettingsCard>
      <SettingsCard title={t('configuration.mail.sections.account')} disabled={saving}>
        <div className={WIDE_FIELD}>
          <Field id="mail-from" label={t('configuration.mail.transport.from')}>
            <Input
              id="mail-from"
              type="email"
              value={mail.from ?? ''}
              onChange={(event) => setMail({ ...mail, from: event.target.value.trim() || null })}
            />
          </Field>
        </div>
        <Field id="mail-username" label={t('configuration.mail.transport.username')}>
          <Input
            id="mail-username"
            value={mail.username ?? ''}
            onChange={(event) => setMail({ ...mail, username: event.target.value.trim() || null })}
          />
        </Field>
        <Field id="mail-password" label={t('configuration.mail.transport.password')}>
          <Input
            id="mail-password"
            type="password"
            autoComplete="new-password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
          />
        </Field>
        {configuration.mail_secret_configured && (
          <div className={WIDE_FIELD}>
            <ToggleField
              label={t('configuration.mail.transport.clearPassword')}
              checked={clearPassword}
              onCheckedChange={(checked) => setClearPassword(checked === true)}
            />
          </div>
        )}
      </SettingsCard>
    </>
  )
}

export function MailSettingsForm({ configuration }: { configuration: OrganizationConfiguration }) {
  const { t } = useTranslation('settings')
  const state = useMailSettings(configuration)
  return (
    <form onSubmit={(event) => void state.submit(event)} className="grid gap-6">
      <MailFields configuration={configuration} state={state} />
      <div className="flex justify-end">
        <Button type="submit" disabled={state.saving}>
          {t(state.saving ? 'configuration.saving' : 'configuration.save')}
        </Button>
      </div>
    </form>
  )
}

function OutgoingEmailForm({ configuration }: { configuration: OrganizationConfiguration }) {
  const { t } = useTranslation('settings')
  const state = useMailSettings(configuration)
  return (
    <Page width="form">
      <PageHeader
        title={t('configuration.mail.title')}
        description={t('configuration.mail.description')}
        actions={
          <Button type="submit" form="outgoing-email-form" disabled={state.saving}>
            {t(state.saving ? 'configuration.saving' : 'configuration.save')}
          </Button>
        }
      />
      <MailNotConfiguredNotice showLink={false} />
      <form id="outgoing-email-form" onSubmit={(event) => void state.submit(event)} className="grid gap-6">
        <MailFields configuration={configuration} state={state} />
      </form>
    </Page>
  )
}

export function OutgoingEmailPage() {
  const { t } = useTranslation('settings')
  return (
    <ConfigurationGate
      fallback={(content) => (
        <Page width="form">
          <PageHeader title={t('configuration.mail.title')} description={t('configuration.mail.description')} />
          <MailNotConfiguredNotice showLink={false} />
          {content}
        </Page>
      )}
    >
      {(configuration) => <OutgoingEmailForm configuration={configuration} />}
    </ConfigurationGate>
  )
}
