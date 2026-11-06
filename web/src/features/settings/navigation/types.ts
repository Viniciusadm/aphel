import type { components } from '@/api'

type Schemas = components['schemas']

export type SupportedLanguage = 'pt-BR' | 'en'

export type NavigationPreferences = Required<Schemas['NavigationPreferencesDto']>
export type PreferencesResponse = Schemas['PreferencesResponse']
export type UpdatePersonalPreferences = Schemas['UpdatePreferencesRequest']

export interface PersonalPreferences {
  keyboard_shortcuts: boolean
  locale: string | null
  navigation: NavigationPreferences
  experience: Schemas['ExperiencePreferences']
}
