import { entityLabel } from '@/sdk/entities'
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
import { RequirePermission, SETTINGS_MANAGE } from '@/sdk'
import {
  DEFAULT_TAG_COLOR,
  GENERAL_VOCABULARY,
  MAX_TAG_NAME_LENGTH,
  useTaggingEntities,
  tagsApi,
  useInvalidateTags,
  useTags,
  useVocabularies,
  type Tag,
  type Vocabulary,
} from './api'
import { TagChip } from './TagChip'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'
import { Field } from '@/components/ui/field'
import { ToggleField } from '@/components/ui/toggle-field'
import { SegmentedControl } from '@/components/ui/segmented-control'
import { SelectField } from '@/components/ui/select-field'

const TERM_ERRORS = ['tag_has_children', 'tag_in_use', 'tag_slug_taken'] as const

function termTree(terms: Tag[]): { tag: Tag; depth: number }[] {
  const result: { tag: Tag; depth: number }[] = []
  const visit = (parent: string | null, depth: number) => {
    for (const tag of terms.filter((item) => (item.parent_id ?? null) === parent).sort((left, right) => left.name.localeCompare(right.name))) {
      result.push({ tag, depth })
      if (depth < 8) visit(tag.id, depth + 1)
    }
  }
  visit(null, 0)
  const listed = new Set(result.map((item) => item.tag.id))
  return [...result, ...terms.filter((tag) => !listed.has(tag.id)).map((tag) => ({ tag, depth: 0 }))]
}

function descendantIds(terms: Tag[], id: string): Set<string> {
  const found = new Set<string>([id])
  let grew = true
  while (grew) {
    grew = false
    for (const term of terms) {
      if (term.parent_id && found.has(term.parent_id) && !found.has(term.id)) {
        found.add(term.id)
        grew = true
      }
    }
  }
  return found
}

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
  const { t, i18n } = useTranslation('tagging')
  const entities = useTaggingEntities()
  const vocabularies = useVocabularies()
  const [vocabularyKey, setVocabularyKey] = useState(GENERAL_VOCABULARY)
  const vocabulary = vocabularies.data?.find((item) => item.key === vocabularyKey)
  const general = vocabularyKey === GENERAL_VOCABULARY
  const tags = useTags(undefined, { vocabulary: vocabularyKey })
  const invalidate = useInvalidateTags()
  const [editor, setEditor] = useState<EditorState | null>(null)
  const [deleting, setDeleting] = useState<Tag | null>(null)
  const vocabularyLabel = (item: Vocabulary) => (item.key === GENERAL_VOCABULARY ? t('settings.generalVocabulary') : item.name)

  const remove = async (tag: Tag) => {
    try {
      await tagsApi.remove(tag.id)
    } catch (error) {
      const code = isApiError(error) ? TERM_ERRORS.find((item) => item === error.code) : undefined
      if (!code) throw error
      const records = isApiError(error) ? Object.values(((error.details as { records?: Record<string, number> } | null)?.records) ?? {}).reduce((sum, count) => sum + count, 0) : 0
      notifyError(t(`settings.errors.${code}`, { name: tag.name, count: records }))
      return
    }
    await invalidate()
    notifySuccess(t('settings.deleted', { name: tag.name }))
  }

  return (
    <Page>
      <PageHeader
        title={t('settings.title')}
        description={t('settings.description')}
        actions={
          <Button size="sm" onClick={() => setEditor({ tag: null })} disabled={!general && !vocabulary}>
            <Plus aria-hidden="true" />
            {general ? t('settings.new') : t('settings.newTerm')}
          </Button>
        }
      />
      {(vocabularies.data?.length ?? 0) > 1 && (
        <SegmentedControl
          aria-label={t('settings.vocabularies')}
          value={vocabularyKey}
          onValueChange={setVocabularyKey}
          options={(vocabularies.data ?? []).map((item) => ({ value: item.key, label: vocabularyLabel(item) }))}
        />
      )}
      {tags.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
        </div>
      ) : tags.isError ? (
        <ErrorState onRetry={() => void tags.refetch()} />
      ) : tags.data.length === 0 ? (
        general ? (
          <EmptyState title={t('settings.emptyTitle')} description={t('settings.emptyDescription')} />
        ) : (
          <EmptyState title={t('settings.emptyTermsTitle', { vocabulary: vocabulary?.name ?? '' })} description={t('settings.emptyTermsDescription')} />
        )
      ) : !general ? (
        <ul className="grid gap-2" aria-label={vocabulary?.name}>
          {termTree(tags.data).map(({ tag, depth }) => (
            <li key={tag.id} className="flex flex-wrap items-center gap-3 rounded-md border bg-card px-3 py-2" style={{ marginInlineStart: `${depth * 1.5}rem` }}>
              <span className="min-w-0 flex-1 font-medium">{tag.name}</span>
              {tag.slug && <span className="text-xs text-muted-foreground">/{tag.slug}</span>}
              <div className="flex items-center gap-1">
                <Button size="sm" variant="ghost" aria-label={t('settings.editLabel', { name: tag.name })} onClick={() => setEditor({ tag })}>
                  <Pencil aria-hidden="true" />
                </Button>
                <Button size="sm" variant="ghost" aria-label={t('settings.deleteLabel', { name: tag.name })} onClick={() => setDeleting(tag)}>
                  <Trash2 aria-hidden="true" />
                </Button>
              </div>
            </li>
          ))}
        </ul>
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
                      {entityLabel(entities.data?.find((entity) => entity.record_type === recordType), i18n.language, recordType)}
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
            <DialogTitle>{general ? t(editor?.tag ? 'settings.editTitle' : 'settings.newTitle') : t(editor?.tag ? 'settings.editTermTitle' : 'settings.newTermTitle', { vocabulary: vocabulary?.name ?? '' })}</DialogTitle>
            <DialogDescription>{general ? t('settings.dialogDescription') : t('settings.termDialogDescription')}</DialogDescription>
          </DialogHeader>
          {editor && general && (
            <TagForm
              key={editor.tag?.id ?? 'new'}
              editing={editor.tag}
              onClose={() => setEditor(null)}
              onSaved={() => void invalidate()}
            />
          )}
          {editor && !general && vocabulary && (
            <TermForm
              key={editor.tag?.id ?? 'new'}
              vocabulary={vocabulary}
              terms={tags.data ?? []}
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
        description={
          general
            ? t('settings.deleteDescription', { name: deleting?.name ?? '', count: deleting?.usage_count ?? 0 })
            : t('settings.deleteTermDescription', { name: deleting?.name ?? '' })
        }
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
  const { t, i18n } = useTranslation('tagging')
  const entities = useTaggingEntities()
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
    if (saving || entities.isPending || entities.isError) return
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
          {entities.isPending && <Skeleton className="h-9 w-full" />}
          {entities.isError && <SharedFieldError>{t('settings.typesLoadFailed')} <Button type="button" variant="link" onClick={() => void entities.refetch()}>{t('settings.retryTypes')}</Button></SharedFieldError>}
          <div className="flex flex-wrap gap-4">
            {entities.data?.map((entity) => {
              const recordType = entity.record_type
              return (
                <ToggleField key={recordType} label={entityLabel(entity, i18n.language, recordType)} checked={recordTypes.includes(recordType)} onCheckedChange={(checked) => toggleType(recordType, checked === true)} />
              )
            })}
          </div>
        </fieldset>
      </form>
      <DialogFooter>
        <Button variant="outline" onClick={onClose} disabled={saving}>
          {t('settings.cancel')}
        </Button>
        <Button type="submit" form={formId} disabled={saving || entities.isPending || entities.isError}>
          {t('settings.save')}
        </Button>
      </DialogFooter>
    </>
  )
}

interface TermFormProps {
  vocabulary: Vocabulary
  terms: Tag[]
  editing: Tag | null
  onClose: () => void
  onSaved: () => void
}

function TermForm({ vocabulary, terms, editing, onClose, onSaved }: TermFormProps) {
  const { t } = useTranslation('tagging')
  const formId = useId()
  const [name, setName] = useState(editing?.name ?? '')
  const [slug, setSlug] = useState(editing?.slug ?? '')
  const [parent, setParent] = useState(editing?.parent_id ?? '')
  const [saving, setSaving] = useState(false)
  const [nameError, setNameError] = useState<string | null>(null)
  const [slugError, setSlugError] = useState<string | null>(null)
  const excluded = editing ? descendantIds(terms, editing.id) : new Set<string>()
  const parents = termTree(terms)
    .filter(({ tag }) => !excluded.has(tag.id))
    .map(({ tag, depth }) => ({ value: tag.id, label: `${'— '.repeat(depth)}${tag.name}` }))

  const submit = async () => {
    if (saving) return
    const trimmed = name.trim()
    if (!trimmed) {
      setNameError(t('settings.nameRequired'))
      return
    }
    setSaving(true)
    try {
      if (editing) {
        await tagsApi.update(editing.id, {
          name: trimmed,
          slug: slug.trim() || undefined,
          parent_id: vocabulary.hierarchical ? parent || null : undefined,
        })
      } else {
        await tagsApi.create({
          vocabulary: vocabulary.key,
          name: trimmed,
          slug: slug.trim() || null,
          parent_id: vocabulary.hierarchical ? parent || null : null,
        })
      }
      notifySuccess(t(editing ? 'settings.termUpdated' : 'settings.termCreated', { name: trimmed }))
      onSaved()
      onClose()
    } catch (error) {
      if (isApiError(error) && error.code === 'tag_name_taken') setNameError(t('settings.termNameTaken'))
      else if (isApiError(error) && error.code === 'tag_slug_taken') setSlugError(t('settings.slugTaken'))
      else if (isApiError(error) && error.status === 422) setSlugError(t('settings.slugInvalid'))
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
        <Field label={t('settings.name')} required error={nameError ?? undefined}>
          {(control) => (
            <Input
              {...control}
              value={name}
              maxLength={MAX_TAG_NAME_LENGTH}
              autoFocus
              onChange={(event) => {
                setName(event.target.value)
                setNameError(null)
              }}
            />
          )}
        </Field>
        <Field label={t('settings.slug')} description={t('settings.slugHelp')} error={slugError ?? undefined}>
          {(control) => (
            <Input
              {...control}
              value={slug}
              maxLength={80}
              autoComplete="off"
              spellCheck={false}
              placeholder={t('settings.slugPlaceholder')}
              onChange={(event) => {
                setSlug(event.target.value.toLowerCase())
                setSlugError(null)
              }}
            />
          )}
        </Field>
        {vocabulary.hierarchical && (
          <Field label={t('settings.parent')} description={t('settings.parentHelp')}>
            {(control) => <SelectField {...control} value={parent} onValueChange={setParent} options={parents} emptyLabel={t('settings.noParent')} />}
          </Field>
        )}
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
