import { DialogForm } from '@/components/ui/dialog-form'
import { NativeSelect } from '@/components/ui/native-select'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Download, FileText, Loader2, Plus } from 'lucide-react'
import { useState } from 'react'
import { isApiError } from '@/api'
import { useSession } from '@/app/session/session-context'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
} from '@/components/ui/dialog'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { fileDownloadUrl, formatDateTime, useRecordFormatting } from '@/sdk'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import type { RecordTabRenderProps } from '@/kernel/modules/define'
import {
  GENERATE_PERMISSION,
  PROPOSALS_KEY,
  proposalsApi,
  proposalsKeys,
  useInvalidateProposals,
  useProposalDocuments,
  useProposalTemplates,
  type ProposalDocument,
} from './api'
import { ProposalDocumentView } from './ProposalDocumentView'
import { Field } from '@/components/ui/field'

const KNOWN_ERRORS = [
  'proposal_template_required',
  'proposal_template_inactive',
  'proposal_template_not_found',
  'deal_not_found',
  'permission_denied',
]

function formatNumber(number: number): string {
  return String(number).padStart(4, '0')
}

export function DealProposalsTab({ recordId, readOnly }: RecordTabRenderProps) {
  const t = useModuleTranslation(PROPOSALS_KEY)
  const { hasPermission } = useSession()
  const documents = useProposalDocuments(recordId)
  const [generating, setGenerating] = useState(false)
  const canGenerate = !readOnly && hasPermission(GENERATE_PERMISSION) && hasPermission('deals.update')

  const generateButton = canGenerate ? (
    <Button size="sm" onClick={() => setGenerating(true)}>
      <Plus aria-hidden="true" />
      {t('deal.generate')}
    </Button>
  ) : undefined

  return (
    <section className="flex flex-col gap-3" aria-label={t('deal.title')}>
      <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
        <div className="grid gap-0.5">
          <h2 className="text-base font-semibold">{t('deal.title')}</h2>
          <p className="text-sm text-muted-foreground">{t('deal.description')}</p>
        </div>
        {generateButton}
      </div>
      {documents.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-14 w-full" />
          <Skeleton className="h-14 w-full" />
        </div>
      ) : documents.isError ? (
        <ErrorState onRetry={() => void documents.refetch()} />
      ) : documents.data.length === 0 ? (
        <EmptyState
          icon={FileText}
          title={t('deal.emptyTitle')}
          description={t('deal.emptyDescription')}
          action={generateButton}
        />
      ) : (
        <ul className="grid gap-2">
          {documents.data.map((document) => (
            <DocumentRow key={document.id} document={document} />
          ))}
        </ul>
      )}
      {canGenerate && <GenerateDialog dealId={recordId} open={generating} onOpenChange={setGenerating} />}
    </section>
  )
}

function DocumentRow({ document }: { document: ProposalDocument }) {
  const t = useModuleTranslation(PROPOSALS_KEY)
  const formatting = useRecordFormatting()
  const date = formatDateTime(document.created_at, formatting)
  return (
    <li className="flex items-center gap-3 rounded-md border bg-card px-3 py-2">
      <FileText className="size-5 shrink-0 text-muted-foreground" aria-hidden="true" />
      <div className="grid min-w-0 flex-1 gap-0.5">
        <p className="truncate text-sm font-medium">{t('deal.number', { number: formatNumber(document.number) })}</p>
        <p className="truncate text-xs text-muted-foreground">
          {document.generated_by_name
            ? t('deal.generatedBy', { template: document.template_name, name: document.generated_by_name, date })
            : t('deal.generatedAt', { template: document.template_name, date })}
        </p>
      </div>
      <Button asChild size="sm" variant="outline">
        <a
          href={fileDownloadUrl(document.file_id)}
          download={document.file_name}
          aria-label={t('deal.downloadLabel', { name: document.file_name })}
        >
          <Download aria-hidden="true" />
          <span className="hidden sm:inline">{t('deal.download')}</span>
        </a>
      </Button>
    </li>
  )
}

interface GenerateDialogProps {
  dealId: string
  open: boolean
  onOpenChange: (open: boolean) => void
}

function GenerateDialog({ dealId, open, onOpenChange }: GenerateDialogProps) {
  const t = useModuleTranslation(PROPOSALS_KEY)
  const queryClient = useQueryClient()
  const invalidate = useInvalidateProposals()
  const templates = useProposalTemplates(open)
  const [chosen, setChosen] = useState<string | null>(null)
  const [showPreview, setShowPreview] = useState(false)
  const [pending, setPending] = useState(false)
  const active = (templates.data ?? []).filter((template) => template.is_active)
  const templateId = chosen ?? active.find((template) => template.is_default)?.id ?? active[0]?.id ?? null

  const preview = useQuery({
    queryKey: [...proposalsKeys.all, 'preview', 'template', templateId, dealId],
    queryFn: ({ signal }) => proposalsApi.preview({ template_id: templateId ?? undefined, deal_id: dealId }, signal),
    enabled: open && showPreview && Boolean(templateId),
  })

  const close = (next: boolean) => {
    if (pending) return
    onOpenChange(next)
    if (!next) {
      setChosen(null)
      setShowPreview(false)
    }
  }

  const generate = async () => {
    if (!templateId) return
    setPending(true)
    try {
      const document = await proposalsApi.generate(dealId, templateId)
      notifySuccess(t('deal.generated', { number: formatNumber(document.number) }))
      await Promise.all([invalidate(), queryClient.invalidateQueries({ queryKey: ['files'] })])
      setPending(false)
      close(false)
    } catch (error) {
      setPending(false)
      if (isApiError(error) && KNOWN_ERRORS.includes(error.code)) {
        notifyError(error, { title: t(`errors.${error.code}`) })
      } else {
        notifyError(error)
      }
    }
  }

  return (
    <Dialog open={open} onOpenChange={close}>
      <DialogContent size={showPreview ? '2xl' : 'md'}>
        <DialogForm
          title={t('deal.dialogTitle')}
          description={t('deal.dialogDescription')}
          cancelLabel={t('deal.cancel')}
          submitLabel={<>{pending && <Loader2 className="animate-spin" aria-hidden="true" />}
            {pending ? t('deal.generating') : t('deal.confirm')}</>}
          onCancel={() => close(false)}
          noValidate
          onSubmit={() => void generate()}
          pending={pending}
          submitDisabled={!templateId || pending}
        >
        {templates.isPending ? (
          <Skeleton className="h-9 w-full" />
        ) : templates.isError ? (
          <ErrorState onRetry={() => void templates.refetch()} />
        ) : active.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t('deal.noTemplates')}</p>
        ) : (
          <div className="grid gap-3">
            <Field label={t('deal.template')}>
              {(fieldControl) => <NativeSelect value={templateId ?? undefined} onValueChange={setChosen} disabled={pending} id={fieldControl.id}><option value="" disabled>{t('deal.templatePlaceholder')}</option>
                  {active.map((template) => (
                    <option key={template.id} value={template.id}>
                      {template.name}
                    </option>
                  ))}
                </NativeSelect>}
            </Field>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="w-fit"
              onClick={() => setShowPreview((value) => !value)}
            >
              {showPreview ? t('deal.hidePreview') : t('deal.showPreview')}
            </Button>
            {showPreview &&
              (preview.isError ? (
                <ErrorState description={t('preview.error')} onRetry={() => void preview.refetch()} />
              ) : preview.data ? (
                <ProposalDocumentView blocks={preview.data.blocks} />
              ) : (
                <Skeleton className="h-48 w-full" />
              ))}
          </div>
        )}
        </DialogForm>
      </DialogContent>
    </Dialog>
  )
}
