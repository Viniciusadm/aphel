import { checkBlock, embedOf, isSafeLink, isValidEmail, isValidPhone, videoOf } from './validation'

export { embedOf, isSafeLink, isValidEmail, isValidPhone, videoOf, type EmbedProvider } from './validation'

export type BlockType = 'hero' | 'text' | 'image' | 'text_image' | 'buttons' | 'spacer' | 'gallery' | 'columns' | 'faq' | 'testimonials' | 'contact' | 'video' | 'form' | 'booking' | 'collection' | 'embed'
export type Alignment = 'center' | 'left'

export interface HeroBlock {
  id: string
  type: 'hero'
  title: string
  subtitle: string
  image: string | null
  button_label: string
  button_link: string
  align: Alignment
}

export interface TextBlock {
  id: string
  type: 'text'
  text: string
  width: 'narrow' | 'wide'
}

export interface ImageBlock {
  id: string
  type: 'image'
  image: string | null
  alt: string
  caption: string
  width: 'wide' | 'narrow' | 'full'
}

export interface TextImageBlock {
  id: string
  type: 'text_image'
  title: string
  text: string
  image: string | null
  alt: string
  image_side: 'left' | 'right'
}

export interface BlockButton {
  label: string
  link: string
  style: 'primary' | 'secondary'
}

export interface ButtonsBlock {
  id: string
  type: 'buttons'
  buttons: BlockButton[]
  align: Alignment
}

export interface SpacerBlock {
  id: string
  type: 'spacer'
  size: 'medium' | 'small' | 'large'
  divider: boolean
}

export interface GalleryItem {
  image: string | null
  alt: string
  caption: string
}

export interface GalleryBlock {
  id: string
  type: 'gallery'
  title: string
  items: GalleryItem[]
  layout: 'grid' | 'carousel'
  columns: '2' | '3' | '4'
}

export const ICONS = ['none', 'check', 'star', 'heart', 'clock', 'phone', 'map-pin', 'truck', 'leaf', 'shield', 'sparkles'] as const
export type IconName = (typeof ICONS)[number]

export interface ColumnItem {
  icon: IconName
  image: string | null
  alt: string
  title: string
  text: string
  link_label: string
  link: string
}

export interface ColumnsBlock {
  id: string
  type: 'columns'
  title: string
  items: ColumnItem[]
  align: Alignment
}

export interface FaqItem {
  question: string
  answer: string
}

export interface FaqBlock {
  id: string
  type: 'faq'
  title: string
  items: FaqItem[]
}

export interface TestimonialItem {
  quote: string
  name: string
  role: string
  image: string | null
}

export interface TestimonialsBlock {
  id: string
  type: 'testimonials'
  title: string
  items: TestimonialItem[]
}

export interface ContactBlock {
  id: string
  type: 'contact'
  title: string
  text: string
  phone: string
  whatsapp: string
  email: string
  address: string
  hours: string
  show_map: boolean
}

export interface VideoBlock {
  id: string
  type: 'video'
  url: string
  title: string
  caption: string
}

export interface FormBlock {
  id: string
  type: 'form'
  title: string
  text: string
  form: string
}

export interface BookingBlock {
  id: string
  type: 'booking'
  title: string
  text: string
  booking: string
}

export interface CollectionBlock {
  id: string
  type: 'collection'
  title: string
  owner: string
  entity: string
  sort: 'newest' | 'oldest'
  limit: '3' | '6' | '9' | '12'
  more_label: string
  more_link: string
}

export interface EmbedBlock {
  id: string
  type: 'embed'
  url: string
  title: string
  height: 'small' | 'medium' | 'large'
}

export type Block =
  | HeroBlock
  | TextBlock
  | ImageBlock
  | TextImageBlock
  | ButtonsBlock
  | SpacerBlock
  | GalleryBlock
  | ColumnsBlock
  | FaqBlock
  | TestimonialsBlock
  | ContactBlock
  | VideoBlock
  | FormBlock
  | BookingBlock
  | CollectionBlock
  | EmbedBlock

export const BLOCK_TYPES: BlockType[] = ['hero', 'text', 'image', 'text_image', 'gallery', 'columns', 'buttons', 'faq', 'testimonials', 'contact', 'form', 'booking', 'collection', 'embed', 'video', 'spacer']
export const ITEM_LIMITS = { gallery: 24, columns: 4, faq: 30, testimonials: 12 } as const
export const MAX_BLOCKS = 300
export const MAX_BUTTONS = 3
export const IMAGE_TYPES = ['image/jpeg', 'image/png', 'image/webp', 'image/gif']

function newId(): string {
  return crypto.randomUUID().slice(0, 13)
}

export function newBlock(type: BlockType): Block {
  const id = newId()
  switch (type) {
    case 'hero':
      return { id, type, title: '', subtitle: '', image: null, button_label: '', button_link: '', align: 'center' }
    case 'text':
      return { id, type, text: '', width: 'narrow' }
    case 'image':
      return { id, type, image: null, alt: '', caption: '', width: 'wide' }
    case 'text_image':
      return { id, type, title: '', text: '', image: null, alt: '', image_side: 'left' }
    case 'buttons':
      return { id, type, buttons: [{ label: '', link: '', style: 'primary' }], align: 'center' }
    case 'spacer':
      return { id, type, size: 'medium', divider: false }
    case 'gallery':
      return { id, type, title: '', items: [], layout: 'grid', columns: '3' }
    case 'columns':
      return { id, type, title: '', items: [newColumn(), newColumn(), newColumn()], align: 'center' }
    case 'faq':
      return { id, type, title: '', items: [newFaqItem()] }
    case 'testimonials':
      return { id, type, title: '', items: [newTestimonial()] }
    case 'contact':
      return { id, type, title: '', text: '', phone: '', whatsapp: '', email: '', address: '', hours: '', show_map: false }
    case 'video':
      return { id, type, url: '', title: '', caption: '' }
    case 'form':
      return { id, type, title: '', text: '', form: '' }
    case 'booking':
      return { id, type, title: '', text: '', booking: '' }
    case 'collection':
      return { id, type, title: '', owner: '', entity: '', sort: 'newest', limit: '6', more_label: '', more_link: '' }
    case 'embed':
      return { id, type, url: '', title: '', height: 'medium' }
  }
}

export function newGalleryItem(image: string | null = null): GalleryItem {
  return { image, alt: '', caption: '' }
}

export function newColumn(): ColumnItem {
  return { icon: 'check', image: null, alt: '', title: '', text: '', link_label: '', link: '' }
}

export function newFaqItem(): FaqItem {
  return { question: '', answer: '' }
}

export function newTestimonial(): TestimonialItem {
  return { quote: '', name: '', role: '', image: null }
}

export function duplicateBlock(block: Block): Block {
  return { ...structuredClone(block), id: newId() }
}

function isBlock(value: unknown): value is Block {
  return typeof value === 'object' && value !== null && BLOCK_TYPES.includes((value as { type?: BlockType }).type as BlockType)
}

export function parseBlocks(text: string | null | undefined): Block[] {
  if (!text) return []
  try {
    const parsed: unknown = JSON.parse(text)
    if (!Array.isArray(parsed)) return []
    return parsed.filter(isBlock).map((block) => ({ ...newBlock(block.type), ...block }) as Block)
  } catch {
    return []
  }
}

export function serializeBlocks(blocks: Block[]): string {
  return JSON.stringify(blocks)
}

function plain(text: string): string {
  return text
    .replace(/[#*_`>[\]()!-]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
}

export function blockSummary(block: Block): string {
  switch (block.type) {
    case 'hero':
      return block.title || block.subtitle
    case 'text':
      return plain(block.text)
    case 'image':
      return block.caption || block.alt
    case 'text_image':
      return block.title || plain(block.text)
    case 'buttons':
      return block.buttons.map((button) => button.label).filter(Boolean).join(' · ')
    case 'spacer':
      return ''
    case 'gallery':
      return block.title || (block.items.length ? String(block.items.length) : '')
    case 'columns':
    case 'faq':
    case 'testimonials':
      return block.title || itemsSummary(block)
    case 'contact':
      return block.title || block.phone || block.email
    case 'video':
      return block.title || block.caption
    case 'form':
    case 'booking':
    case 'collection':
      return block.title
    case 'embed':
      return block.title || (embedOf(block.url)?.provider ?? '')
  }
}

function itemsSummary(block: ColumnsBlock | FaqBlock | TestimonialsBlock): string {
  if (block.type === 'columns') return block.items.map((item) => item.title).filter(Boolean).join(' · ')
  if (block.type === 'faq') return block.items.map((item) => item.question).filter(Boolean).join(' · ')
  return block.items.map((item) => item.name).filter(Boolean).join(' · ')
}

export type BlockProblem = 'link' | 'video' | 'email' | 'phone' | 'embed' | 'block'

export function blockProblems(block: Block): BlockProblem[] {
  const problems: BlockProblem[] = []
  const links =
    block.type === 'hero'
      ? [block.button_link]
      : block.type === 'buttons'
        ? block.buttons.map((button) => button.link)
        : block.type === 'columns'
          ? block.items.map((item) => item.link)
          : block.type === 'collection'
            ? [block.more_link]
            : []
  if (links.some((link) => !isSafeLink(link))) problems.push('link')
  if (block.type === 'video' && block.url.trim() && !videoOf(block.url)) problems.push('video')
  if (block.type === 'embed' && block.url.trim() && !embedOf(block.url)) problems.push('embed')
  if (block.type === 'contact') {
    if (!isValidEmail(block.email)) problems.push('email')
    if (!isValidPhone(block.phone) || !isValidPhone(block.whatsapp)) problems.push('phone')
  }
  if (problems.length === 0 && checkBlock(block)) problems.push('block')
  return problems
}
