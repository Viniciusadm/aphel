import type { ReactNode } from 'react'
import { ExperienceContext } from './experience-context'
import { useExperienceResolution } from './use-experience-resolution'

export function ExperienceProvider({ children }: { children: ReactNode }) {
  const resolution = useExperienceResolution()
  return <ExperienceContext.Provider value={resolution}>{children}</ExperienceContext.Provider>
}
