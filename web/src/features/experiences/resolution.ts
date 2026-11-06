import type { components } from '@/api'
import type { RecordTab, RecordSectionSource } from '@/features/records/detail/types'

type Schemas = components['schemas']
export type ExperienceProfile = Schemas['ExperienceProfile']
export type OrganizationExperiences = Schemas['OrganizationExperiences']
export type ExperiencePreferences = Schemas['ExperiencePreferences']
export interface ExperienceDiagnostic {
  code: 'navigationIgnored' | 'loadFailed' | 'profileIgnored' | 'formatUnsupported' | 'frameUnavailable' | 'presentationUnavailable' | 'sectionUnavailable' | 'sectionIgnored' | 'compositionUnsupported' | 'recordTypeUnavailable'
  identity: string
}

export function selectExperience(config: OrganizationExperiences | null | undefined, personal: ExperiencePreferences | undefined) {
  const diagnostics: ExperienceDiagnostic[] = []
  const allowed = (key: string | null | undefined) => config?.profiles.find((entry) => entry.enabled && entry.profile.key === key && config.policy.allowed_profiles.includes(entry.profile.key))?.profile
  const chosen = config?.policy.customization.profile_selection ? allowed(personal?.profile) : undefined
  if (personal?.profile && !chosen) diagnostics.push({ code: 'profileIgnored', identity: personal.profile })
  const profile = chosen ?? allowed(config?.default_profile)
  if (profile && profile.format_version !== 1) {
    diagnostics.push({ code: 'formatUnsupported', identity: profile.key })
    return { profile: undefined, diagnostics }
  }
  return { profile, diagnostics }
}

export function sameSource(left: RecordSectionSource, right: RecordSectionSource) {
  return left.module === right.module && left.key === right.key
}

export function sourceIdentity(source: RecordSectionSource) {
  return `${source.module}:${source.key}`
}

export interface RecordComposition {
  main: RecordSectionSource[]
  order: RecordSectionSource[]
  hidden: RecordSectionSource[]
}

export function composeRecordSections(tabs: RecordTab[], composition: RecordComposition | undefined) {
  if (!composition) return { tabs, mainSections: undefined, diagnostics: [] as ExperienceDiagnostic[] }
  const diagnostics: ExperienceDiagnostic[] = []
  const unique = (source: RecordSectionSource) => tabs.filter((tab) => tab.source && sameSource(tab.source, source))
  const hidden = composition.hidden.filter((source) => {
    const matches = unique(source)
    const permitted = composition.main.some((other) => sameSource(other, source)) && matches.length === 1 && matches[0].personallyOmittable === true
    if (!permitted) diagnostics.push({ code: 'sectionIgnored', identity: sourceIdentity(source) })
    return permitted
  })
  const sources = [...composition.order.filter((source) => composition.main.some((main) => sameSource(main, source))), ...composition.main]
    .filter((source, index, all) => all.findIndex((other) => sameSource(other, source)) === index)
  const mainSections = sources.flatMap((source) => {
    if (hidden.some((other) => sameSource(other, source))) return []
    const matches = unique(source)
    if (matches.length === 1) return matches
    diagnostics.push({ code: 'sectionUnavailable', identity: sourceIdentity(source) })
    return []
  })
  for (const tab of tabs) {
    if (tab.source && unique(tab.source).length !== 1 && !sources.some((source) => sameSource(source, tab.source!))) diagnostics.push({ code: 'sectionUnavailable', identity: sourceIdentity(tab.source) })
  }
  const available = tabs.filter((tab) => {
    if (!tab.source) return !composition.main.some((source) => source.key === tab.key)
    return unique(tab.source).length === 1 && !hidden.some((source) => sameSource(source, tab.source!))
  })
  return { tabs: [...mainSections, ...available.filter((tab) => !mainSections.includes(tab))], mainSections, diagnostics }
}
