import {
  DndContext,
  KeyboardSensor,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
  type Announcements,
  type DragEndEvent,
  type UniqueIdentifier,
} from '@dnd-kit/core'
import {
  SortableContext,
  arrayMove,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { ArrowDown, ArrowUp, ChevronDown, ChevronRight, GripVertical, MoreHorizontal, StarOff } from 'lucide-react'
import { createElement, useId, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { NavLink } from 'react-router'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { readStorage, writeStorage } from '@/lib/storage'
import { cn } from '@/lib/utils'
import { useFavoriteMutations, useFavorites, type Favorite } from './api'
import { favoriteHref, favoriteIcon } from './links'

const SECTION_OPEN_KEY = 'aphel.sidebar.favorites.open'

const linkClasses =
  'flex h-8 min-w-0 flex-1 items-center gap-3 rounded-md px-2 text-sm text-sidebar-foreground transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none'

export interface SidebarFavoritesProps {
  collapsed?: boolean
  onNavigate?: () => void
}

function useFavoriteLabel() {
  const { t } = useTranslation('favorites')
  return (favorite: Favorite) => (favorite.available && favorite.title ? favorite.title : t('sidebar.unavailable'))
}

function CollapsedFavorites({ favorites, onNavigate }: { favorites: Favorite[]; onNavigate?: () => void }) {
  const { t } = useTranslation('favorites')
  const labelOf = useFavoriteLabel()
  const available = favorites.filter((favorite) => favoriteHref(favorite))
  if (available.length === 0) return null

  return (
    <nav aria-label={t('sidebar.label')} className="mt-3 flex flex-col gap-1 border-t border-sidebar-border px-2 pt-3">
      {available.map((favorite) => {
        const Icon = favoriteIcon(favorite)
        const label = labelOf(favorite)
        return (
          <Tooltip key={favorite.id}>
            <TooltipTrigger asChild>
              <NavLink
                to={favoriteHref(favorite) ?? '#'}
                onClick={onNavigate}
                aria-label={label}
                className={cn(linkClasses, 'h-9 flex-none justify-center px-0 aria-[current=page]:bg-sidebar-accent')}
              >
                <Icon className="size-4 shrink-0" aria-hidden />
              </NavLink>
            </TooltipTrigger>
            <TooltipContent side="right">{label}</TooltipContent>
          </Tooltip>
        )
      })}
    </nav>
  )
}

interface FavoriteRowProps {
  favorite: Favorite
  index: number
  total: number
  onMove: (from: number, to: number) => void
  onRemove: (favorite: Favorite) => void
  onNavigate?: () => void
}

function FavoriteRow({ favorite, index, total, onMove, onRemove, onNavigate }: FavoriteRowProps) {
  const { t } = useTranslation('favorites')
  const labelOf = useFavoriteLabel()
  const label = labelOf(favorite)
  const href = favoriteHref(favorite)
  const { attributes, listeners, setNodeRef, setActivatorNodeRef, transform, transition, isDragging } = useSortable({
    id: favorite.id,
  })

  const content: ReactNode = (
    <>
      {createElement(favoriteIcon(favorite), { className: 'size-4 shrink-0', 'aria-hidden': true })}
      <span className="truncate">{label}</span>
    </>
  )

  return (
    <li
      ref={setNodeRef}
      style={{ transform: CSS.Translate.toString(transform), transition }}
      className={cn('group/favorite relative flex items-center gap-0.5', isDragging && 'z-10 rounded-md bg-sidebar opacity-90 shadow-lg')}
    >
      <button
        type="button"
        ref={setActivatorNodeRef}
        className="flex h-8 w-4 shrink-0 cursor-grab touch-none items-center justify-center rounded text-muted-foreground opacity-0 transition-opacity outline-none group-focus-within/favorite:opacity-100 group-hover/favorite:opacity-100 focus-visible:opacity-100 focus-visible:ring-2 focus-visible:ring-ring active:cursor-grabbing max-md:opacity-100"
        {...attributes}
        {...listeners}
        aria-label={t('reorder.handle', { label })}
      >
        <GripVertical className="size-3.5" aria-hidden />
      </button>
      {href ? (
        <NavLink
          to={href}
          end
          onClick={onNavigate}
          className={({ isActive }) =>
            cn(
              linkClasses,
              isActive &&
                favorite.favorite_type === 'record' &&
                'bg-sidebar-accent font-medium text-sidebar-accent-foreground',
            )
          }
        >
          {content}
        </NavLink>
      ) : (
        <span className={cn(linkClasses, 'text-muted-foreground italic hover:bg-transparent')} title={t('sidebar.unavailableHint')}>
          {content}
        </span>
      )}
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <button
            type="button"
            className="flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground opacity-0 transition-opacity outline-none group-focus-within/favorite:opacity-100 group-hover/favorite:opacity-100 hover:bg-sidebar-accent focus-visible:opacity-100 focus-visible:ring-2 focus-visible:ring-ring data-[state=open]:opacity-100 max-md:opacity-100"
            aria-label={t('sidebar.actions', { label })}
          >
            <MoreHorizontal className="size-4" aria-hidden />
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" side="right" className="min-w-48">
          <DropdownMenuItem disabled={index === 0} onSelect={() => onMove(index, index - 1)}>
            <ArrowUp aria-hidden />
            {t('sidebar.moveUp')}
          </DropdownMenuItem>
          <DropdownMenuItem disabled={index === total - 1} onSelect={() => onMove(index, index + 1)}>
            <ArrowDown aria-hidden />
            {t('sidebar.moveDown')}
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem onSelect={() => onRemove(favorite)}>
            <StarOff aria-hidden />
            {t('sidebar.remove')}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </li>
  )
}

export function SidebarFavorites({ collapsed = false, onNavigate }: SidebarFavoritesProps) {
  const { t } = useTranslation('favorites')
  const favorites = useFavorites()
  const { remove, reorder } = useFavoriteMutations()
  const [open, setOpen] = useState(() => readStorage(SECTION_OPEN_KEY) !== 'false')
  const labelOf = useFavoriteLabel()
  const listId = useId()
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 4 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  )

  const items = favorites.data ?? []
  if (collapsed) return <CollapsedFavorites favorites={items} onNavigate={onNavigate} />
  if (!favorites.isSuccess) return null

  const ids = items.map((favorite) => favorite.id)
  const labelById = (id: UniqueIdentifier) => {
    const favorite = items.find((item) => item.id === String(id))
    return favorite ? labelOf(favorite) : String(id)
  }
  const positionOf = (id: UniqueIdentifier) => ids.indexOf(String(id)) + 1
  const announcements: Announcements = {
    onDragStart: ({ active }) => t('reorder.picked', { label: labelById(active.id) }),
    onDragOver: ({ active, over }) =>
      over ? t('reorder.over', { label: labelById(active.id), position: positionOf(over.id), total: ids.length }) : undefined,
    onDragEnd: ({ active, over }) =>
      over
        ? t('reorder.dropped', { label: labelById(active.id), position: positionOf(over.id), total: ids.length })
        : undefined,
    onDragCancel: ({ active }) => t('reorder.cancelled', { label: labelById(active.id) }),
  }

  const move = (from: number, to: number) => {
    if (from === to || to < 0 || to >= ids.length) return
    reorder.mutate(arrayMove(ids, from, to))
  }

  const handleDragEnd = ({ active, over }: DragEndEvent) => {
    if (!over || active.id === over.id) return
    move(ids.indexOf(String(active.id)), ids.indexOf(String(over.id)))
  }

  const toggleOpen = () => {
    setOpen((current) => {
      writeStorage(SECTION_OPEN_KEY, String(!current))
      return !current
    })
  }

  const ToggleIcon = open ? ChevronDown : ChevronRight

  return (
    <section aria-label={t('sidebar.label')} className="mt-3 border-t border-sidebar-border px-2 pt-3">
      <button
        type="button"
        onClick={toggleOpen}
        aria-expanded={open}
        aria-controls={listId}
        className="flex h-7 w-full items-center gap-1 rounded-md px-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
      >
        <ToggleIcon className="size-3.5" aria-hidden />
        <span>{t('sidebar.title')}</span>
        {items.length > 0 && <span className="ml-auto font-normal tabular-nums">{items.length}</span>}
      </button>
      {open && (
        <div id={listId} className="mt-1">
          {items.length === 0 ? (
            <p className="px-2 py-1 text-xs text-muted-foreground">{t('sidebar.empty')}</p>
          ) : (
            <DndContext
              sensors={sensors}
              collisionDetection={closestCenter}
              onDragEnd={handleDragEnd}
              accessibility={{ announcements, screenReaderInstructions: { draggable: t('reorder.instructions') } }}
            >
              <SortableContext items={ids} strategy={verticalListSortingStrategy}>
                <ul className="flex flex-col gap-0.5">
                  {items.map((favorite, index) => (
                    <FavoriteRow
                      key={favorite.id}
                      favorite={favorite}
                      index={index}
                      total={items.length}
                      onMove={move}
                      onRemove={(item) => remove.mutate(item)}
                      onNavigate={onNavigate}
                    />
                  ))}
                </ul>
              </SortableContext>
            </DndContext>
          )}
        </div>
      )}
    </section>
  )
}
