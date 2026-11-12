import { checkBlock, embedOf, isSafeLink, isValidEmail, isValidPhone, videoOf } from './validation'

export { embedOf, isSafeLink, isValidEmail, isValidPhone, videoOf, type EmbedProvider } from './validation'

export type ContainerType = 'group' | 'row' | 'stack' | 'grid' | 'cover' | 'details' | 'query'
export type SiteBlockType = 'site_logo' | 'site_title' | 'site_tagline' | 'navigation' | 'search' | 'languages' | 'account' | 'site_cta' | 'breadcrumbs' | 'social_links' | 'site_contact' | 'site_footer_text' | 'copyright' | 'template_part' | 'pattern'
export type RecordBlockType = 'record_title' | 'record_field' | 'record_image' | 'record_terms' | 'record_body' | 'record_link' | 'archive_title' | 'archive_description' | 'query_pagination'
export type PluginBlockType = `${string}:${string}`
export type BlockType = 'hero' | 'text' | 'image' | 'text_image' | 'buttons' | 'spacer' | 'quote' | 'code' | 'callout' | 'gallery' | 'columns' | 'faq' | 'testimonials' | 'contact' | 'video' | 'form' | 'booking' | 'collection' | 'embed' | ContainerType | SiteBlockType | RecordBlockType
export type Alignment = 'center' | 'left'

export type SpaceStep = '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8'
export type StyleRadius = 'none' | 'sm' | 'md' | 'lg' | 'full'
export type StyleDevice = 'desktop' | 'tablet' | 'mobile'

export interface BlockStyle {
  text?: string
  background?: string
  gradient?: string
  link?: string
  padding?: Partial<Record<'top' | 'right' | 'bottom' | 'left', SpaceStep>>
  margin?: Partial<Record<'top' | 'bottom', SpaceStep>>
  gap?: SpaceStep
  border?: { width?: '0' | '1' | '2' | '4' | '8'; style?: 'solid' | 'dashed' | 'dotted'; color?: string; radius?: StyleRadius }
  radius?: StyleRadius
  shadow?: 'none' | 'sm' | 'md' | 'lg'
  font_family?: string
  font_size?: string
  font_weight?: '300' | '400' | '500' | '600' | '700' | '800' | '900'
  line_height?: 'tight' | 'snug' | 'normal' | 'relaxed' | 'loose'
  letter_spacing?: 'tighter' | 'tight' | 'normal' | 'wide' | 'wider'
  text_transform?: 'none' | 'uppercase' | 'lowercase' | 'capitalize'
  text_align?: 'left' | 'center' | 'right'
  width?: 'none' | 'wide' | 'full'
  hide_on?: StyleDevice[]
  classes?: string
  variation?: string
}

export interface GroupBlock {
  id: string
  type: 'group'
  inner_width: 'full' | 'content' | 'wide'
  children: Block[]
}

export interface RowBlock {
  id: string
  type: 'row'
  justify: 'start' | 'center' | 'end' | 'between'
  vertical: 'center' | 'top' | 'bottom' | 'stretch'
  wrap: boolean
  children: Block[]
}

export interface StackBlock {
  id: string
  type: 'stack'
  align_items: 'stretch' | 'start' | 'center' | 'end'
  children: Block[]
}

export type GridLayout = '1-1' | '1-1-1' | '1-1-1-1' | '2-1' | '1-2' | '1-2-1' | '3-2' | '2-3'

export interface GridBlock {
  id: string
  type: 'grid'
  layout: GridLayout
  vertical: 'center' | 'top' | 'bottom' | 'stretch'
  stack_on: 'mobile' | 'tablet' | 'never'
  children: Block[]
}

export interface CoverBlock {
  id: string
  type: 'cover'
  image: string | null
  focal_x: number
  focal_y: number
  overlay: '0' | '20' | '40' | '60' | '80'
  min_height: 'small' | 'medium' | 'large' | 'screen'
  position: 'center' | 'bottom-left' | 'bottom-center' | 'top-left' | 'center-left'
  dark_text: boolean
  children: Block[]
}

export interface DetailsBlock {
  id: string
  type: 'details'
  summary: string
  open: boolean
  children: Block[]
}

export interface QueryBlock {
  id: string
  type: 'query'
  source: 'inherit' | 'custom'
  owner: string
  entity: string
  sort: 'newest' | 'oldest'
  order_by: string
  limit: '6' | '3' | '4' | '9' | '12' | '24'
  layout: 'grid' | 'list' | 'featured' | 'masonry' | 'carousel'
  columns: '2' | '1' | '3' | '4'
  scope_field: string
  scope: string
  match_field: string
  exclude_current: boolean
  title: string
  empty_text: string
  children: Block[]
}

export type ContainerBlock = GroupBlock | RowBlock | StackBlock | GridBlock | CoverBlock | DetailsBlock | QueryBlock

export interface SiteLogoBlock { id: string; type: 'site_logo'; show: 'logo_and_title' | 'logo' | 'title'; tagline: boolean }
export interface NavigationBlock { id: string; type: 'navigation'; menu: string; layout: 'horizontal' | 'vertical'; mobile: 'panel' | 'list' }
export interface SearchBlock { id: string; type: 'search'; display: 'icon' | 'field' }
export interface TemplatePartBlock { id: string; type: 'template_part'; part: string }
export interface PatternBlock { id: string; type: 'pattern'; pattern: string }
export interface SimpleSiteBlock { id: string; type: 'site_title' | 'site_tagline' | 'languages' | 'account' | 'site_cta' | 'breadcrumbs' | 'social_links' | 'site_contact' | 'site_footer_text' | 'copyright' | 'archive_description' }
export interface RecordTitleBlock { id: string; type: 'record_title'; level: 'h1' | 'h2' | 'h3' | 'p'; link: boolean }
export interface RecordFieldBlock { id: string; type: 'record_field'; field: string; format: 'text' | 'date' | 'datetime' | 'markdown' | 'number' | 'reading_time'; label: string }
export interface RecordImageBlock { id: string; type: 'record_image'; field: string; ratio: 'original' | 'landscape' | 'square' | 'portrait' | 'wide'; link: boolean; caption: boolean }
export interface RecordTermsBlock { id: string; type: 'record_terms'; vocabulary: 'all' | 'categories' | 'tags'; display: 'chips' | 'plain' | 'kicker' }
export interface RecordBodyBlock { id: string; type: 'record_body'; field: string }
export interface RecordLinkBlock { id: string; type: 'record_link'; label: string }
export interface ArchiveTitleBlock { id: string; type: 'archive_title'; level: 'h1' | 'h2' }
export interface QueryPaginationBlock { id: string; type: 'query_pagination'; display: 'arrows' | 'numbers' }
export interface PluginBlock { id: string; type: PluginBlockType; [setting: string]: unknown }

type SiteContentBlock = SiteLogoBlock | NavigationBlock | SearchBlock | TemplatePartBlock | PatternBlock | SimpleSiteBlock | RecordTitleBlock | RecordFieldBlock | RecordImageBlock | RecordTermsBlock | RecordBodyBlock | RecordLinkBlock | ArchiveTitleBlock | QueryPaginationBlock

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

export interface QuoteBlock {
  id: string
  type: 'quote'
  text: string
  source: string
}

export interface CodeBlock {
  id: string
  type: 'code'
  code: string
  language: string
}

export interface CalloutBlock {
  id: string
  type: 'callout'
  tone: 'info' | 'success' | 'warning'
  title: string
  text: string
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
  scope_field: string
  scope: string
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

type ContentBlock =
  | HeroBlock
  | TextBlock
  | ImageBlock
  | TextImageBlock
  | ButtonsBlock
  | SpacerBlock
  | QuoteBlock
  | CodeBlock
  | CalloutBlock
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
  | ContainerBlock
  | SiteContentBlock

export type Block = (ContentBlock & { style?: BlockStyle }) | (PluginBlock & { style?: BlockStyle })

export const CONTAINER_TYPES: ContainerType[] = ['group', 'row', 'stack', 'grid', 'cover', 'details']
export const SITE_TYPES: SiteBlockType[] = ['site_logo', 'site_title', 'site_tagline', 'navigation', 'search', 'languages', 'account', 'site_cta', 'breadcrumbs', 'social_links', 'site_contact', 'site_footer_text', 'copyright', 'template_part', 'pattern']
export const RECORD_TYPES: (RecordBlockType | 'query')[] = ['record_title', 'record_field', 'record_image', 'record_terms', 'record_body', 'record_link', 'archive_title', 'archive_description', 'query', 'query_pagination']
export const BLOCK_TYPES: BlockType[] = ['hero', 'text', 'image', 'text_image', 'gallery', 'columns', 'buttons', 'quote', 'callout', 'code', 'faq', 'testimonials', 'contact', 'form', 'booking', 'collection', 'embed', 'video', 'spacer', ...CONTAINER_TYPES, ...SITE_TYPES, ...RECORD_TYPES]
export const ITEM_LIMITS = { gallery: 24, columns: 4, faq: 30, testimonials: 12 } as const
export const MAX_BLOCKS = 600
export const MAX_DEPTH = 6
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
    case 'quote':
      return { id, type, text: '', source: '' }
    case 'code':
      return { id, type, code: '', language: '' }
    case 'callout':
      return { id, type, tone: 'info', title: '', text: '' }
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
      return { id, type, title: '', owner: '', entity: '', scope_field: '', scope: '', sort: 'newest', limit: '6', more_label: '', more_link: '' }
    case 'embed':
      return { id, type, url: '', title: '', height: 'medium' }
    case 'group':
      return { id, type, inner_width: 'full', children: [] }
    case 'row':
      return { id, type, justify: 'start', vertical: 'center', wrap: false, children: [] }
    case 'stack':
      return { id, type, align_items: 'stretch', children: [] }
    case 'grid':
      return { id, type, layout: '1-1', vertical: 'stretch', stack_on: 'mobile', children: [] }
    case 'cover':
      return { id, type, image: null, focal_x: 50, focal_y: 50, overlay: '40', min_height: 'medium', position: 'center', dark_text: false, children: [] }
    case 'details':
      return { id, type, summary: '', open: false, children: [] }
    case 'query':
      return { id, type, source: 'inherit', owner: '', entity: '', sort: 'newest', order_by: '', limit: '12', layout: 'grid', columns: '2', scope_field: '', scope: '', match_field: '', exclude_current: false, title: '', empty_text: '', children: [{ id: newId(), type: 'record_title', level: 'h3', link: true }, { id: newId(), type: 'record_field', field: 'summary', format: 'text', label: '' }] }
    case 'site_logo':
      return { id, type, show: 'logo_and_title', tagline: false }
    case 'navigation':
      return { id, type, menu: 'main', layout: 'horizontal', mobile: 'panel' }
    case 'search':
      return { id, type, display: 'icon' }
    case 'template_part':
      return { id, type, part: '' }
    case 'pattern':
      return { id, type, pattern: '' }
    case 'record_title':
      return { id, type, level: 'h2', link: false }
    case 'record_field':
      return { id, type, field: '', format: 'text', label: '' }
    case 'record_image':
      return { id, type, field: 'cover', ratio: 'landscape', link: false, caption: false }
    case 'record_terms':
      return { id, type, vocabulary: 'all', display: 'chips' }
    case 'record_body':
      return { id, type, field: 'body' }
    case 'record_link':
      return { id, type, label: '' }
    case 'archive_title':
      return { id, type, level: 'h1' }
    case 'query_pagination':
      return { id, type, display: 'arrows' }
    case 'site_title':
    case 'site_tagline':
    case 'languages':
    case 'account':
    case 'site_cta':
    case 'breadcrumbs':
    case 'social_links':
    case 'site_contact':
    case 'site_footer_text':
    case 'copyright':
    case 'archive_description':
      return { id, type }
  }
}

export function newPluginBlock(type: PluginBlockType, defaults: Record<string, unknown> = {}): Block {
  return { ...defaults, id: newId(), type }
}

export function isPluginBlock(block: Block): block is PluginBlock & { style?: BlockStyle } {
  return block.type.includes(':')
}

export function isContainer(block: Block): block is ContainerBlock & { style?: BlockStyle } {
  return block.type === 'query' || (CONTAINER_TYPES as string[]).includes(block.type)
}

export function flattenBlocks(blocks: Block[]): Block[] {
  return blocks.flatMap((block) => [block, ...(isContainer(block) ? flattenBlocks(block.children) : [])])
}

export function countBlocks(blocks: Block[]): number {
  return blocks.reduce((total, block) => total + 1 + (isContainer(block) ? countBlocks(block.children) : 0), 0)
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
  const copy = { ...structuredClone(block), id: newId() }
  if (isContainer(copy)) copy.children = copy.children.map(duplicateBlock)
  return copy
}

function isBlock(value: unknown): value is Block {
  if (typeof value !== 'object' || value === null) return false
  const type = (value as { type?: string }).type ?? ''
  return BLOCK_TYPES.includes(type as BlockType) || /^[a-z0-9_]+:[a-z0-9_]+$/.test(type)
}

export function parseBlocks(text: string | null | undefined): Block[] {
  if (!text) return []
  try {
    const parsed: unknown = JSON.parse(text)
    if (!Array.isArray(parsed)) return []
    return hydrate(parsed)
  } catch {
    return []
  }
}

function hydrate(list: unknown[]): Block[] {
  return list.filter(isBlock).map((block) => {
    const source = block as Record<string, unknown> & { type: string }
    const merged: Block = source.type.includes(':') ? (source as unknown as Block) : ({ ...(newBlock(source.type as BlockType) as unknown as Record<string, unknown>), ...source } as unknown as Block)
    if (isContainer(merged)) merged.children = hydrate(Array.isArray(merged.children) ? merged.children : [])
    return merged
  })
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
    case 'quote':
      return plain(block.text)
    case 'code':
      return block.language || block.code.split('\n')[0] || ''
    case 'callout':
      return block.title || plain(block.text)
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
    case 'details':
      return block.summary || childrenSummary(block.children)
    case 'group':
    case 'row':
    case 'stack':
    case 'grid':
    case 'cover':
      return childrenSummary(block.children)
    case 'query':
      return block.title || (block.source === 'custom' ? block.entity : '')
    case 'navigation':
      return block.menu
    case 'template_part':
      return block.part
    case 'pattern':
      return block.pattern
    case 'record_field':
    case 'record_image':
    case 'record_body':
      return block.field
    case 'record_link':
      return block.label
    default:
      return ''
  }
}

function childrenSummary(children: Block[]): string {
  return children.map(blockSummary).filter(Boolean).slice(0, 3).join(' · ')
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
  if (isContainer(block)) {
    for (const child of block.children) for (const problem of blockProblems(child)) if (!problems.includes(problem)) problems.push(problem)
  }
  if (problems.length === 0 && checkBlock(block)) problems.push('block')
  return problems
}
