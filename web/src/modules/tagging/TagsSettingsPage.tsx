import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { Pencil, Plus, Trash2 } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import {
  DEFAULT_TAG_COLOR,
  MAX_TAG_NAME_LENGTH,
  TAGGABLE_RECORD_TYPES,
  tagsApi,
  useInvalidateTags,
  useTags,
  type Tag,
  type TaggableRecordType,
} from './api'
import { TagChip } from './TagChip'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'
import { Field } from '@/components/ui/field'
import { ToggleField } from '@/components/ui/toggle-field'

const HEX_COLOR = /^#[0-9a-f]{6}$/i

export function TagsSettingsPage() {
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <TagsSection />
    </RequirePermission>
  )
}

interface EditorState {
  tag: Tag | null
}

function TagsSection() {
  const { t } = useTranslation('tagging')
  const tags = useTags()
  const invalidate = useInvalidateTags()
  const [editor, setEditor] = useState<EditorState | null>(null)
  const [deleting, setDeleting] = useState<Tag | null>(null)

  const remove = async (tag: Tag) => {
    await tagsApi.remove(tag.id)
    await invalidate()
    notifySuccess(t('settings.deleted', { name: tag.name }))
  }

  return (
    <Page>
      <PageHeader
        title={t('settings.title')}
        description={t('settings.description')}
        actions={
          <Button size="sm" onClick={() => setEditor({ tag: null })}>
            <Plus aria-hidden="true" />
            {t('settings.new')}
          </Button>
        }
      />
      {tags.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
        </div>
      ) : tags.isError ? (
        <ErrorState onRetry={() => void tags.refetch()} />
      ) : tags.data.length === 0 ? (
        <EmptyState title={t('settings.emptyTitle')} description={t('settings.emptyDescription')} />
      ) : (
        <ul className="grid gap-2" aria-label={t('settings.listLabel')}>
          {tags.data.map((tag) => (
            <li key={tag.id} className="flex flex-wrap items-center gap-3 rounded-md border bg-card px-3 py-2">
              <TagChip name={tag.name} color={tag.color} className="text-sm" />
              <div className="flex min-w-0 flex-1 flex-wrap items-center gap-1.5">
                {tag.record_types.length === 0 ? (
                  <Badge variant="secondary">{t('settings.allTypes')}</Badge>
                ) : (
                  tag.record_types.map((recordType) => (
                    <Badge key={recordType} variant="outline">
                      {t(`recordTypes.${recordType as TaggableRecordType}`)}
                    </Badge>
                  ))
                )}
                <span className="text-xs text-muted-foreground">{t('settings.usage', { count: tag.usage_count })}</span>
              </div>
              <div className="flex items-center gap-1">
                <Button
                  size="sm"
                  variant="ghost"
                  aria-label={t('settings.editLabel', { name: tag.name })}
                  onClick={() => setEditor({ tag })}
                >
                  <Pencil aria-hidden="true" />
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  aria-label={t('settings.deleteLabel', { name: tag.name })}
                  onClick={() => setDeleting(tag)}
                >
                  <Trash2 aria-hidden="true" />
                </Button>
              </div>
            </li>
          ))}
        </ul>
      )}
      <Dialog open={editor !== null} onOpenChange={(open) => !open && setEditor(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t(editor?.tag ? 'settings.editTitle' : 'settings.newTitle')}</DialogTitle>
            <DialogDescription>{t('settings.dialogDescription')}</DialogDescription>
          </DialogHeader>
          {editor && (
            <TagForm
              key={editor.tag?.id ?? 'new'}
              editing={editor.tag}
              onClose={() => setEditor(null)}
              onSaved={() => void invalidate()}
            />
          )}
        </DialogContent>
      </Dialog>
      <ConfirmDialog
        open={deleting !== null}
        onOpenChange={(open) => !open && setDeleting(null)}
        title={t('settings.deleteTitle')}
        description={t('settings.deleteDescription', {
          name: deleting?.name ?? '',
          count: deleting?.usage_count ?? 0,
        })}
        confirmLabel={t('settings.deleteConfirm')}
        variant="destructive"
        onConfirm={() => (deleting ? remove(deleting) : undefined)}
      />
    </Page>
  )
}

interface TagFormProps {
  editing: Tag | null
  onClose: () => void
  onSaved: () => void
}

function TagForm({ editing, onClose, onSaved }: TagFormProps) {
  const { t } = useTranslation('tagging')
  const formId = useId()
  const [name, setName] = useState(editing?.name ?? '')
  const [color, setColor] = useState(editing?.color ?? DEFAULT_TAG_COLOR)
  const [recordTypes, setRecordTypes] = useState<string[]>(editing?.record_types ?? [])
  const [saving, setSaving] = useState(false)
  const [nameError, setNameError] = useState<string | null>(null)

  const toggleType = (recordType: string, checked: boolean) =>
    setRecordTypes((current) =>
      checked ? [...current.filter((item) => item !== recordType), recordType] : current.filter((item) => item !== recordType),
    )

  const submit = async () => {
    const trimmed = name.trim()
    if (!trimmed) {
      setNameError(t('settings.nameRequired'))
      return
    }
    setSaving(true)
    const body = {
      name: trimmed,
      color: HEX_COLOR.test(color) ? color : DEFAULT_TAG_COLOR,
      record_types: recordTypes,
    }
    try {
      if (editing) await tagsApi.update(editing.id, body)
      else await tagsApi.create(body)
      notifySuccess(t(editing ? 'settings.updated' : 'settings.created', { name: trimmed }))
      onSaved()
      onClose()
    } catch (error) {
      if (isApiError(error) && error.code === 'tag_name_taken') setNameError(t('settings.nameTaken'))
      else notifyError(error)
    } finally {
      setSaving(false)
    }
  }

  return (
    <>
      <form
        id={formId}
        className="grid gap-4"
        noValidate
        onSubmit={(event) => {
          event.preventDefault()
          void submit()
        }}
      >
        <Field label={t('settings.name')}>
          {(control) => <>
            <Input placeholder={t('placeholders.name')}
            {...control}
            value={name}
            maxLength={MAX_TAG_NAME_LENGTH}
            autoFocus
            aria-invalid={nameError ? true : undefined}
            aria-describedby={nameError ? `${formId}-name-error` : undefined}
            onChange={(event) => {
              setName(event.target.value)
              setNameError(null)
            }}
          />
          {nameError && (
            <SharedFieldError id={`${formId}-name-error`}>
              {nameError}
            </SharedFieldError>
          )}
          </>}
        </Field>
        <Field id={`${formId}-color`} label={t('settings.color')}>
          <div className="flex items-center gap-2">
            <input
              id={`${formId}-color`}
              type="color"
              className="h-9 w-12 shrink-0 cursor-pointer rounded-md border border-input bg-background p-1"
              value={HEX_COLOR.test(color) ? color : DEFAULT_TAG_COLOR}
              onChange={(event) => setColor(event.target.value)}
            />
            <Input placeholder={t('placeholders.colorHex')} aria-label={t('settings.colorHex')} value={color} onChange={(event) => setColor(event.target.value)} />
          </div>
        </Field>
        <fieldset className="grid gap-2">
          <legend className="text-sm font-medium">{t('settings.recordTypes')}</legend>
          <p className="text-xs text-muted-foreground">{t('settings.recordTypesHint')}</p>
          <div className="flex flex-wrap gap-4">
            {TAGGABLE_RECORD_TYPES.map((recordType) => {
              return (
                <ToggleField key={recordType} label={t(`recordTypes.${recordType}`)} checked={recordTypes.includes(recordType)} onCheckedChange={(checked) => toggleType(recordType, checked === true)} />
              )
            })}
          </div>
        </fieldset>
      </form>
      <DialogFooter>
        <Button variant="outline" onClick={onClose} disabled={saving}>
          {t('settings.cancel')}
        </Button>
        <Button type="submit" form={formId} disabled={saving}>
          {t('settings.save')}
        </Button>
      </DialogFooter>
    </>
  )
}
