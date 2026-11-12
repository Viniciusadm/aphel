import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { Plus } from 'lucide-react'
import { useState } from 'react'

import { useTranslation } from 'react-i18next'
import { BLOCK_ICONS, blockIcon } from './blockIcons'
import { BLOCK_TYPES, CONTAINER_TYPES, RECORD_TYPES, SITE_TYPES, newPluginBlock, type Block, type BlockType } from './blocks'
import { localized, useCollections, useEmbeds, usePluginBlocks } from './api'
import { useBlockScope, type BlockScope } from './scope'

interface AddBlockProps {
  onAdd: (type: BlockType) => void
  disabled?: boolean
  compact?: boolean
  allowContainers?: boolean
  scope?: BlockScope
  onAddBlock?: (block: Block) => void
}

export function AddBlock({ onAdd, disabled, compact, allowContainers = true, scope: requested, onAddBlock }: AddBlockProps) {
  const contextual = useBlockScope()
  const scope = requested ?? contextual
  const { i18n } = useTranslation()
  const plugins = usePluginBlocks()
  const templating = scope === 'template'
  const t = useModuleTranslation('site-blocks')
  const [open, setOpen] = useState(false)
  const [search, setSearch] = useState('')
  const embeds = useEmbeds()
  const collections = useCollections()
  const available = (type: BlockType) => {
    if (type === 'form' || type === 'booking') return embeds.data?.some((source) => source.kind === type) ?? false
    if (type === 'collection') return (collections.data?.length ?? 0) > 0
    return true
  }
  const needle = normalize(search)
  const matches = (label: string, help: string) => !needle || normalize(`${label} ${help}`).includes(needle)
  const shown = (type: BlockType) => available(type) && matches(t(`types.${type}`), t(`types.${type}Help`))
  const pluginBlocks = templating && onAddBlock ? (plugins.data ?? []).filter((definition) => matches(localized(definition.label, i18n.language) || definition.type, localized(definition.description, i18n.language))) : []
  const sections = [
    { title: t('editor.chooseType'), types: BLOCK_TYPES.filter((type) => ![...CONTAINER_TYPES, ...SITE_TYPES, ...RECORD_TYPES].includes(type as never)) },
    { title: t('editor.chooseLayout'), types: allowContainers ? (CONTAINER_TYPES as BlockType[]) : [] },
    { title: t('editor.chooseSite'), types: templating ? (SITE_TYPES as BlockType[]) : [] },
    { title: t('editor.chooseRecord'), types: templating ? (RECORD_TYPES as BlockType[]) : [] },
  ].filter((section) => section.types.some(shown))
  const choose = (type: BlockType) => {
    setOpen(false)
    setSearch('')
    onAdd(type)
  }
  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) setSearch('')
      }}
    >
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
        <Input
          className="blocks-types-search"
          type="search"
          autoFocus
          value={search}
          placeholder={t('editor.searchBlocks')}
          aria-label={t('editor.searchBlocks')}
          onChange={(event) => setSearch(event.target.value)}
          onKeyDown={(event) => {
            if (event.key !== 'Enter') return
            event.preventDefault()
            const first = sections.flatMap((section) => section.types.filter(shown))[0]
            if (first) choose(first)
          }}
        />
        {sections.length === 0 && pluginBlocks.length === 0 && <p className="blocks-types-empty">{t('editor.noBlocks')}</p>}
        {sections
          .map((section) => (
            <section key={section.title}>
              <p className="blocks-types-title">{section.title}</p>
              <ul>
                {section.types.filter(shown).map((type) => {
                  const Icon = BLOCK_ICONS[type] ?? blockIcon(type)
                  return (
                    <li key={type}>
                      <button
                        type="button"
                        className="blocks-type"
                        onClick={() => choose(type)}
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
            </section>
          ))}
        {onAddBlock && pluginBlocks.length > 0 && (
          <section>
            <p className="blocks-types-title">{t('editor.choosePlugin')}</p>
            <ul>
              {pluginBlocks.map((definition) => {
                const Icon = blockIcon(definition.type)
                return (
                  <li key={definition.type}>
                    <button
                      type="button"
                      className="blocks-type"
                      onClick={() => {
                        setOpen(false)
                        setSearch('')
                        onAddBlock(newPluginBlock(definition.type, Object.fromEntries(definition.fields.filter((field) => field.default !== undefined).map((field) => [field.key, field.default]))))
                      }}
                    >
                      <Icon aria-hidden size={20} />
                      <span>
                        <strong>{localized(definition.label, i18n.language) || definition.type}</strong>
                        <small>{localized(definition.description, i18n.language)}</small>
                      </span>
                    </button>
                  </li>
                )
              })}
            </ul>
          </section>
        )}
      </PopoverContent>
    </Popover>
  )
}

function normalize(text: string) {
  return text
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .toLowerCase()
    .trim()
}
