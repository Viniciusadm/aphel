import { Copy, FileText, MoreHorizontal, Pencil, Plus, Star } from 'lucide-react'
import { useState } from 'react'
import { Link, useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { Skeleton } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { formatDateTime, MANAGE_PERMISSION, PROPOSALS_KEY, PROPOSALS_PATH, proposalsApi, useInvalidateProposals, useProposalTemplates, useRecordFormatting, type ProposalTemplate, type UpdateProposalTemplate } from '@/sdk'
import { NoAccess } from './NoAccess'

const EXCERPT_CHARS = 220

function excerpt(body: string): string {
  return body
    .replace(/\{\{\s*[\w.]+\s*\}\}/g, '…')
    .replace(/^[-*]{3,}\s*$/gm, '')
    .replace(/^#{1,6}\s*/gm, '')
    .replace(/^-\s+/gm, '')
    .replace(/[*_`]/g, '')
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => /\p{L}/u.test(line))
    .join(' · ')
    .slice(0, EXCERPT_CHARS)
}

export function ProposalsPage() {
  const t = useModuleTranslation(PROPOSALS_KEY)
  const navigate = useNavigate()
  const { hasPermission } = useSession()
  const canManage = hasPermission(MANAGE_PERMISSION)
  const templates = useProposalTemplates(canManage)
  const invalidate = useInvalidateProposals()
  const [deleting, setDeleting] = useState<ProposalTemplate | null>(null)

  if (!canManage) return <NoAccess />

  const update = async (template: ProposalTemplate, changes: UpdateProposalTemplate) => {
    try {
      const updated = await proposalsApi.updateTemplate(template.id, changes)
      notifySuccess(t('list.updated', { name: updated.name }))
      await invalidate()
    } catch (error) {
      notifyError(error)
    }
  }

  const duplicate = async (template: ProposalTemplate) => {
    try {
      const copy = await proposalsApi.createTemplate({
        name: t('list.copyOf', { name: template.name }).slice(0, 160),
        description: template.description ?? undefined,
        body: template.body,
        is_active: true,
        is_default: false,
      })
      notifySuccess(t('list.duplicated', { name: copy.name }))
      await invalidate()
      navigate(`${PROPOSALS_PATH}/${copy.id}`)
    } catch (error) {
      notifyError(error)
    }
  }

  const newButton = (
    <Button asChild size="sm">
      <Link to={`${PROPOSALS_PATH}/new`}>
        <Plus aria-hidden="true" />
        {t('list.new')}
      </Link>
    </Button>
  )

  return (
    <Page>
      <PageHeader title={t('list.title')} description={t('list.description')} actions={newButton} />
      {templates.isPending ? (
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3" aria-busy="true">
          <Skeleton className="h-44 w-full" />
          <Skeleton className="h-44 w-full" />
        </div>
      ) : templates.isError ? (
        <ErrorState onRetry={() => void templates.refetch()} />
      ) : templates.data.length === 0 ? (
        <EmptyState icon={FileText} title={t('list.emptyTitle')} description={t('list.emptyDescription')} action={newButton} />
      ) : (
        <ul className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3" aria-label={t('list.listLabel')}>
          {templates.data.map((template) => (
            <TemplateCard
              key={template.id}
              template={template}
              onUpdate={(changes) => update(template, changes)}
              onDuplicate={() => duplicate(template)}
              onDelete={() => setDeleting(template)}
            />
          ))}
        </ul>
      )}
      <ConfirmDialog
        open={deleting !== null}
        onOpenChange={(open) => !open && setDeleting(null)}
        title={t('confirmDelete.title')}
        description={deleting ? t('confirmDelete.description', { name: deleting.name }) : ''}
        confirmLabel={t('confirmDelete.action')}
        variant="destructive"
        onConfirm={async () => {
          if (!deleting) return
          await proposalsApi.deleteTemplate(deleting.id)
          notifySuccess(t('list.deleted', { name: deleting.name }))
          await invalidate()
        }}
      />
    </Page>
  )
}

interface TemplateCardProps {
  template: ProposalTemplate
  onUpdate: (changes: UpdateProposalTemplate) => void
  onDuplicate: () => void
  onDelete: () => void
}

function TemplateCard({ template, onUpdate, onDuplicate, onDelete }: TemplateCardProps) {
  const t = useModuleTranslation(PROPOSALS_KEY)
  const formatting = useRecordFormatting()
  const editPath = `${PROPOSALS_PATH}/${template.id}`
  const summary = excerpt(template.body)

  return (
    <li className={cn('flex flex-col gap-3 rounded-lg border bg-card p-4 shadow-sm transition-shadow hover:shadow-md', !template.is_active && 'opacity-75')}>
      <div className="flex items-start gap-3">
        <span className="flex size-10 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary">
          <FileText className="size-5" aria-hidden="true" />
        </span>
        <div className="grid min-w-0 flex-1 gap-1">
          <Link to={editPath} className="font-semibold break-words hover:underline">
            {template.name}
          </Link>
          <div className="flex flex-wrap items-center gap-1.5">
            {template.is_default && (
              <Badge variant="secondary" title={t('list.isDefaultHint')}>
                <Star className="size-3" aria-hidden="true" />
                {t('list.default')}
              </Badge>
            )}
            <Badge variant={template.is_active ? 'success' : 'outline'}>
              {template.is_active ? t('list.active') : t('list.inactive')}
            </Badge>
          </div>
        </div>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button size="icon" variant="ghost" aria-label={t('list.moreActions', { name: template.name })}>
              <MoreHorizontal aria-hidden="true" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            {template.is_active && !template.is_default && (
              <DropdownMenuItem onSelect={() => onUpdate({ is_default: true })}>{t('list.makeDefault')}</DropdownMenuItem>
            )}
            <DropdownMenuItem onSelect={() => onUpdate({ is_active: !template.is_active })}>
              {template.is_active ? t('list.deactivate') : t('list.activate')}
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem className="text-destructive focus:text-destructive" onSelect={onDelete}>
              {t('list.delete')}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
      {template.description && <p className="text-sm break-words text-muted-foreground">{template.description}</p>}
      <p className="line-clamp-3 min-h-[3.75rem] rounded-md bg-muted/40 px-3 py-2 text-xs leading-relaxed break-words text-muted-foreground">
        {summary || t('list.noText')}
      </p>
      <div className="mt-auto flex items-center justify-between gap-2">
        <p className="text-xs text-muted-foreground">{t('list.updatedAt', { date: formatDateTime(template.updated_at, formatting) })}</p>
        <div className="flex gap-2">
          <Button size="sm" variant="outline" onClick={onDuplicate}>
            <Copy aria-hidden="true" />
            {t('list.duplicate')}
          </Button>
          <Button asChild size="sm">
            <Link to={editPath}>
              <Pencil aria-hidden="true" />
              {t('list.edit')}
            </Link>
          </Button>
        </div>
      </div>
    </li>
  )
}
