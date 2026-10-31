import { Building2, Link2Off } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { isApiError } from '@/api'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { RecordFieldInput } from '@/components/fields'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { useContactCompanies, useContactCompanyMutations, type ContactCompanyLink } from './api'
import { CONTACT_COMPANY_ROLES } from './resource'
import { Field } from '@/components/ui/field'

export interface ContactCompaniesTabProps {
  contactId: string
  readOnly?: boolean
}

export function ContactCompaniesTab({ contactId, readOnly = false }: ContactCompaniesTabProps) {
  const { t } = useTranslation('contacts')
  const companies = useContactCompanies(contactId)
  const [removing, setRemoving] = useState<ContactCompanyLink | null>(null)
  const { unlink } = useContactCompanyMutations(contactId)

  const roleLabel = (role: string | null) =>
    role && (CONTACT_COMPANY_ROLES as readonly string[]).includes(role)
      ? t(`roles.${role as (typeof CONTACT_COMPANY_ROLES)[number]}`)
      : (role ?? t('companies.noRole'))

  return (
    <div className="grid gap-4">
      {!readOnly && <LinkCompanyForm contactId={contactId} />}
      {companies.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
        </div>
      ) : companies.isError ? (
        <ErrorState title={t('companies.loadError')} onRetry={() => void companies.refetch()} />
      ) : companies.data.data.length === 0 ? (
        <EmptyState icon={Building2} title={t('companies.empty')} />
      ) : (
        <ul aria-label={t('companies.title')} className="grid gap-2">
          {companies.data.data.map((link) => (
            <li
              key={link.link_id ?? `primary-${link.company.id}`}
              className="flex flex-wrap items-center justify-between gap-2 rounded-md border p-3"
            >
              <div className="grid min-w-0 gap-0.5">
                <Link to={`/companies/${encodeURIComponent(link.company.id)}`} className="truncate font-medium hover:underline">
                  {link.company.name}
                </Link>
                <span className="text-xs text-muted-foreground">
                  {[link.company.domain, link.is_primary ? null : roleLabel(link.role)].filter(Boolean).join(' · ')}
                </span>
              </div>
              <div className="flex items-center gap-2">
                {link.is_primary && <Badge variant="secondary">{t('companies.primary')}</Badge>}
                {!link.is_primary && !readOnly && (
                  <Button
                    variant="ghost"
                    size="icon"
                    aria-label={t('companies.remove', { name: link.company.name })}
                    onClick={() => setRemoving(link)}
                  >
                    <Link2Off aria-hidden="true" />
                  </Button>
                )}
              </div>
            </li>
          ))}
        </ul>
      )}
      <ConfirmDialog
        open={removing !== null}
        onOpenChange={(open) => !open && setRemoving(null)}
        title={t('companies.removeTitle')}
        description={t('companies.removeDescription', { name: removing?.company.name ?? '' })}
        confirmLabel={t('companies.removeConfirm')}
        onConfirm={async () => {
          if (!removing) return
          await unlink.mutateAsync(removing.company.id)
          notifySuccess(t('companies.removed'))
        }}
      />
    </div>
  )
}

function LinkCompanyForm({ contactId }: { contactId: string }) {
  const { t } = useTranslation('contacts')
  const companyLabelId = useId()
  const [companyId, setCompanyId] = useState<string | null>(null)
  const [role, setRole] = useState('')
  const { link } = useContactCompanyMutations(contactId)

  const submit = async () => {
    if (!companyId) return
    try {
      await link.mutateAsync({ company_id: companyId, role: role || null })
      notifySuccess(t('companies.linked'))
      setCompanyId(null)
      setRole('')
    } catch (error) {
      if (isApiError(error) && error.status === 409) notifyError(error, { title: t('companies.alreadyLinked') })
      else notifyError(error)
    }
  }

  return (
    <form
      className="grid gap-3 rounded-md border p-3 sm:grid-cols-[minmax(0,1fr)_12rem_auto] sm:items-end"
      onSubmit={(event) => {
        event.preventDefault()
        void submit()
      }}
    >
      <Field id={companyLabelId} label={t('companies.companyLabel')} className="min-w-0">
        <RecordFieldInput targetRecordType="company" value={companyId} onChange={setCompanyId} id={companyLabelId} />
      </Field>
      <Field label={t('companies.roleLabel')}>
        {(control) => <NativeSelect {...control} value={role} onChange={(event) => setRole(event.target.value)}>
          <option value="">{t('companies.roleNone')}</option>
          {CONTACT_COMPANY_ROLES.map((value) => (
            <option key={value} value={value}>
              {t(`roles.${value}`)}
            </option>
          ))}
        </NativeSelect>}
      </Field>
      <Button type="submit" disabled={!companyId || link.isPending}>
        {link.isPending ? t('companies.adding') : t('companies.add')}
      </Button>
    </form>
  )
}
