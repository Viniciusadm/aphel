import type { PageTemplate } from './api'

export interface StarterTexts {
  aboutHero: string
  aboutStoryTitle: string
  aboutStory: string
  aboutValuesTitle: string
  values: [string, string][]
  aboutTestimonialsTitle: string
  servicesSubtitle: string
  servicesTitle: string
  services: [string, string][]
  faqTitle: string
  faq: [string, string][]
  contactTitle: string
  contactText: string
  campaignSubtitle: string
  campaignButton: string
  campaignBenefitsTitle: string
  benefits: [string, string][]
  campaignCall: string
}

const VALUE_ICONS = ['heart', 'star', 'leaf']
const SERVICE_ICONS = ['check', 'check', 'check']
const BENEFIT_ICONS = ['sparkles', 'clock', 'shield']

type Block = Record<string, unknown>

function hero(title: string, subtitle: string, button: [string, string] = ['', '']): Block {
  return { id: 'hero', type: 'hero', title, subtitle, image: null, button_label: button[0], button_link: button[1], align: 'center' }
}

function columns(id: string, title: string, icons: string[], items: [string, string][]): Block {
  return {
    id,
    type: 'columns',
    title,
    align: 'center',
    items: items.map(([name, text], index) => ({ icon: icons[index] ?? 'none', image: null, alt: '', title: name, text, link_label: '', link: '' })),
  }
}

function contact(texts: StarterTexts): Block {
  return { id: 'contact', type: 'contact', title: texts.contactTitle, text: texts.contactText, phone: '', whatsapp: '', email: '', address: '', hours: '', show_map: false }
}

export function starterBlocks(template: PageTemplate, title: string, texts: StarterTexts): Block[] {
  switch (template) {
    case 'about':
      return [
        hero(title, texts.aboutHero),
        { id: 'story', type: 'text_image', title: texts.aboutStoryTitle, text: texts.aboutStory, image: null, alt: '', image_side: 'right' },
        columns('values', texts.aboutValuesTitle, VALUE_ICONS, texts.values),
        { id: 'testimonials', type: 'testimonials', title: texts.aboutTestimonialsTitle, items: [] },
      ]
    case 'services':
      return [
        hero(title, texts.servicesSubtitle),
        columns('services', texts.servicesTitle, SERVICE_ICONS, texts.services),
        { id: 'faq', type: 'faq', title: texts.faqTitle, items: texts.faq.map(([question, answer]) => ({ question, answer })) },
        contact(texts),
      ]
    case 'campaign':
      return [
        hero(title, texts.campaignSubtitle, [texts.campaignButton, '#contact']),
        columns('benefits', texts.campaignBenefitsTitle, BENEFIT_ICONS, texts.benefits),
        { id: 'call', type: 'text', text: `**${texts.campaignCall}**`, width: 'narrow' },
        contact(texts),
      ]
    default:
      return [hero(title, ''), { id: 'text', type: 'text', text: '', width: 'narrow' }]
  }
}
