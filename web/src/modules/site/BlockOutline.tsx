import { createElement } from 'react'
import { useTranslation } from 'react-i18next'
import { ArrowDown, ArrowUp, Copy, Trash2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { AddBlock, blockIcon, blockSummary, isContainerBlock, newBlock, type Block } from '@/sdk/blocks'

interface OutlineProps {
  blocks: Block[]
  selected: string | null
  disabled: boolean
  onSelect: (id: string) => void
  onMove: (id: string, offset: number) => void
  onDuplicate: (id: string) => void
  onRemove: (id: string) => void
  onInsert: (parent: string | null, index: number, block: Block) => void
  pluginLabel: (type: string) => string | null
}

function Level({ blocks, parent, depth, props }: { blocks: Block[]; parent: string | null; depth: number; props: OutlineProps }) {
  const t = useModuleTranslation('site-blocks')
  const { t: site } = useTranslation('site')
  return (
    <ul className="site-outline-level" role={depth === 0 ? 'tree' : 'group'}>
      {blocks.map((block, index) => (
        <li key={block.id} role="treeitem" aria-selected={props.selected === block.id}>
          <div className="site-outline-row" data-selected={props.selected === block.id || undefined} style={{ paddingLeft: `${0.35 + depth * 0.9}rem` }}>
            <button type="button" className="site-outline-name" onClick={() => props.onSelect(block.id)}>
              {createElement(blockIcon(block.type), { 'aria-hidden': true, size: 15 })}
              <span>{props.pluginLabel(block.type) ?? t(`types.${block.type}`)}</span>
              <small>{blockSummary(block)}</small>
            </button>
            {props.selected === block.id && (
              <span className="site-outline-tools">
                <Button type="button" size="icon" variant="ghost" disabled={props.disabled || index === 0} aria-label={site('siteEditor.moveUp')} title={site('siteEditor.moveUp')} onClick={() => props.onMove(block.id, -1)}>
                  <ArrowUp aria-hidden />
                </Button>
                <Button type="button" size="icon" variant="ghost" disabled={props.disabled || index === blocks.length - 1} aria-label={site('siteEditor.moveDown')} title={site('siteEditor.moveDown')} onClick={() => props.onMove(block.id, 1)}>
                  <ArrowDown aria-hidden />
                </Button>
                <Button type="button" size="icon" variant="ghost" disabled={props.disabled} aria-label={site('siteEditor.duplicate')} title={site('siteEditor.duplicate')} onClick={() => props.onDuplicate(block.id)}>
                  <Copy aria-hidden />
                </Button>
                <Button type="button" size="icon" variant="ghost" disabled={props.disabled} aria-label={site('siteEditor.remove')} title={site('siteEditor.remove')} onClick={() => props.onRemove(block.id)}>
                  <Trash2 aria-hidden />
                </Button>
              </span>
            )}
          </div>
          {isContainerBlock(block) && <Level blocks={block.children} parent={block.id} depth={depth + 1} props={props} />}
        </li>
      ))}
      <li className="site-outline-add" style={{ paddingLeft: `${0.35 + depth * 0.9}rem` }}>
        <AddBlock compact scope="template" disabled={props.disabled} allowContainers={depth < 5} onAdd={(type) => props.onInsert(parent, blocks.length, newBlock(type))} onAddBlock={(block) => props.onInsert(parent, blocks.length, block)} />
      </li>
    </ul>
  )
}

export function BlockOutline(props: OutlineProps) {
  return <Level blocks={props.blocks} parent={null} depth={0} props={props} />
}
