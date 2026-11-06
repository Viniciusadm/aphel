import { createContext, useContext } from 'react'
import { EMPTY_NAVIGATION } from '@/features/settings/navigation/api'
import type { useExperienceResolution } from './use-experience-resolution'

type ExperienceResolution = ReturnType<typeof useExperienceResolution>
export const ExperienceContext = createContext<ExperienceResolution | null>(null)

export function useExperience() {
  return useContext(ExperienceContext)
}

export function useExperienceNavigation() {
  return useExperience()?.navigation ?? EMPTY_NAVIGATION
}
