import { useMemo } from 'react'
import { contractBucket } from '@/kernel/modules/contract-components'
import { DEFAULT_RECORD_PRESENTATION, RECORD_PRESENTATION_CONTRACT, type RecordPresentationComponent } from '@/kernel/modules/record-presentation'
import { activeContributions, useModuleState } from '@/kernel/modules/registry'
import { useExperience } from './experience-context'
import { sameSource, type ExperienceDiagnostic, type RecordComposition } from './resolution'

export function useRecordExperience(recordType: string) {
  const experience = useExperience()
  const modules = useModuleState()
  const detail = experience?.profile?.record_detail
  const selected = detail?.record_type === recordType ? detail : undefined
  const requested = selected?.presentation.module ?? DEFAULT_RECORD_PRESENTATION
  const matches = activeContributions<RecordPresentationComponent>(contractBucket(RECORD_PRESENTATION_CONTRACT), modules).filter(({ module }) => module === requested)
  const candidate = matches.length === 1 ? matches[0].value : undefined
  const compatible = requested === DEFAULT_RECORD_PRESENTATION
    ? !selected?.regions
    : candidate?.experience?.recordTypes.includes(recordType) && (!selected?.regions || candidate.experience.regions.includes('main'))
  const Presentation = compatible ? candidate : undefined
  const diagnostics: ExperienceDiagnostic[] = []
  if (modules.loaded && !Presentation) diagnostics.push({ code: candidate && !compatible ? 'compositionUnsupported' : 'presentationUnavailable', identity: requested })
  const policy = experience?.config?.policy.customization
  const saved = experience?.personal?.record_detail
  const adjustment = saved?.record_type === recordType ? saved : undefined
  if (saved && saved.record_type !== recordType && selected) diagnostics.push({ code: 'sectionIgnored', identity: saved.record_type })
  if (adjustment && selected && !policy?.section_order && adjustment.order.length) diagnostics.push({ code: 'sectionIgnored', identity: selected.record_type })
  const main = Presentation ? selected?.regions?.main : undefined
  if (adjustment && selected && !main && (adjustment.order.length || adjustment.hidden.length)) diagnostics.push({ code: 'sectionIgnored', identity: selected.record_type })
  const composition = useMemo<RecordComposition | undefined>(() => main && {
    main,
    order: policy?.section_order ? adjustment?.order ?? [] : [],
    hidden: (adjustment?.hidden ?? []).filter((source) => main.some((other) => sameSource(other, source)) && policy?.section_hidden.some((other) => sameSource(other, source))),
  }, [main, policy, adjustment])
  if (adjustment && main) {
    for (const source of [...adjustment.order, ...adjustment.hidden]) {
      if (!main.some((other) => sameSource(source, other))) diagnostics.push({ code: 'sectionIgnored', identity: `${source.module}:${source.key}` })
    }
    for (const source of adjustment.hidden) {
      if (!policy?.section_hidden.some((other) => sameSource(source, other))) diagnostics.push({ code: 'sectionIgnored', identity: `${source.module}:${source.key}` })
    }
  }
  return { Presentation, requested, composition, diagnostics, profileKey: experience?.profile?.key, detail: selected }
}
