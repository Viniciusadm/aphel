import { useTranslation } from 'react-i18next'
import { getRecordResource } from '@/features/records/resources'
import { contractBucket } from '@/kernel/modules/contract-components'
import { APP_FRAME_CONTRACT } from '@/kernel/modules/app-frame'
import { RECORD_PRESENTATION_CONTRACT, type RecordPresentationComponent } from '@/kernel/modules/record-presentation'
import { activeContributions, isModuleActive, moduleDefinition, useModuleState } from '@/kernel/modules/registry'
import { changedProfiles, sourceKey, type Catalog, type CatalogView } from './model'

function EntrySummary({ entry }: { entry: Catalog['profiles'][number] }) {
  const { t } = useTranslation('experiences-settings')
  const profile = entry.profile
  return <dl className="grid gap-1 break-words text-sm">
    <div><dt className="font-medium">{t('label')}</dt><dd>{profile.label} · {t(entry.enabled ? 'enabled' : 'disabled')}</dd></div>
    <div><dt className="font-medium">{t('frame')}</dt><dd>{profile.frame.module}</dd></div>
    <div><dt className="font-medium">{t('recordType')}</dt><dd>{profile.record_detail.record_type}</dd></div>
    <div><dt className="font-medium">{t('presentation')}</dt><dd>{profile.record_detail.presentation.module}</dd></div>
    <div><dt className="font-medium">{t('sections')}</dt><dd>{profile.record_detail.regions?.main.map(sourceKey).join(' → ') || t('providerDefault')}</dd></div>
  </dl>
}

export function CatalogReview({ before, report }: { before: Catalog; report: CatalogView }) {
  const { t } = useTranslation('experiences-settings')
  const modules = useModuleState()
  const after = report.configuration
  if (!after) return null
  const frames = activeContributions(contractBucket(APP_FRAME_CONTRACT), modules)
  const presentations = activeContributions<RecordPresentationComponent>(contractBucket(RECORD_PRESENTATION_CONTRACT), modules)
  return (
    <section className="grid gap-4 rounded-md border p-4" aria-label={t('review')}>
      <h2 className="font-semibold">{t('review')}</h2>
      <p role="note">{t('sharedWarning')}</p>
      <p>{t('defaultChange', { before: before.default_profile ?? t('baseline'), after: after.default_profile ?? t('baseline') })}</p>
      {before.policy.allowed_profiles.some((key) => !after.policy.allowed_profiles.includes(key)) && <p className="text-warning">{t('withdrawWarning')}</p>}
      <ul className="grid gap-3">
        {changedProfiles(before, after).map(({ key, previous, next }) => <li key={key} className="grid gap-2 rounded border p-3">
          <h3 className="break-all font-medium">{key} · {t(!next ? 'removed' : !previous ? 'added' : 'changed')}</h3>
          {previous && (!next || previous.enabled && !next.enabled || !after.policy.allowed_profiles.includes(key)) && <p className="text-warning">{t('withdrawWarning')}</p>}
          {previous && <details><summary>{t('before')}</summary><EntrySummary entry={previous} /></details>}
          {next && <details open><summary>{t('after')}</summary><EntrySummary entry={next} /></details>}
        </li>)}
      </ul>
      {JSON.stringify(before.policy) !== JSON.stringify(after.policy) && <div className="grid gap-2">
        <h3 className="font-medium">{t('policyChanged')}</h3>
        <ul className="grid gap-1 text-sm">
          <li>{t('allowed')}: {before.policy.allowed_profiles.join(', ') || t('none')} → {after.policy.allowed_profiles.join(', ') || t('none')}</li>
          {(['profile_selection', 'navigation', 'section_order'] as const).map((key) => <li key={key}>
            {t(key === 'profile_selection' ? 'allowSelection' : key === 'navigation' ? 'allowNavigation' : 'allowOrder')}: {t(before.policy.customization[key] ? 'enabled' : 'disabled')} → {t(after.policy.customization[key] ? 'enabled' : 'disabled')}
          </li>)}
          <li>{t('hideAllowlist')}: {before.policy.customization.section_hidden.map(sourceKey).join(', ') || t('none')} → {after.policy.customization.section_hidden.map(sourceKey).join(', ') || t('none')}</li>
        </ul>
      </div>}
      <h3 className="font-medium">{t('availability')}</h3>
      <p className="text-sm text-muted-foreground">{t('previewHint')}</p>
      {!modules.loaded && <p role="status">{t('modulesLoading')}</p>}
      <ul className="grid gap-3">
        {after.profiles.map(({ profile, enabled }) => {
          const detail = profile.record_detail
          const matches = presentations.filter(({ module }) => module === detail.presentation.module)
          const presentation = matches.length === 1 ? matches[0].value : undefined
          const compatible = detail.presentation.module === 'kernel' ? !detail.regions : presentation?.experience?.recordTypes.includes(detail.record_type) && (!detail.regions || presentation.experience.regions.includes('main'))
          return <li key={profile.key} className="grid gap-1 break-words">
            <h4 className="font-medium">{profile.label} · {enabled ? t('enabled') : t('disabled')}</h4>
            <p>{t('frame')}: {profile.frame.module} — {t(modules.loaded && frames.filter(({ module }) => module === profile.frame.module).length === 1 ? 'registered' : 'unavailable')}</p>
            <p>{t('presentation')}: {detail.presentation.module} — {t(modules.loaded && presentation && compatible ? 'compatible' : 'unavailable')}</p>
            <p>{t('recordType')}: {detail.record_type} — {t(getRecordResource(detail.record_type) ? 'registered' : 'unavailable')}</p>
            {(detail.regions?.main ?? []).map((source) => <p key={sourceKey(source)}>{sourceKey(source)} — {t(moduleDefinition(source.module) && isModuleActive(source.module, modules) ? 'contextRequired' : 'unavailable')}</p>)}
          </li>
        })}
      </ul>
      {report.diagnostics.length > 0 && <details open>
        <summary>{t('serverReport')}</summary>
        <ul className="grid gap-1 text-sm">
          {report.diagnostics.map((diagnostic, index) => <li key={index} className="break-all">{diagnostic.profile_key} · {diagnostic.path}: {t(diagnostic.code === 'record_type_unavailable' ? 'unavailable' : 'contextRequired')}</li>)}
        </ul>
      </details>}
      <p className="text-sm text-muted-foreground">{t('codeWarning')}</p>
    </section>
  )
}
