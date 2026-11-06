import { useQuery } from '@tanstack/react-query'
import type { ComponentType } from 'react'
import { api, type components } from '@/api'
import { usePreferences } from '@/features/command-palette/preferences/api'
import { getRecordResource } from '@/features/records/resources'
import { EMPTY_NAVIGATION } from '@/features/settings/navigation/api'
import type { NavigationPreferences } from '@/features/settings/navigation/types'
import { APP_FRAME_CONTRACT, DEFAULT_APP_FRAME, type AppFrameProps } from '@/kernel/modules/app-frame'
import { contractBucket } from '@/kernel/modules/contract-components'
import { activeContributions, useModuleState } from '@/kernel/modules/registry'
import { selectExperience, type ExperienceDiagnostic } from './resolution'

export const organizationExperiencesKey = ['organization', 'experiences'] as const

export function useExperienceResolution() {
  const catalog = useQuery({
    queryKey: organizationExperiencesKey,
    queryFn: () => api.get<components['schemas']['OrganizationExperiencesView']>('/v1/organization/experiences'),
    staleTime: 5 * 60_000,
  })
  const preferences = usePreferences()
  const modules = useModuleState()
  const paused = catalog.fetchStatus === 'paused' || preferences.fetchStatus === 'paused'
  const config = catalog.isError ? null : catalog.data?.configuration
  const personal = preferences.isError || catalog.isError || !catalog.data ? undefined : preferences.data?.stored_experience
  const { profile, diagnostics: selectionDiagnostics } = selectExperience(config, personal)
  const ready = (!catalog.isPending && !preferences.isPending || paused) && (modules.loaded || paused)
  const requestedFrame = profile?.frame.module ?? DEFAULT_APP_FRAME
  const providers = activeContributions<ComponentType<AppFrameProps>>(contractBucket(APP_FRAME_CONTRACT), modules)
  const matches = providers.filter(({ module }) => module === requestedFrame)
  const Frame = ready && matches.length === 1 ? matches[0].value : undefined
  const diagnostics: ExperienceDiagnostic[] = [...selectionDiagnostics]
  if (catalog.isError || preferences.isError || paused && (!catalog.data || !preferences.data)) diagnostics.push({ code: 'loadFailed', identity: '' })
  if (ready && matches.length !== 1) diagnostics.push({ code: 'frameUnavailable', identity: requestedFrame })
  if (profile && !getRecordResource(profile.record_detail.record_type)) diagnostics.push({ code: 'recordTypeUnavailable', identity: profile.record_detail.record_type })
  const savedNavigation = preferences.isError ? undefined : preferences.data?.stored_navigation ?? preferences.data?.navigation
  if (config?.policy.customization.navigation === false && (savedNavigation?.home || savedNavigation?.order?.length || savedNavigation?.hidden?.length)) diagnostics.push({ code: 'navigationIgnored', identity: '' })
  const navigation: NavigationPreferences = !ready || catalog.isError || !catalog.data || config?.policy.customization.navigation === false
    ? EMPTY_NAVIGATION : { order: savedNavigation?.order ?? [], hidden: savedNavigation?.hidden ?? [], home: savedNavigation?.home ?? null }
  return { ready, config, personal, profile, Frame, requestedFrame, diagnostics, navigation }
}

