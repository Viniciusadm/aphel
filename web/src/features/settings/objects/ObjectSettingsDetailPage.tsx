import { ArrowLeft, ExternalLink, LayoutTemplate, ListChecks, Pencil, ShieldCheck, Trash2 } from 'lucide-react'
import { useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams } from 'react-router'
import { isApiError } from '@/api'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { ObjectIcon, customObjectPath, objectColor } from '@/features/custom-records'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { useCustomObjectAdmin, useInvalidateObjects, type CustomObjectAdmin } from './api'
import { ObjectEditorDialog } from './ObjectEditorDialog'
import { ObjectRemovalDialogs, type RemovalState } from './ObjectRemovalDialogs'
import { RelationshipsSection } from './RelationshipsSection'

export function ObjectSettingsDetailPage() {
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <ObjectSettingsDetail />
    </RequirePermission>
  )
}

function ObjectSettingsDetail() {
  const { t } = useTranslation('objects')
  const { key = '' } = useParams()
  const object = useCustomObjectAdmin(key)

  if (object.isPending) {
    return (
      <div className="grid gap-3" aria-busy="true">
        <Skeleton className="h-12 w-full" />
        <Skeleton className="h-40 w-full" />
      </div>
    )
  }
  if (object.isError) {
    const notFound = isApiError(object.error) && object.error.status === 404
    return notFound ? (
      <EmptyState
        title={t('detailSettings.notFound')}
        action={
          <Button variant="outline" asChild>
            <Link to="/settings/objects">{t('detailSettings.back')}</Link>
          </Button>
        }
      />
    ) : (
      <ErrorState title={t('settings.loadError')} onRetry={() => void object.refetch()} />
    )
  }
  return <ObjectSettingsContent object={object.data} />
}

function ObjectSettingsContent({ object }: { object: CustomObjectAdmin }) {
  const { t } = useTranslation('objects')
  const navigate = useNavigate()
  const invalidate = useInvalidateObjects()
  const [editing, setEditing] = useState(false)
  const [removal, setRemoval] = useState<RemovalState | null>(null)
  const recordType = object.record_type

  return (
    <div className="grid min-w-0 gap-6">
      <div className="grid gap-3">
        <Button variant="ghost" size="sm" className="justify-self-start" asChild>
          <Link to="/settings/objects">
            <ArrowLeft aria-hidden="true" />
            {t('detailSettings.back')}
          </Link>
        </Button>
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="flex min-w-0 items-start gap-3">
            <span
              className="flex size-10 shrink-0 items-center justify-center rounded-md border bg-card"
              style={{ color: objectColor(object.color) }}
            >
              <ObjectIcon icon={object.icon} className="size-5" aria-hidden />
            </span>
            <div className="grid min-w-0 gap-1">
              <div className="flex flex-wrap items-center gap-2">
                <h2 className="truncate text-lg font-semibold">{object.label_plural}</h2>
                {!object.is_active && <Badge variant="secondary">{t('settings.inactive')}</Badge>}
              </div>
              <p className="text-xs text-muted-foreground">
                <span className="font-mono">{recordType}</span>
                {' · '}
                {t('settings.recordCount', { count: object.record_count })}
              </p>
              {object.description && <p className="text-sm text-muted-foreground">{object.description}</p>}
            </div>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            {object.is_active && (
              <Button size="sm" variant="outline" asChild>
                <Link to={customObjectPath(object.key)}>
                  <ExternalLink aria-hidden="true" />
                  {t('detailSettings.openRecords')}
                </Link>
              </Button>
            )}
            <Button size="sm" variant="outline" onClick={() => setEditing(true)}>
              <Pencil aria-hidden="true" />
              {t('detailSettings.edit')}
            </Button>
            <Button
              size="sm"
              variant="outline"
              onClick={() => setRemoval({ object, step: object.record_count > 0 ? 'deactivate' : 'delete' })}
            >
              <Trash2 aria-hidden="true" />
              {t('detailSettings.delete')}
            </Button>
          </div>
        </div>
      </div>
      <div className="grid gap-3 sm:grid-cols-3">
        <LinkCard
          icon={<ListChecks className="size-4" aria-hidden />}
          title={t('detailSettings.fieldsTitle')}
          description={t('detailSettings.fieldsDescription')}
          links={[{ to: `/settings/fields/${recordType}`, label: t('detailSettings.manageFields') }]}
        />
        <LinkCard
          icon={<LayoutTemplate className="size-4" aria-hidden />}
          title={t('detailSettings.layoutsTitle')}
          description={t('detailSettings.layoutsDescription')}
          links={[{ to: `/settings/layouts/${recordType}`, label: t('detailSettings.manageLayouts') }]}
        />
        <LinkCard
          icon={<ShieldCheck className="size-4" aria-hidden />}
          title={t('detailSettings.permissionsTitle')}
          description={t('detailSettings.permissionsDescription', { prefix: recordType })}
          links={[
            { to: '/settings/roles', label: t('detailSettings.manageRoles') },
            { to: `/settings/fields/${recordType}/permissions`, label: t('detailSettings.manageFieldPermissions') },
          ]}
        />
      </div>
      <RelationshipsSection object={object} />
      <ObjectEditorDialog object={object} open={editing} onOpenChange={setEditing} onSaved={() => void invalidate()} />
      <ObjectRemovalDialogs
        state={removal}
        onChange={setRemoval}
        onDone={() => {
          void invalidate()
          if (removal?.step === 'delete') navigate('/settings/objects')
        }}
      />
    </div>
  )
}

interface LinkCardProps {
  icon: ReactNode
  title: string
  description: string
  links: { to: string; label: string }[]
}

function LinkCard({ icon, title, description, links }: LinkCardProps) {
  return (
    <section className="flex min-w-0 flex-col gap-3 rounded-lg border bg-card p-4">
      <div className="grid gap-1">
        <h3 className="flex items-center gap-2 text-sm font-semibold">
          {icon}
          {title}
        </h3>
        <p className="text-sm text-muted-foreground">{description}</p>
      </div>
      <div className="mt-auto flex flex-wrap gap-2">
        {links.map((link) => (
          <Button key={link.to} size="sm" variant="outline" asChild>
            <Link to={link.to}>{link.label}</Link>
          </Button>
        ))}
      </div>
    </section>
  )
}
