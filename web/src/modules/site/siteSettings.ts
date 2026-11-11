import { type OrganizationConfiguration, type SiteLink, type SiteSettings, useSaveConfiguration } from '@/sdk'

export interface SiteHeaderSettings {
  align: 'left' | 'center'
  cta: { label: string; url: string } | null
}

export interface SiteSocialLink {
  network: string
  url: string
}

export interface SiteFooterSettings {
  text: string | null
  contact: { phone: string | null; whatsapp: string | null; email: string | null; address: string | null }
  social: SiteSocialLink[]
  links: SiteLink[]
  credit: boolean
}

export interface SiteTexts {
  tagline?: string | null
  footer_text?: string | null
  cta_label?: string | null
  links?: Record<string, string>
}

export interface SiteModuleSettings extends SiteSettings {
  link_people: boolean
  people_owner: string | null
  header?: SiteHeaderSettings
  footer?: SiteFooterSettings
  translations?: Record<string, SiteTexts>
}

const DEFAULT_SETTINGS: SiteModuleSettings = {
  indexing: true,
  accounts: false,
  theme: null,
  tagline: null,
  menu: [],
  languages: [],
  link_people: false,
  people_owner: null,
  links: [],
  home: null,
  scrollbar: 'default',
}

export function currentSettings(configuration: OrganizationConfiguration): SiteModuleSettings {
  return { ...DEFAULT_SETTINGS, ...(configuration.site as Partial<SiteModuleSettings> | undefined) }
}

export function useSaveSite(configuration: OrganizationConfiguration) {
  const save = useSaveConfiguration()
  return (site: Partial<SiteModuleSettings>, extra: Record<string, unknown> = {}) =>
    save({ ...extra, site: { ...currentSettings(configuration), ...site } })
}

export function languageName(code: string, locale: string): string {
  try {
    return new Intl.DisplayNames([locale], { type: 'language' }).of(code) ?? code
  } catch {
    return code
  }
}
