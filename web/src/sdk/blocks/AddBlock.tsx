import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { Plus } from 'lucide-react'
import { useState } from 'react'

import { BLOCK_ICONS } from './blockIcons'
import { BLOCK_TYPES, type BlockType } from './blocks'
import { useCollections, useEmbeds } from './api'

interface AddBlockProps {
  onAdd: (type: BlockType) => void
  disabled?: boolean
  compact?: boolean
}

export function AddBlock({ onAdd, disabled, compact }: AddBlockProps) {
  const t = useModuleTranslation('site-blocks')
  const [open, setOpen] = useState(false)
  const embeds = useEmbeds()
  const collections = useCollections()
  const available = (type: BlockType) => {
    if (type === 'form' || type === 'booking') return embeds.data?.some((source) => source.kind === type) ?? false
    if (type === 'collection') return (collections.data?.length ?? 0) > 0
    return true
  }
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        {compact ? (
          <button type="button" className="blocks-insert" disabled={disabled} aria-label={t('editor.addHere')} title={t('editor.addHere')}>
            <Plus aria-hidden size={14} />
          </button>
        ) : (
          <Button type="button" variant="outline" className="blocks-add" disabled={disabled}>
            <Plus aria-hidden />
            {t('editor.add')}
          </Button>
        )}
      </PopoverTrigger>
      <PopoverContent className="blocks-types" align="start" collisionPadding={8}>
        <p className="blocks-types-title">{t('editor.chooseType')}</p>
        <ul>
          {BLOCK_TYPES.filter(available).map((type) => {
            const Icon = BLOCK_ICONS[type]
            return (
              <li key={type}>
                <button
                  type="button"
                  className="blocks-type"
                  onClick={() => {
                    setOpen(false)
                    onAdd(type)
                  }}
                >
                  <Icon aria-hidden size={20} />
                  <span>
                    <strong>{t(`types.${type}`)}</strong>
                    <small>{t(`types.${type}Help`)}</small>
                  </span>
                </button>
              </li>
            )
          })}
        </ul>
      </PopoverContent>
    </Popover>
  )
}
