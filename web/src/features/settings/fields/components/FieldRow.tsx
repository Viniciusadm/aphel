import { ArrowDown, ArrowUp, MoreHorizontal, Pencil, Power, PowerOff } from 'lucide-react'
import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { cn } from '@/lib/utils'
import type { Field } from '../api'

export type FieldAction = 'edit' | 'moveUp' | 'moveDown' | 'deactivate' | 'activate'

interface FieldRowProps {
  field: Field
  handle: ReactNode
  canMoveUp: boolean
  canMoveDown: boolean
  onAction: (action: FieldAction, field: Field) => void
}

export function FieldRow({ field, handle, canMoveUp, canMoveDown, onAction }: FieldRowProps) {
  const { t } = useTranslation('fields')
  const inactive = !field.is_active

  return (
    <div
      data-inactive={inactive || undefined}
      className={cn(
        'flex items-center gap-2 rounded-lg border bg-card px-2 py-3 text-card-foreground sm:gap-3 sm:px-3',
        inactive && 'border-dashed bg-muted/40 text-muted-foreground',
      )}
    >
      {handle}
      <div className="grid min-w-0 flex-1 gap-1.5">
        <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
          <span className={cn('font-medium break-words', inactive && 'text-muted-foreground')}>{field.label}</span>
          <code className="text-xs break-all text-muted-foreground">{field.key}</code>
        </div>
        <div className="flex flex-wrap items-center gap-1">
          <Badge variant="outline">{t(`types.${field.field_type}`)}</Badge>
          {field.is_system && <Badge variant="secondary">{t('list.system')}</Badge>}
          {field.module_key && (
            <Badge variant="secondary">{t('list.managed', { module: field.module_key })}</Badge>
          )}
          {field.is_required && <Badge variant="outline">{t('list.required')}</Badge>}
          {field.is_unique && <Badge variant="outline">{t('list.unique')}</Badge>}
          {field.is_searchable && <Badge variant="outline">{t('list.searchable')}</Badge>}
          {field.is_sensitive ? (
            <Badge variant="warning">{t('list.sensitive')}</Badge>
          ) : (
            field.is_personal && <Badge variant="secondary">{t('list.personal')}</Badge>
          )}
          {inactive && <Badge variant="warning">{t('list.inactive')}</Badge>}
        </div>
      </div>
      <div className="flex shrink-0 items-center gap-1">
        <Button
          variant="ghost"
          size="icon"
          aria-label={`${t('actions.edit')}: ${field.label}`}
          disabled={Boolean(field.module_key)}
          onClick={() => onAction('edit', field)}
        >
          <Pencil />
        </Button>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="icon" aria-label={`${t('actions.more')}: ${field.label}`}>
              <MoreHorizontal />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem disabled={!canMoveUp} onSelect={() => onAction('moveUp', field)}>
              <ArrowUp />
              {t('actions.moveUp')}
            </DropdownMenuItem>
            <DropdownMenuItem disabled={!canMoveDown} onSelect={() => onAction('moveDown', field)}>
              <ArrowDown />
              {t('actions.moveDown')}
            </DropdownMenuItem>
            {!field.is_system && !field.module_key && (
              <>
                <DropdownMenuSeparator />
                {field.is_active ? (
                  <DropdownMenuItem variant="destructive" onSelect={() => onAction('deactivate', field)}>
                    <PowerOff />
                    {t('actions.deactivate')}
                  </DropdownMenuItem>
                ) : (
                  <DropdownMenuItem onSelect={() => onAction('activate', field)}>
                    <Power />
                    {t('actions.activate')}
                  </DropdownMenuItem>
                )}
              </>
            )}
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  )
}
