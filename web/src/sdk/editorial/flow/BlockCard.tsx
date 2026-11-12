import { Node, NodeViewWrapper, ReactNodeViewRenderer, mergeAttributes, type NodeViewProps } from '@tiptap/react'
import { Pencil, Trash2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { fileDownloadUrl } from '@/features/files'
import { useModuleTranslation } from '@/kernel/modules/i18n'

import { BLOCK_ICONS } from '../../blocks/blockIcons'
import { blockSummary, type Block } from '../../blocks/blocks'

export interface BlockCardOptions {
  onEdit: (position: number, block: Block) => void
  editable: () => boolean
}

function parse(source: unknown): Block | null {
  try {
    const parsed = JSON.parse(String(source)) as Block
    return typeof parsed?.type === 'string' ? parsed : null
  } catch {
    return null
  }
}

function BlockCardView({ node, getPos, deleteNode, extension }: NodeViewProps) {
  const t = useModuleTranslation('site-blocks')
  const editorial = useModuleTranslation('editorial')
  const options = extension.options as BlockCardOptions
  const block = parse(node.attrs.block)
  if (!block) return <NodeViewWrapper className="flow-card" />
  const Icon = BLOCK_ICONS[block.type]
  const picture = block.type === 'image' && block.image ? fileDownloadUrl(block.image) : null
  const editable = options.editable()
  return (
    <NodeViewWrapper className="flow-card" data-drag-handle>
      {picture && <img className="flow-card-image" src={picture} alt={block.type === 'image' ? block.alt : ''} />}
      <div className="flow-card-row">
        <Icon aria-hidden size={18} />
        <span className="flow-card-name">{t(`types.${block.type}`)}</span>
        <span className="flow-card-summary">{blockSummary(block)}</span>
        {editable && (
          <span className="flow-card-tools">
            <Button type="button" variant="ghost" size="sm" onClick={() => options.onEdit(Number(getPos()), block)}>
              <Pencil aria-hidden />
              {editorial('flow.editBlock')}
            </Button>
            <Button type="button" variant="ghost" size="icon" aria-label={editorial('flow.removeBlock')} title={editorial('flow.removeBlock')} onClick={() => deleteNode()}>
              <Trash2 aria-hidden />
            </Button>
          </span>
        )}
      </div>
    </NodeViewWrapper>
  )
}

export const BlockCard = Node.create<BlockCardOptions>({
  name: 'blockCard',
  group: 'block',
  atom: true,
  draggable: true,
  selectable: true,
  addOptions() {
    return { onEdit: () => undefined, editable: () => true }
  },
  addAttributes() {
    return {
      block: {
        default: '{}',
        parseHTML: (element) => element.getAttribute('data-block-card') ?? '{}',
        renderHTML: (attributes) => ({ 'data-block-card': attributes.block as string }),
      },
    }
  },
  parseHTML() {
    return [{ tag: 'div[data-block-card]' }]
  },
  renderHTML({ HTMLAttributes }) {
    return ['div', mergeAttributes(HTMLAttributes)]
  },
  addNodeView() {
    return ReactNodeViewRenderer(BlockCardView)
  },
})
