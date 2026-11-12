import { useEditorialTranslation } from './translation'
import { AddBlock, BLOCK_ICONS, type Block, BlockForm, BlockImageUsageContext, type BlockType, MAX_BLOCKS, blockProblems, blockSummary, collectImageIds, duplicateBlock, imageIdsOfJson, newBlock, parseBlocks, serializeBlocks, useBlocksValidation } from '../blocks'
import { formatDateTime, useRecordFormatting } from '../records'
import { Button, ConfirmDialog, DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger, ErrorState, Input, PageLoader, SelectField, StatusBadge, toast } from '../ui'
import { useSession } from '@/app/session/session-context'
import { errorMessage } from '@/components/feedback'
import { useKeybindingHandlers, useKeybindingScope } from '@/kernel/keybindings'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowDown, ArrowLeft, ArrowUp, CalendarClock, Copy, ExternalLink, Files, GripVertical, History, Home, Menu, MoreHorizontal, Redo2, Send, Settings2, Trash2, Undo2 } from 'lucide-react'
import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams } from 'react-router'

import { OPEN_STATUSES, documentKeys, documentState, previewApi, publicationApi, readText, siteApi, siteKeys, siteTarget, textField, useDocumentKind, writeText, type DocumentRecord, type PreviewLink, type PublicationRequest, type SiteLanguage } from './document'
import { Preview, type LiveFrame } from './Preview'
import { VersionsDialog } from './VersionsDialog'
import { WhenDialog } from './WhenDialog'
import { STATE_TONES } from './states'

const SAVE_DELAY_MS = 700
const HISTORY_LIMIT = 50
const PREVIEW_RENEW_MS = 25 * 60 * 1000
const LIVE_DELAY_MS = 400

type SaveStatus = 'idle' | 'saving' | 'saved' | 'failed'

interface Snapshot {
  title: string
  blocks: Block[]
}

export function DocumentEditorPage() {
  const t = useEditorialTranslation()
  const kind = useDocumentKind()
  const keys = documentKeys(kind.recordType)
  const { id = '' } = useParams()
  const page = useQuery({ queryKey: keys.detail(id), queryFn: () => kind.get(id), refetchOnWindowFocus: false })
  const languages = useQuery({ queryKey: siteKeys.languages, queryFn: siteApi.languages, staleTime: 60_000 })
  const validation = useBlocksValidation()
  const [languageCode, setLanguageCode] = useState<string | null>(null)
  if (page.isPending || languages.isPending || validation.isPending) return <PageLoader />
  if (page.isError || !page.data || validation.isError)
    return (
      <div className="editorial-error">
        <ErrorState title={errorMessage(page.error ?? validation.error)} />
        <Button asChild variant="outline">
          <Link to={kind.listPath(page.data ?? ({} as DocumentRecord))}>{t('editor.back')}</Link>
        </Button>
      </div>
    )
  const available = languages.data ?? []
  const language = available.find((candidate) => candidate.code === languageCode) ?? available.find((candidate) => candidate.default) ?? null
  return (
    <Editor
      key={`${page.data.id}:${language?.code ?? ''}`}
      initial={page.data}
      language={language}
      languages={available}
      onLanguageChange={setLanguageCode}
    />
  )
}

interface EditorProps {
  initial: DocumentRecord
  language: SiteLanguage | null
  languages: SiteLanguage[]
  onLanguageChange: (code: string) => void
}

function languageName(code: string, locale: string): string {
  try {
    return new Intl.DisplayNames([locale], { type: 'language' }).of(code) ?? code
  } catch {
    return code
  }
}

function Editor({ initial, language, languages, onLanguageChange }: EditorProps) {
  const t = useEditorialTranslation()
  const kind = useDocumentKind()
  const keys = documentKeys(kind.recordType)
  const publicationApiOf = publicationApi(kind.recordType)
  const previews = previewApi(kind)
  const blocksField = `draft_${kind.blocks}`
  const blockLabels = useModuleTranslation('site-blocks')
  const client = useQueryClient()
  const navigate = useNavigate()
  const { hasPermission } = useSession()
  const canEdit = hasPermission(kind.updatePermission)
  const canPublish = hasPermission(kind.publishPermission)
  const canDelete = hasPermission(kind.deletePermission)
  const [record, setRecord] = useState(initial)
  const translating = language !== null && !language.default
  const prefix = language?.prefix ?? ''
  const { i18n } = useTranslation()
  const [current, setCurrent] = useState<Snapshot>(() => ({ title: readText(initial, 'draft_title', language), blocks: parseBlocks(readText(initial, blocksField, language)) }))
  const [selected, setSelected] = useState<string | null>(() => parseBlocks(readText(initial, blocksField, language))[0]?.id ?? null)
  const [past, setPast] = useState<Snapshot[]>([])
  const [future, setFuture] = useState<Snapshot[]>([])
  const [version, setVersion] = useState(0)
  const [savedVersion, setSavedVersion] = useState(0)
  const [status, setStatus] = useState<SaveStatus>('idle')
  const [busy, setBusy] = useState(false)
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [deleting, setDeleting] = useState(false)
  const [historyOpen, setHistoryOpen] = useState(false)
  const [focus, setFocus] = useState(false)
  const [showPreview, setShowPreview] = useState(false)
  const [scheduleOpen, setScheduleOpen] = useState(false)
  const formatting = useRecordFormatting()
  const [dragging, setDragging] = useState<string | null>(null)
  const [dropAt, setDropAt] = useState<number | null>(null)
  const publication = useQuery({ queryKey: keys.publication(initial.id), queryFn: () => publicationApiOf.status(initial.id) })
  const structure = useQuery({ queryKey: siteKeys.structure, queryFn: siteApi.structure, enabled: kind.structure })
  const canManageSite = kind.structure && hasPermission('settings.manage')
  const target = siteTarget(kind.recordType, initial.id)
  const isHome = structure.data?.home === target
  const inMenu = structure.data?.menu.includes(target) ?? false
  const openRequest = publication.data?.requests.find((request) => OPEN_STATUSES.includes(request.status)) ?? null
  const lastRequest = publication.data?.requests[0] ?? null
  const stopped = lastRequest && (lastRequest.status === 'blocked' || lastRequest.status === 'failed') ? lastRequest : null
  const pendingRevision = useQuery({
    queryKey: [...keys.all, 'revision', openRequest?.revision_id ?? ''],
    queryFn: () => publicationApiOf.revision(openRequest?.revision_id ?? ''),
    enabled: Boolean(openRequest?.revision_id),
  })
  const [preview, setPreview] = useState<{ link: PreviewLink; at: number; path: string } | null>(null)
  const [previewFailed, setPreviewFailed] = useState(false)
  const [nonce, setNonce] = useState(0)
  const [liveReady, setLiveReady] = useState(false)
  const [live, setLive] = useState<LiveFrame | null>(null)
  const lastRendered = useRef<string | null>(null)
  const rendering = useRef(false)
  const queuedRender = useRef<(() => void) | null>(null)
  const selectedRef = useRef(selected)
  selectedRef.current = selected
  const recordRef = useRef(record)
  recordRef.current = record
  const saving = useRef<Promise<boolean> | null>(null)
  const latest = useRef({ current, version })
  latest.current = { current, version }

  const titleMissing = !kind.Body && !translating && current.title.trim() === ''
  const invalid = titleMissing || current.blocks.some((block) => blockProblems(block).length > 0)
  const pathOf = useCallback((target: DocumentRecord) => `${prefix}${kind.path(target) ?? '/'}`, [kind, prefix])

  const remember = useCallback((updated: DocumentRecord) => {
    setRecord(updated)
    client.setQueryData(keys.detail(updated.id), updated)
    void client.invalidateQueries({ queryKey: keys.list() })
  }, [client, keys])

  const refreshPreview = useCallback(
    async (target: DocumentRecord, force = false) => {
      const stale = !preview || Date.now() - preview.at > PREVIEW_RENEW_MS || pathOf(target) !== preview.path
      if (!force && !stale) {
        if (!liveReady) setNonce((value) => value + 1)
        return
      }
      try {
        const link = await previews.link(target, pathOf(target))
        setPreview({ link, at: Date.now(), path: pathOf(target) })
        setPreviewFailed(false)
      } catch {
        setPreviewFailed(true)
      }
    },
    [preview, pathOf, previews, liveReady],
  )

  useEffect(() => {
    if (!liveReady) {
      lastRendered.current = null
      return
    }
    const timer = window.setTimeout(() => {
      const values: Record<string, string> = { title: current.title, [kind.blocks]: serializeBlocks(current.blocks) }
      const target = recordRef.current
      const key = `${prefix}\n${JSON.stringify(values)}`
      const render = () => {
        if (key === lastRendered.current) return drain()
        rendering.current = true
        lastRendered.current = key
        previews
          .render(target, values, pathOf(target))
          .then(({ html }) => setLive({ html, anchor: selectedRef.current }))
          .catch(() => {
            lastRendered.current = null
            setLiveReady(false)
          })
          .finally(drain)
      }
      const drain = () => {
        rendering.current = false
        const next = queuedRender.current
        queuedRender.current = null
        next?.()
      }
      if (rendering.current) queuedRender.current = render
      else render()
    }, LIVE_DELAY_MS)
    return () => window.clearTimeout(timer)
  }, [current, liveReady, prefix, kind.blocks, pathOf, previews])

  useEffect(() => {
    setLive((previous) => (previous && previous.anchor !== selected ? { ...previous, anchor: selected } : previous))
  }, [selected])

  const started = useRef(false)
  useEffect(() => {
    if (started.current) return
    started.current = true
    void refreshPreview(initial, true)
  }, [initial, refreshPreview])

  const save = useCallback(async (): Promise<boolean> => {
    const { current: snapshot, version: target } = latest.current
    if (target === savedVersion) return true
    setStatus('saving')
    try {
      const base = recordRef.current
      const updated = await kind.update(record.id, { ...writeText(base, 'draft_title', language, snapshot.title.trim() || kind.untitled || ''), ...writeText(base, blocksField, language, serializeBlocks(snapshot.blocks)), _expected_updated_at: base.updated_at ?? undefined })
      remember(updated)
      setSavedVersion(target)
      setStatus(latest.current.version === target ? 'saved' : 'saving')
      void refreshPreview(updated)
      return true
    } catch (failure) {
      setStatus('failed')
      toast({ title: errorMessage(failure), variant: 'destructive' })
      return false
    }
  }, [record.id, refreshPreview, remember, savedVersion, language, kind, blocksField])

  const flush = useCallback(async (): Promise<boolean> => {
    if (saving.current) await saving.current
    if (latest.current.version === savedVersion) return true
    if (invalid) return false
    const running = save()
    saving.current = running
    const result = await running
    saving.current = null
    return result
  }, [invalid, save, savedVersion])

  useEffect(() => {
    if (version === savedVersion || invalid || !canEdit) return
    const timer = window.setTimeout(() => void flush(), SAVE_DELAY_MS)
    return () => window.clearTimeout(timer)
  }, [version, savedVersion, invalid, canEdit, flush])

  useEffect(() => {
    const warn = (event: BeforeUnloadEvent) => {
      if (latest.current.version !== savedVersion) event.preventDefault()
    }
    window.addEventListener('beforeunload', warn)
    return () => window.removeEventListener('beforeunload', warn)
  }, [savedVersion])

  function change(next: Snapshot, coalesce = false) {
    if (!coalesce) {
      setPast((items) => [...items.slice(-HISTORY_LIMIT + 1), current])
      setFuture([])
    }
    setCurrent(next)
    setVersion((value) => value + 1)
  }

  const lastEdit = useRef<string | null>(null)
  function edit(key: string, next: Snapshot) {
    const coalesce = lastEdit.current === key
    lastEdit.current = key
    change(next, coalesce)
  }

  function structural(next: Snapshot) {
    lastEdit.current = null
    change(next)
  }

  function undo() {
    const previous = past[past.length - 1]
    if (!previous) return
    lastEdit.current = null
    setPast((items) => items.slice(0, -1))
    setFuture((items) => [current, ...items])
    setCurrent(previous)
    setVersion((value) => value + 1)
  }

  function redo() {
    const next = future[0]
    if (!next) return
    lastEdit.current = null
    setFuture((items) => items.slice(1))
    setPast((items) => [...items, current])
    setCurrent(next)
    setVersion((value) => value + 1)
  }

  function replaceDraft(updated: DocumentRecord) {
    lastEdit.current = null
    setPast((items) => [...items.slice(-HISTORY_LIMIT + 1), current])
    setFuture([])
    const restored = { title: readText(updated, 'draft_title', language), blocks: parseBlocks(readText(updated, blocksField, language)) }
    setCurrent(restored)
    const next = version + 1
    setVersion(next)
    setSavedVersion(next)
    latest.current = { current: restored, version: next }
  }

  function insert(type: BlockType, index: number) {
    if (current.blocks.length >= MAX_BLOCKS) {
      toast({ title: t('editor.tooMany', { count: MAX_BLOCKS }), variant: 'destructive' })
      return
    }
    const block = newBlock(type)
    const blocks = [...current.blocks]
    blocks.splice(index, 0, block)
    structural({ ...current, blocks })
    setSelected(block.id)
  }

  function move(id: string, offset: number) {
    const index = current.blocks.findIndex((block) => block.id === id)
    const target = index + offset
    if (index < 0 || target < 0 || target >= current.blocks.length) return
    const blocks = [...current.blocks]
    const [block] = blocks.splice(index, 1)
    blocks.splice(target, 0, block)
    structural({ ...current, blocks })
    window.requestAnimationFrame(() => document.getElementById(`editorial-block-${id}`)?.focus())
  }

  function moveTo(id: string, target: number) {
    const index = current.blocks.findIndex((block) => block.id === id)
    if (index < 0) return
    const destination = target > index ? target - 1 : target
    if (destination === index) return
    const blocks = [...current.blocks]
    const [block] = blocks.splice(index, 1)
    blocks.splice(destination, 0, block)
    structural({ ...current, blocks })
  }

  function duplicate(id: string) {
    const index = current.blocks.findIndex((block) => block.id === id)
    if (index < 0 || current.blocks.length >= MAX_BLOCKS) return
    const copy = duplicateBlock(current.blocks[index])
    const blocks = [...current.blocks]
    blocks.splice(index + 1, 0, copy)
    structural({ ...current, blocks })
    setSelected(copy.id)
  }

  function remove(id: string) {
    structural({ ...current, blocks: current.blocks.filter((block) => block.id !== id) })
    if (selected === id) setSelected(null)
    toast({ title: t('editor.removed') })
  }

  function updateBlock(next: Block) {
    edit(`block:${next.id}`, { ...current, blocks: current.blocks.map((block) => (block.id === next.id ? next : block)) })
  }

  function announce(request: PublicationRequest) {
    if (request.status === 'completed') toast({ title: t('editor.published'), variant: 'success' })
    else if (request.status === 'awaiting_approval') toast({ title: t('review.sent'), variant: 'success' })
    else if (request.status === 'scheduled') toast({ title: t('schedule.done'), variant: 'success' })
    else if (request.status === 'blocked' || request.status === 'failed') toast({ title: t('review.blocked', { reason: reasonText(request.reason_code) }), variant: 'destructive' })
    else toast({ title: t('review.queued'), variant: 'success' })
  }

  function reasonText(code?: string | null): string {
    const known = ['approval_rejected', 'approval_cancelled', 'publication_requester_unauthorized', 'approval_no_approver']
    return t(`review.reasons.${code && known.includes(code) ? code : 'other'}`)
  }

  async function publish(at?: string) {
    if (!(await flush())) throw new Error(t('editor.saveFailed'))
    const latest = await kind.get(record.id)
    const request = await publicationApiOf.publish(latest, at)
    const updated = await kind.get(record.id)
    remember(updated)
    await client.invalidateQueries({ queryKey: keys.publication(record.id) })
    await client.invalidateQueries({ queryKey: keys.revisions(record.id) })
    announce(request)
    void refreshPreview(updated, true)
  }

  async function publishNow() {
    setBusy(true)
    try {
      await publish()
    } catch (failure) {
      toast({ title: errorMessage(failure), variant: 'destructive' })
    } finally {
      setBusy(false)
    }
  }

  async function cancelOpen() {
    if (!openRequest) return
    setBusy(true)
    try {
      await publicationApiOf.cancel(openRequest.id)
      await client.invalidateQueries({ queryKey: keys.publication(record.id) })
      toast({ title: t('schedule.canceled'), variant: 'success' })
    } catch (failure) {
      toast({ title: errorMessage(failure), variant: 'destructive' })
    } finally {
      setBusy(false)
    }
  }

  async function changeStructure(action: () => Promise<unknown>, done: string) {
    setBusy(true)
    try {
      await action()
      await client.invalidateQueries({ queryKey: siteKeys.structure })
      toast({ title: done, variant: 'success' })
    } catch (failure) {
      toast({ title: errorMessage(failure), variant: 'destructive' })
    } finally {
      setBusy(false)
    }
  }

  async function run(action: () => Promise<DocumentRecord>, done: string) {
    setBusy(true)
    try {
      if (!(await flush())) return
      const updated = await action()
      remember(updated)
      await client.invalidateQueries({ queryKey: keys.publication(record.id) })
      toast({ title: done, variant: 'success' })
      void refreshPreview(updated, true)
    } catch (failure) {
      toast({ title: errorMessage(failure), variant: 'destructive' })
    } finally {
      setBusy(false)
    }
  }

  useKeybindingScope('editorial.editor')
  useKeybindingHandlers({
    'editorial.save': () => void flush(),
    'editorial.undo': undo,
    'editorial.redo': redo,
    'editorial.move_up': () => {
      if (selected) move(selected, -1)
    },
    'editorial.move_down': () => {
      if (selected) move(selected, 1)
    },
  })

  const state = documentState(record)
  const dirty = version !== savedVersion
  const statusText = invalid && dirty ? t('editor.saveFailed') : status === 'saving' || (dirty && status !== 'failed') ? t('editor.saving') : status === 'failed' ? t('editor.saveFailed') : status === 'saved' ? t('editor.saved') : ''

  const imagesInUse = collectImageIds(current.blocks, imageIdsOfJson(textField(record, kind.blocks), new Set()))
  const pendingFields = pendingRevision.data?.payload?.fields ?? {}
  for (const value of Object.values(pendingFields)) {
    if (typeof value === 'string') imageIdsOfJson(value, imagesInUse)
  }

  return (
    <BlockImageUsageContext.Provider value={imagesInUse}>
    <div className="editorial-editor" data-focus={focus || undefined}>
      <header className="editorial-topbar">
        <Button asChild variant="ghost" size="sm">
          <Link to={kind.listPath(record)}>
            <ArrowLeft aria-hidden />
            {t('editor.back')}
          </Link>
        </Button>
        <div className="editorial-topbar-title">
          {languages.length > 1 && (
            <SelectField
              aria-label={t('languages.label')}
              className="editorial-language"
              value={language?.code ?? ''}
              options={languages.map((candidate) => ({ value: candidate.code, label: languageName(candidate.code, i18n.language) }))}
              onValueChange={(code) =>
                void (async () => {
                  if (await flush()) onLanguageChange(code)
                })()
              }
            />
          )}
          {!kind.Body && (
          <Input
            placeholder={translating ? record.draft_title : undefined}
            aria-label={t('editor.titleLabel')}
            aria-invalid={titleMissing}
            value={current.title}
            maxLength={160}
            disabled={!canEdit}
            onChange={(event) => edit('title', { ...current, title: event.target.value })}
          />
          )}
          <StatusBadge tone={STATE_TONES[state]}>{t(`state.${state}`)}</StatusBadge>
          {isHome && (
            <StatusBadge tone="primary" icon={<Home aria-hidden />}>
              {t('list.home')}
            </StatusBadge>
          )}
          <span className="editorial-save-status" role="status" aria-live="polite">
            {statusText}
            {status === 'failed' && !invalid && (
              <Button type="button" variant="link" size="sm" onClick={() => void flush()}>
                {t('editor.retry')}
              </Button>
            )}
          </span>
        </div>
        <div className="editorial-topbar-actions">
          {!kind.Body && <Button type="button" variant="ghost" size="icon" aria-label={t('keybindings.undo')} title={t('keybindings.undo')} disabled={!past.length || !canEdit} onClick={undo}>
            <Undo2 aria-hidden />
          </Button>}
          {!kind.Body && <Button type="button" variant="ghost" size="icon" aria-label={t('keybindings.redo')} title={t('keybindings.redo')} disabled={!future.length || !canEdit} onClick={redo}>
            <Redo2 aria-hidden />
          </Button>}
          <Button type="button" variant="ghost" onClick={() => setHistoryOpen(true)}>
            <History aria-hidden />
            {t('history.open')}
          </Button>
          <Button type="button" variant="outline" onClick={() => setSettingsOpen(true)} disabled={!canEdit}>
            <Settings2 aria-hidden />
            {t('editor.settings')}
          </Button>
          {preview && (
            <Button asChild variant="outline">
              <a href={preview.link.url} target="_blank" rel="noreferrer">
                <ExternalLink aria-hidden />
                {t('editor.openPreview')}
              </a>
            </Button>
          )}
          {canPublish && state !== 'published' && (
            <Button type="button" disabled={busy || invalid} onClick={() => void publishNow()}>
              <Send aria-hidden />
              {state === 'changed' ? t('editor.publishChanges') : t('editor.publish')}
            </Button>
          )}
          {(canPublish || canDelete || hasPermission(kind.createPermission)) && (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button type="button" variant="ghost" size="icon" aria-label={t('editor.more')}>
                  <MoreHorizontal aria-hidden />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                {canPublish && (
                  <DropdownMenuItem disabled={busy || invalid} onSelect={() => setScheduleOpen(true)}>
                    <CalendarClock aria-hidden />
                    {t('schedule.open')}
                  </DropdownMenuItem>
                )}
                {canPublish && openRequest && (
                  <DropdownMenuItem disabled={busy} onSelect={() => void cancelOpen()}>
                    {openRequest.status === 'scheduled' ? t('schedule.cancel') : t('review.cancel')}
                  </DropdownMenuItem>
                )}
                {canManageSite && record.publication_state === 'published' && !isHome && (
                  <DropdownMenuItem disabled={busy} onSelect={() => void changeStructure(() => siteApi.setHome(target), t('editor.homeChanged'))}>
                    <Home aria-hidden />
                    {t('editor.makeHome')}
                  </DropdownMenuItem>
                )}
                {canManageSite && record.publication_state === 'published' && (
                  <DropdownMenuItem disabled={busy} onSelect={() => void changeStructure(() => siteApi.setMenu(target, !inMenu), inMenu ? t('editor.menuRemoved') : t('editor.menuAdded'))}>
                    <Menu aria-hidden />
                    {inMenu ? t('editor.removeFromMenu') : t('editor.addToMenu')}
                  </DropdownMenuItem>
                )}
                {canPublish && record.publication_state === 'published' && (
                  <DropdownMenuItem disabled={busy} onSelect={() => void run(async () => {
                    await publicationApiOf.withdraw(record.id)
                    return kind.get(record.id)
                  }, t('editor.unpublished'))}>
                    {t('editor.unpublish')}
                  </DropdownMenuItem>
                )}
                {kind.duplicate && hasPermission(kind.createPermission) && (
                  <DropdownMenuItem
                    disabled={busy}
                    onSelect={() =>
                      void (async () => {
                        setBusy(true)
                        try {
                          if (!(await flush())) return
                          const copy = await kind.duplicate!(record.id)
                          await client.invalidateQueries({ queryKey: keys.list() })
                          toast({ title: t('editor.duplicated'), variant: 'success' })
                          navigate(kind.editPath(copy.id))
                        } catch (failure) {
                          toast({ title: errorMessage(failure), variant: 'destructive' })
                        } finally {
                          setBusy(false)
                        }
                      })()
                    }
                  >
                    <Files aria-hidden />
                    {t('editor.duplicate_page')}
                  </DropdownMenuItem>
                )}
                {canDelete && (
                  <>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem variant="destructive" onSelect={() => setDeleting(true)}>
                      <Trash2 aria-hidden />
                      {t('editor.delete')}
                    </DropdownMenuItem>
                  </>
                )}
              </DropdownMenuContent>
            </DropdownMenu>
          )}
        </div>
      </header>
      <div className="editorial-notice" role="note">
        {record.publication_state === 'published' && preview ? (
          <>
            {t('editor.liveAt')}{' '}
            <a href={preview.link.page_url} target="_blank" rel="noreferrer">
              {preview.link.page_url.replace(/^https?:\/\//, '')}
            </a>
            {state === 'changed' && <strong> · {t('editor.pendingChanges')}</strong>}
          </>
        ) : record.publication_state === 'published' ? null : (
          <>
            {t('editor.notLive')}
            {!canPublish && ` ${t('editor.noPublishPermission')}`}
          </>
        )}
        {isHome && <span> · {t('editor.isHome')}</span>}
        {openRequest?.status === 'scheduled' && openRequest.run_at && (
          <strong> · {t('schedule.notice', { date: formatDateTime(openRequest.run_at, formatting) })}</strong>
        )}
        {openRequest?.status === 'awaiting_approval' && (
          <>
            <strong> · {t('review.notice')}</strong>
            <Button asChild variant="link" size="sm">
              <Link to="/approvals">{t('review.open')}</Link>
            </Button>
          </>
        )}
        {!openRequest && stopped && <strong> · {t('review.blocked', { reason: reasonText(stopped.reason_code) })}</strong>}
      </div>
      <div className="editorial-workspace" data-flow={kind.Body ? '' : undefined}>
        {kind.Body ? (
          <kind.Body
            record={record}
            recordType={kind.recordType}
            title={current.title === kind.untitled ? '' : current.title}
            blocks={current.blocks}
            disabled={!canEdit}
            translating={translating}
            onTitleChange={(title) => edit('title', { ...current, title })}
            onBlocksChange={(blocks) => edit('body', { ...current, blocks })}
            focus={focus}
            onFocusChange={setFocus}
            preview={showPreview}
            onPreviewChange={setShowPreview}
          />
        ) : (
        <section className="editorial-blocks" aria-labelledby="editorial-blocks-title">
          <div className="editorial-blocks-header">
            <h2 id="editorial-blocks-title">{t('editor.blocks')}</h2>
            <p className="editorial-muted">{t('editor.blocksHelp')}</p>
          </div>
          {current.blocks.length === 0 && !translating && <p className="editorial-muted">{t('editor.empty')}</p>}
          {current.blocks.length === 0 && translating && canEdit && (
            <div className="editorial-translate">
              <p>{t('languages.empty', { language: languageName(language.code, i18n.language) })}</p>
              <Button
                type="button"
                variant="outline"
                onClick={() =>
                  structural({
                    title: current.title || record.draft_title,
                    blocks: parseBlocks(textField(record, blocksField)),
                  })
                }
              >
                {t('languages.copy', { language: languageName(languages.find((candidate) => candidate.default)?.code ?? '', i18n.language) })}
              </Button>
            </div>
          )}
          <ol className="editorial-block-list">
            {current.blocks.map((block, index) => {
              const Icon = BLOCK_ICONS[block.type]
              const open = selected === block.id
              const problems = blockProblems(block)
              const problem = problems.length > 0
              return (
                <li
                  key={block.id}
                  className="editorial-block"
                  data-open={open || undefined}
                  data-problem={problem || undefined}
                  data-dragging={dragging === block.id || undefined}
                  data-drop={dragging !== null && dropAt === index ? 'before' : dragging !== null && dropAt === index + 1 && index === current.blocks.length - 1 ? 'after' : undefined}
                  onDragOver={(event) => {
                    if (!dragging) return
                    event.preventDefault()
                    const box = event.currentTarget.getBoundingClientRect()
                    setDropAt(event.clientY < box.top + box.height / 2 ? index : index + 1)
                  }}
                  onDrop={(event) => {
                    event.preventDefault()
                    if (dragging && dropAt !== null) moveTo(dragging, dropAt)
                    setDragging(null)
                    setDropAt(null)
                  }}
                >
                  {canEdit && <AddBlock compact onAdd={(type) => insert(type, index)} />}
                  <div className="editorial-block-card">
                    <div className="editorial-block-head">
                      {canEdit && (
                        <span
                          className="editorial-block-grip"
                          draggable
                          title={t('editor.drag')}
                          aria-hidden
                          onDragStart={(event) => {
                            event.dataTransfer.effectAllowed = 'move'
                            event.dataTransfer.setData('text/plain', block.id)
                            setDragging(block.id)
                          }}
                          onDragEnd={() => {
                            setDragging(null)
                            setDropAt(null)
                          }}
                        >
                          <GripVertical size={16} />
                        </span>
                      )}
                      <button
                        id={`editorial-block-${block.id}`}
                        type="button"
                        className="editorial-block-toggle"
                        aria-expanded={open}
                        onClick={() => setSelected(open ? null : block.id)}
                        onFocus={() => setSelected((value) => value ?? block.id)}
                      >
                        <Icon aria-hidden size={18} />
                        <span className="editorial-block-name">{blockLabels(`types.${block.type}`)}</span>
                        <span className="editorial-block-summary">{blockSummary(block)}</span>
                      </button>
                      {canEdit && (
                        <div className="editorial-block-tools">
                          <Button type="button" variant="ghost" size="icon" aria-label={t('editor.moveUp')} title={t('editor.moveUp')} disabled={index === 0} onClick={() => move(block.id, -1)}>
                            <ArrowUp aria-hidden />
                          </Button>
                          <Button type="button" variant="ghost" size="icon" aria-label={t('editor.moveDown')} title={t('editor.moveDown')} disabled={index === current.blocks.length - 1} onClick={() => move(block.id, 1)}>
                            <ArrowDown aria-hidden />
                          </Button>
                          <Button type="button" variant="ghost" size="icon" aria-label={t('editor.duplicate')} title={t('editor.duplicate')} onClick={() => duplicate(block.id)}>
                            <Copy aria-hidden />
                          </Button>
                          <Button type="button" variant="ghost" size="icon" aria-label={t('editor.remove')} title={t('editor.remove')} onClick={() => remove(block.id)}>
                            <Trash2 aria-hidden />
                          </Button>
                        </div>
                      )}
                    </div>
                    {problem && <p className="editorial-block-problem">{problems.map((item) => t(`problems.${item}`)).join(' ')}</p>}
                    {open && (
                      <div className="editorial-block-body">
                        <BlockForm block={block} record={{ recordType: kind.recordType, recordId: record.id }} disabled={!canEdit} onChange={updateBlock} />
                      </div>
                    )}
                  </div>
                </li>
              )
            })}
          </ol>
          {canEdit && <AddBlock onAdd={(type) => insert(type, current.blocks.length)} />}
        </section>
        )}
        {(!kind.Body || showPreview) && <Preview link={preview?.link ?? null} failed={previewFailed} nonce={nonce} anchor={selected} live={live} onLiveChange={setLiveReady} />}
      </div>
      <kind.Settings
        language={language}
        open={settingsOpen}
        record={record}
        onOpenChange={setSettingsOpen}
        onSaved={(updated) => {
          remember(updated)
          void refreshPreview(updated, true)
        }}
      />
      <VersionsDialog
        open={historyOpen}
        record={record}
        activeRevisionId={publication.data?.active_revision?.id ?? null}
        onOpenChange={setHistoryOpen}
        beforeRestore={flush}
        onChanged={(updated, draftReplaced) => {
          remember(updated)
          if (draftReplaced) replaceDraft(updated)
          void refreshPreview(updated, true)
        }}
      />
      <WhenDialog
        open={scheduleOpen}
        title={t('schedule.open')}
        description={t('schedule.description')}
        submitLabel={t('schedule.submit')}
        onOpenChange={setScheduleOpen}
        onSubmit={async (at) => {
          await publish(at)
        }}
      />
      <ConfirmDialog
        open={deleting}
        onOpenChange={setDeleting}
        title={t('remove.title')}
        description={t('remove.description')}
        confirmLabel={t('remove.confirm')}
        variant="destructive"
        onConfirm={async () => {
          await kind.remove(record.id)
          await client.invalidateQueries({ queryKey: keys.list() })
          toast({ title: t('remove.done'), variant: 'success' })
          navigate(kind.listPath(record))
        }}
      />
    </div>
    </BlockImageUsageContext.Provider>
  )
}
