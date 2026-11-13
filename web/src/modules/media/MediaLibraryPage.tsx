import { useQueryClient } from '@tanstack/react-query'
import { Folder, FolderOpen, FolderPlus, Images, LayoutGrid, List, Pencil, Trash2, Upload, X } from 'lucide-react'
import { useRef, useState, type DragEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { ConfirmDialog } from '@/components/ui/confirm-dialog'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { NativeSelect } from '@/components/ui/native-select'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { SearchInput } from '@/components/ui/search-input'
import { SegmentedControl } from '@/components/ui/segmented-control'
import { Skeleton } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'
import {
  MEDIA_MANAGE,
  ROOT_FOLDER,
  mediaApi,
  mediaKeys,
  type MediaAsset,
  type MediaFolder,
  type MediaKind,
  type MediaSort,
  useMediaFolders,
  useMediaLibrary,
} from '@/sdk/media/api'
import { MediaThumbnail } from '@/sdk/media/MediaPicker'
import { AssetDialog } from './AssetDialog'
import { FolderDialog } from './FolderDialog'
import { flattenFolders, folderPath } from './folders'

type View = 'grid' | 'list'

function typeLabel(asset: MediaAsset, fallback: string): string {
  const type = asset.content_type ?? ''
  if (type.startsWith('image/')) return type.slice(6).toUpperCase().replace('SVG+XML', 'SVG')
  const subtype = type.split('/')[1]?.split(/[.+;]/).pop()
  return subtype ? subtype.toUpperCase().slice(0, 6) : fallback
}

export function MediaLibraryPage() {
  const { t, i18n } = useTranslation('media')
  const { hasPermission } = useSession()
  const canManage = hasPermission(MEDIA_MANAGE)
  const client = useQueryClient()
  const input = useRef<HTMLInputElement>(null)
  const [search, setSearch] = useState('')
  const [kind, setKind] = useState<MediaKind>('all')
  const [sort, setSort] = useState<MediaSort>('-created_at')
  const [view, setView] = useState<View>('grid')
  const [folder, setFolder] = useState<string>('all')
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null)
  const [editing, setEditing] = useState<MediaAsset | null>(null)
  const [folderDialog, setFolderDialog] = useState<{ folder?: MediaFolder; parentId?: string | null } | null>(null)
  const [deletingFolder, setDeletingFolder] = useState<MediaFolder | null>(null)
  const [selected, setSelected] = useState<string[]>([])
  const [moving, setMoving] = useState(false)
  const [deletingSelected, setDeletingSelected] = useState(false)
  const [dragging, setDragging] = useState(false)
  const folders = useMediaFolders()
  const allFolders = folders.data ?? []
  const tree = flattenFolders(allFolders)
  const assets = useMediaLibrary({ search, kind, sort, folder })
  const list = assets.data ?? []
  const current = allFolders.find((item) => item.id === folder)
  const targetFolder = current?.id ?? null
  const filtered = search.trim() !== '' || kind !== 'all'
  const dateFormat = new Intl.DateTimeFormat(i18n.language, { dateStyle: 'medium' })

  async function upload(files: File[]) {
    if (!canManage || files.length === 0) return
    setProgress({ done: 0, total: files.length })
    let done = 0
    for (const file of files) {
      try {
        await mediaApi.create(file, targetFolder)
        done += 1
        setProgress({ done, total: files.length })
      } catch (error) {
        notifyError(error)
      }
    }
    setProgress(null)
    if (input.current) input.current.value = ''
    await client.invalidateQueries({ queryKey: mediaKeys.all })
    if (done > 0) notifySuccess(t('library.uploaded', { count: done }))
  }

  function drop(event: DragEvent) {
    event.preventDefault()
    setDragging(false)
    void upload([...event.dataTransfer.files])
  }

  function toggle(id: string) {
    setSelected((items) => (items.includes(id) ? items.filter((item) => item !== id) : [...items, id]))
  }

  function pickFolder(value: string) {
    setFolder(value)
    setSelected([])
  }

  async function moveSelected(target: string) {
    setMoving(true)
    try {
      for (const id of selected) await mediaApi.update(id, { folder_id: target || null })
      await client.invalidateQueries({ queryKey: mediaKeys.all })
      notifySuccess(t('bulk.moved', { count: selected.length }))
      setSelected([])
    } catch (error) {
      notifyError(error)
    } finally {
      setMoving(false)
    }
  }

  async function deleteSelected() {
    let removed = 0
    let blocked = 0
    for (const id of selected) {
      try {
        await mediaApi.remove(id)
        removed += 1
      } catch {
        blocked += 1
      }
    }
    await client.invalidateQueries({ queryKey: mediaKeys.all })
    setSelected([])
    if (removed > 0) notifySuccess(t('bulk.deleted', { count: removed }))
    if (blocked > 0) notifyError(new Error(t('bulk.blocked', { count: blocked })))
  }

  const heading = current ? folderPath(allFolders, current.id).join(' / ') : folder === ROOT_FOLDER ? t('folders.withoutFolder') : t('folders.all')

  const sidebarItem = (value: string, label: string, depth: number, icon: 'all' | 'folder') => {
    const active = folder === value
    const Icon = icon === 'all' ? Images : active ? FolderOpen : Folder
    return (
      <li key={value}>
        <button
          type="button"
          aria-current={active ? 'true' : undefined}
          onClick={() => pickFolder(value)}
          style={{ paddingLeft: `${0.75 + depth * 1}rem` }}
          className={cn(
            'flex min-h-9 w-full items-center gap-2 rounded-md pr-3 text-left text-sm transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none',
            active && 'bg-muted font-medium',
          )}
        >
          <Icon className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
          <span className="truncate">{label}</span>
        </button>
      </li>
    )
  }

  return (
    <Page>
      <PageHeader
        title={t('library.title')}
        description={t('library.description')}
        actions={
          canManage ? (
            <>
              <input
                ref={input}
                type="file"
                multiple
                className="hidden"
                tabIndex={-1}
                aria-hidden
                onChange={(event) => void upload([...(event.target.files ?? [])])}
              />
              <Button variant="outline" onClick={() => setFolderDialog({ parentId: targetFolder })}>
                <FolderPlus aria-hidden />
                {t('folders.new')}
              </Button>
              <Button disabled={progress !== null} onClick={() => input.current?.click()}>
                <Upload aria-hidden />
                {progress ? t('library.uploading', progress) : t('library.upload')}
              </Button>
            </>
          ) : undefined
        }
      />
      <div className="grid gap-6 lg:grid-cols-[15rem_minmax(0,1fr)]">
        <nav aria-label={t('folders.title')} className="lg:sticky lg:top-4 lg:self-start">
          <NativeSelect className="w-full lg:hidden" aria-label={t('folders.title')} value={folder} onValueChange={pickFolder}>
            <option value="all">{t('folders.all')}</option>
            <option value={ROOT_FOLDER}>{t('folders.withoutFolder')}</option>
            {tree.map((item) => (
              <option key={item.id} value={item.id}>
                {'  '.repeat(item.depth)}
                {item.name}
              </option>
            ))}
          </NativeSelect>
          <ul className="hidden gap-0.5 lg:grid">
            {sidebarItem('all', t('folders.all'), 0, 'all')}
            {sidebarItem(ROOT_FOLDER, t('folders.withoutFolder'), 0, 'folder')}
            {tree.map((item) => sidebarItem(item.id, item.name, item.depth, 'folder'))}
          </ul>
        </nav>
        <section
          aria-label={heading}
          className={cn('grid min-w-0 content-start gap-4 rounded-lg', dragging && 'ring-2 ring-primary ring-offset-4')}
          onDragOver={(event) => {
            if (!canManage || !event.dataTransfer.types.includes('Files')) return
            event.preventDefault()
            setDragging(true)
          }}
          onDragLeave={(event) => {
            if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setDragging(false)
          }}
          onDrop={drop}
        >
          <div className="flex flex-wrap items-center justify-between gap-2">
            <h2 className="min-w-0 truncate text-base font-semibold">{heading}</h2>
            {canManage && current && (
              <div className="flex gap-1">
                <Button size="sm" variant="ghost" onClick={() => setFolderDialog({ folder: current })}>
                  <Pencil aria-hidden />
                  {t('folders.rename')}
                </Button>
                <Button size="sm" variant="ghost" className="text-destructive" onClick={() => setDeletingFolder(current)}>
                  <Trash2 aria-hidden />
                  {t('folders.delete')}
                </Button>
              </div>
            )}
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <SearchInput value={search} onValueChange={setSearch} placeholder={t('library.search')} aria-label={t('library.search')} wrapperClassName="flex-1 min-w-52" />
            <SegmentedControl
              aria-label={t('library.kind')}
              value={kind}
              onValueChange={setKind}
              options={[
                { value: 'all', label: t('library.kinds.all') },
                { value: 'images', label: t('library.kinds.images') },
                { value: 'documents', label: t('library.kinds.documents') },
              ]}
            />
            <NativeSelect className="w-44" aria-label={t('library.sort')} value={sort} onValueChange={(value) => setSort(value as MediaSort)}>
              <option value="-created_at">{t('library.sorts.newest')}</option>
              <option value="created_at">{t('library.sorts.oldest')}</option>
              <option value="name">{t('library.sorts.name')}</option>
            </NativeSelect>
            <SegmentedControl
              aria-label={t('library.view')}
              value={view}
              onValueChange={setView}
              options={[
                { value: 'grid', label: <LayoutGrid className="size-4" aria-hidden />, ariaLabel: t('library.views.grid') },
                { value: 'list', label: <List className="size-4" aria-hidden />, ariaLabel: t('library.views.list') },
              ]}
            />
          </div>
          {canManage && selected.length > 0 && (
            <div role="status" className="flex flex-wrap items-center gap-2 rounded-md border bg-muted/40 px-3 py-2 text-sm">
              <span className="font-medium">{t('bulk.selected', { count: selected.length })}</span>
              <NativeSelect className="w-52" aria-label={t('bulk.move')} disabled={moving} value="" onValueChange={(value) => void moveSelected(value === ROOT_FOLDER ? '' : value)}>
                <option value="" disabled>
                  {t('bulk.move')}
                </option>
                <option value={ROOT_FOLDER}>{t('folders.root')}</option>
                {tree.map((item) => (
                  <option key={item.id} value={item.id}>
                    {'  '.repeat(item.depth)}
                    {item.name}
                  </option>
                ))}
              </NativeSelect>
              <Button size="sm" variant="outline" className="text-destructive" onClick={() => setDeletingSelected(true)}>
                <Trash2 aria-hidden />
                {t('bulk.delete')}
              </Button>
              <Button size="sm" variant="ghost" className="ml-auto" onClick={() => setSelected([])}>
                <X aria-hidden />
                {t('bulk.clear')}
              </Button>
            </div>
          )}
          {assets.isPending ? (
            <Skeleton className="h-64" />
          ) : assets.isError ? (
            <ErrorState onRetry={() => void assets.refetch()} />
          ) : list.length === 0 ? (
            <EmptyState
              title={filtered ? t('library.noResults') : current ? t('library.emptyFolder') : t('library.empty')}
              description={filtered ? undefined : canManage ? t('library.emptyDescription') : undefined}
            />
          ) : view === 'grid' ? (
            <ul className="grid grid-cols-2 gap-4 sm:grid-cols-3 xl:grid-cols-4">
              {list.map((asset) => (
                <li key={asset.id} className="relative">
                  <button
                    type="button"
                    className={cn(
                      'grid w-full gap-1 rounded-md border bg-card p-1.5 text-left text-sm hover:border-primary focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none',
                      selected.includes(asset.id) && 'border-primary',
                    )}
                    aria-label={t('library.edit', { name: asset.name })}
                    onClick={() => setEditing(asset)}
                  >
                    <MediaThumbnail asset={asset} className="flex aspect-[4/3] w-full items-center justify-center rounded-sm bg-muted object-cover text-muted-foreground" />
                    <span className="truncate px-1">{asset.name}</span>
                  </button>
                  {canManage && (
                    <Checkbox
                      className="absolute top-3 left-3 bg-background"
                      aria-label={t('bulk.select', { name: asset.name })}
                      checked={selected.includes(asset.id)}
                      onCheckedChange={() => toggle(asset.id)}
                    />
                  )}
                </li>
              ))}
            </ul>
          ) : (
            <ul className="grid gap-1">
              {list.map((asset) => (
                <li key={asset.id} className="flex items-center gap-3 rounded-md border bg-card p-2">
                  {canManage && (
                    <Checkbox aria-label={t('bulk.select', { name: asset.name })} checked={selected.includes(asset.id)} onCheckedChange={() => toggle(asset.id)} />
                  )}
                  <button
                    type="button"
                    className="grid min-w-0 flex-1 grid-cols-[3rem_minmax(0,1fr)] items-center gap-3 text-left text-sm focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none sm:grid-cols-[3rem_minmax(0,1fr)_5rem_7rem]"
                    aria-label={t('library.edit', { name: asset.name })}
                    onClick={() => setEditing(asset)}
                  >
                    <MediaThumbnail asset={asset} className="flex size-12 items-center justify-center rounded-sm bg-muted object-cover text-muted-foreground" />
                    <span className="truncate font-medium">{asset.name}</span>
                    <span className="hidden text-xs text-muted-foreground sm:block">{typeLabel(asset, t('library.file'))}</span>
                    <span className="hidden text-xs text-muted-foreground sm:block">{dateFormat.format(new Date(asset.created_at))}</span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>
      {editing && <AssetDialog key={editing.id} asset={editing} canManage={canManage} onClose={() => setEditing(null)} />}
      {folderDialog && (
        <FolderDialog
          folders={allFolders}
          folder={folderDialog.folder}
          parentId={folderDialog.parentId}
          onClose={(saved) => {
            setFolderDialog(null)
            if (saved && !folderDialog.folder) setFolder(saved.id)
          }}
        />
      )}
      <ConfirmDialog
        open={deletingFolder !== null}
        onOpenChange={(open) => !open && setDeletingFolder(null)}
        title={t('folders.deleteTitle', { name: deletingFolder?.name ?? '' })}
        description={t('folders.deleteDescription')}
        confirmLabel={t('folders.delete')}
        variant="destructive"
        onError={() => notifyError(new Error(t('folders.notEmpty')))}
        onConfirm={async () => {
          if (!deletingFolder) return
          await mediaApi.removeFolder(deletingFolder.id)
          await client.invalidateQueries({ queryKey: mediaKeys.all })
          setFolder(deletingFolder.parent_id ?? 'all')
          notifySuccess(t('folders.deleted'))
        }}
      />
      <ConfirmDialog
        open={deletingSelected}
        onOpenChange={setDeletingSelected}
        title={t('bulk.deleteTitle', { count: selected.length })}
        description={t('bulk.deleteDescription')}
        confirmLabel={t('bulk.delete')}
        variant="destructive"
        onConfirm={deleteSelected}
      />
    </Page>
  )
}
