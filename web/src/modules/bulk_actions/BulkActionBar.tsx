import {
  Archive,
  ArchiveRestore,
  ChevronDown,
  Download,
  ListOrdered,
  MoveRight,
  PencilLine,
  Tag,
  Trash2,
  UserRound,
} from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import type { BulkActionContext } from '@/features/records/table'
import { useHasReferenceOptions } from '@/kernel/modules/reference-options'
import { lazyComponent } from '@/lib/lazy-component'
import { bulkPermission, isBulkRecordType, type BulkRecordType } from './api'
import type { BulkActionKind } from './BulkActionDialog'

const BulkActionDialog = lazyComponent(async () => (await import('./BulkActionDialog')).BulkActionDialog)

interface MenuEntry {
  kind: BulkActionKind
  icon: typeof Tag
  permission: 'update' | 'archive' | 'delete' | 'export'
  dealsOnly?: boolean
  contactsOnly?: boolean
  requires?: string
  reference?: string
}

const PRIMARY: MenuEntry[] = [
  { kind: 'assign_owner', icon: UserRound, permission: 'update' },
  { kind: 'tags', icon: Tag, permission: 'update', reference: 'tag' },
]

const MORE: MenuEntry[] = [
  { kind: 'set_field', icon: PencilLine, permission: 'update' },
  { kind: 'move_stage', icon: MoveRight, permission: 'update', dealsOnly: true, reference: 'pipeline_stage' },
  {
    kind: 'enroll_in_sequence',
    icon: ListOrdered,
    permission: 'update',
    contactsOnly: true,
    requires: 'sequences.enroll',
    reference: 'sequence',
  },
  { kind: 'export', icon: Download, permission: 'export' },
  { kind: 'archive', icon: Archive, permission: 'archive' },
  { kind: 'unarchive', icon: ArchiveRestore, permission: 'archive' },
  { kind: 'delete', icon: Trash2, permission: 'delete' },
]

export function BulkActionBar(context: BulkActionContext) {
  const { t } = useTranslation('bulk_actions')
  const { hasPermission } = useSession()
  const [active, setActive] = useState<BulkActionKind | null>(null)
  const references: Record<string, boolean> = {
    tag: useHasReferenceOptions('tag'),
    pipeline_stage: useHasReferenceOptions('pipeline_stage'),
    sequence: useHasReferenceOptions('sequence'),
  }
  const recordType = context.recordType
  if (!isBulkRecordType(recordType)) return null

  const allowed = (entry: MenuEntry, type: BulkRecordType) =>
    (entry.requires ? hasPermission(entry.requires) : hasPermission(bulkPermission(type, entry.permission))) &&
    (!entry.dealsOnly || type === 'deals.deal') &&
    (!entry.contactsOnly || type === 'contacts.contact') &&
    (!entry.reference || references[entry.reference])
  const primary = PRIMARY.filter((entry) => allowed(entry, recordType))
  const more = MORE.filter((entry) => allowed(entry, recordType))
  const destructive = more.filter((entry) => entry.kind === 'delete')
  const regular = more.filter((entry) => entry.kind !== 'delete')

  return (
    <>
      {primary.map((entry) => (
        <Button key={entry.kind} variant="outline" size="sm" onClick={() => setActive(entry.kind)}>
          <entry.icon aria-hidden="true" />
          {t(`actions.${entry.kind}`)}
        </Button>
      ))}
      {more.length > 0 && (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="outline" size="sm">
              {t('actions.more')}
              <ChevronDown aria-hidden="true" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start">
            {regular.map((entry) => (
              <DropdownMenuItem key={entry.kind} onSelect={() => setActive(entry.kind)}>
                <entry.icon aria-hidden="true" />
                {t(`actions.${entry.kind}`)}
              </DropdownMenuItem>
            ))}
            {destructive.length > 0 && regular.length > 0 && <DropdownMenuSeparator />}
            {destructive.map((entry) => (
              <DropdownMenuItem key={entry.kind} className="text-destructive" onSelect={() => setActive(entry.kind)}>
                <entry.icon aria-hidden="true" />
                {t(`actions.${entry.kind}`)}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      )}
      {active && (
        <BulkActionDialog
          kind={active}
          context={context}
          recordType={recordType}
          onClose={() => setActive(null)}
        />
      )}
    </>
  )
}
