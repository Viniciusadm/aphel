import type { components } from '@/api'

export type Catalog = components['schemas']['OrganizationExperiences']
export type CatalogView = components['schemas']['OrganizationExperiencesView']
export type Profile = components['schemas']['ExperienceProfile']
export type Source = components['schemas']['ExperienceSection']

export const catalogPath = '/v1/organization/experiences'
export const emptyCatalog = (): Catalog => ({ profiles: [], default_profile: null, policy: { allowed_profiles: [], customization: { profile_selection: false, navigation: true, section_order: false, section_hidden: [] } } })
export const sourceKey = (source: Source) => `${source.module}:${source.key}`

export function pruneReferences(catalog: Catalog): Catalog {
  const enabled = catalog.profiles.filter((entry) => entry.enabled).map((entry) => entry.profile.key)
  const allowed = catalog.policy.allowed_profiles.filter((key) => enabled.includes(key))
  const sources = catalog.profiles.filter((entry) => entry.enabled && allowed.includes(entry.profile.key)).flatMap((entry) => entry.profile.record_detail.regions?.main ?? [])
  return { ...catalog, default_profile: allowed.includes(catalog.default_profile ?? '') ? catalog.default_profile : null, policy: { ...catalog.policy, allowed_profiles: allowed, customization: { ...catalog.policy.customization, section_hidden: catalog.policy.customization.section_hidden.filter((source) => sources.some((other) => sourceKey(other) === sourceKey(source))) } } }
}

export function changedProfiles(before: Catalog, after: Catalog) {
  return [...new Set([...before.profiles.map(({ profile }) => profile.key), ...after.profiles.map(({ profile }) => profile.key)])].flatMap((key) => {
    const previous = before.profiles.find(({ profile }) => profile.key === key)
    const next = after.profiles.find(({ profile }) => profile.key === key)
    return JSON.stringify(previous) === JSON.stringify(next) ? [] : [{ key, previous, next }]
  })
}
