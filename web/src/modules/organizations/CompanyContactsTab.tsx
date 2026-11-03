import { Users } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Badge } from '@/components/ui/badge'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { CONTACT_COMPANY_ROLES } from '@/modules/people/resource'
import { useCompanyContacts } from './api'

export interface CompanyContactsTabProps {
  companyId: string
}

export function CompanyContactsTab({ companyId }: CompanyContactsTabProps) {
  const { t } = useTranslation('organizations')
  const { t: tContacts } = useTranslation('people')
  const people = useCompanyContacts(companyId)

  if (people.isPending) {
    return (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-12 w-full" />
        <Skeleton className="h-12 w-full" />
      </div>
    )
  }
  if (people.isError) return <ErrorState title={t('people.loadError')} onRetry={() => void people.refetch()} />
  if (people.data.data.length === 0) return <EmptyState icon={Users} title={t('people.empty')} />

  const roleLabel = (role: string | null) =>
    role && (CONTACT_COMPANY_ROLES as readonly string[]).includes(role)
      ? tContacts(`roles.${role as (typeof CONTACT_COMPANY_ROLES)[number]}`)
      : role

  const {
    data: items,
    meta: { total },
  } = people.data
  return (
    <div className="grid gap-2">
      <ul aria-label={t('people.title')} className="grid gap-2">
        {items.map((link) => (
          <li
            key={link.link_id ?? `primary-${link.contact.id}`}
            className="flex flex-wrap items-center justify-between gap-2 rounded-md border p-3"
          >
            <div className="grid min-w-0 gap-0.5">
              <Link to={`/people/${encodeURIComponent(link.contact.id)}`} className="truncate font-medium hover:underline">
                {link.contact.name}
              </Link>
              <span className="truncate text-xs text-muted-foreground">
                {[link.contact.job_title, link.contact.email, link.contact.phone ?? link.contact.mobile]
                  .filter(Boolean)
                  .join(' · ')}
              </span>
            </div>
            <div className="flex items-center gap-2">
              {link.is_primary ? (
                <Badge variant="secondary">{t('people.primary')}</Badge>
              ) : (
                link.role && <Badge variant="outline">{roleLabel(link.role)}</Badge>
              )}
            </div>
          </li>
        ))}
      </ul>
      {total > items.length && (
        <p className="text-xs text-muted-foreground">{t('people.more', { shown: items.length, total })}</p>
      )}
    </div>
  )
}
