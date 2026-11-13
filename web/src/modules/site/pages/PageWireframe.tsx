import type { PageTemplate } from './api'

export const TEMPLATE_BLOCKS: Record<PageTemplate, string[]> = {
  blank: ['hero', 'text'],
  about: ['hero', 'text_image', 'columns', 'testimonials'],
  services: ['hero', 'columns', 'faq', 'contact'],
  campaign: ['hero', 'columns', 'text', 'contact'],
}

export function blockTypes(raw: unknown): string[] {
  if (typeof raw !== 'string' || !raw) return []
  try {
    const parsed: unknown = JSON.parse(raw)
    return Array.isArray(parsed) ? parsed.map((block) => (block && typeof block === 'object' && 'type' in block ? String(block.type) : 'text')) : []
  } catch {
    return []
  }
}

function Stripe({ type }: { type: string }) {
  if (type === 'hero') return <span className="pages-wf-hero"><i /><i /></span>
  if (type === 'columns' || type === 'testimonials' || type === 'gallery' || type === 'collection') return <span className="pages-wf-cols"><i /><i /><i /></span>
  if (type === 'text_image') return <span className="pages-wf-split"><i /><span><i /><i /><i /></span></span>
  if (type === 'faq') return <span className="pages-wf-faq"><i /><i /><i /></span>
  if (type === 'contact' || type === 'form' || type === 'booking') return <span className="pages-wf-box"><i /><i /></span>
  if (type === 'image' || type === 'video') return <span className="pages-wf-media" />
  return <span className="pages-wf-text"><i /><i /><i /></span>
}

export function PageWireframe({ types }: { types: string[] }) {
  const shown = types.slice(0, 4)
  return (
    <span className="pages-wf" aria-hidden="true">
      {shown.length === 0 ? <span className="pages-wf-text"><i /><i /></span> : shown.map((type, index) => <Stripe key={index} type={type} />)}
    </span>
  )
}
