import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { NativeSelect } from '@/components/ui/native-select'
import { recordResources } from '@/features/records/resources'
import { APP_FRAME_CONTRACT } from '@/kernel/modules/app-frame'
import { RECORD_PRESENTATION_CONTRACT } from '@/kernel/modules/record-presentation'
import { contractBucket } from '@/kernel/modules/contract-components'
import { activeContributions, useModuleState } from '@/kernel/modules/registry'
import { sourceKey, type Profile } from './model'

export function ProfileEditor({ initial, existing, onDone, onCancel }: { initial?: Profile; existing: string[]; onDone: (profile: Profile) => void; onCancel: () => void }) {
  const { t } = useTranslation('experiences-settings')
  const id = useId()
  const modules = useModuleState()
  const [profile, setProfile] = useState<Profile>(() => initial ?? { format_version: 1, key: '', label: '', frame: { module: 'kernel' }, record_detail: { record_type: '', presentation: { module: 'kernel' } } })
  const [source, setSource] = useState({ module: '', key: '' })
  const providers = (contract: string, saved: string) => [...new Set([saved, ...activeContributions(contractBucket(contract), modules).map(({ module }) => module)])]
  const main = profile.record_detail.regions?.main
  const invalid = !profile.key.trim() || !profile.label.trim() || !profile.record_detail.record_type || (!initial && existing.includes(profile.key))
  return (
    <form className="grid gap-3" onSubmit={(event) => { event.preventDefault(); if (!invalid) onDone(profile) }}>
      <Label htmlFor={`${id}-key`}>{t('key')}</Label>
      <Input id={`${id}-key`} value={profile.key} disabled={Boolean(initial)} required maxLength={128} onChange={(event) => setProfile({ ...profile, key: event.target.value })} placeholder="org.work" />
      <Label htmlFor={`${id}-label`}>{t('label')}</Label>
      <Input id={`${id}-label`} value={profile.label} required maxLength={120} onChange={(event) => setProfile({ ...profile, label: event.target.value })} />
      <Label htmlFor={`${id}-record`}>{t('recordType')}</Label>
      <NativeSelect id={`${id}-record`} value={profile.record_detail.record_type} required onChange={(event) => setProfile({ ...profile, record_detail: { ...profile.record_detail, record_type: event.target.value } })}>
        <option value="">{t('choose')}</option>
        {[...new Set([profile.record_detail.record_type, ...recordResources().map((resource) => resource.recordType)])].filter(Boolean).map((key) => <option key={key} value={key}>{key}</option>)}
      </NativeSelect>
      <Label htmlFor={`${id}-frame`}>{t('frame')}</Label>
      <NativeSelect id={`${id}-frame`} value={profile.frame.module} onChange={(event) => setProfile({ ...profile, frame: { module: event.target.value } })}>
        {providers(APP_FRAME_CONTRACT, profile.frame.module).map((key) => <option key={key}>{key}</option>)}
      </NativeSelect>
      <Label htmlFor={`${id}-presentation`}>{t('presentation')}</Label>
      <NativeSelect id={`${id}-presentation`} value={profile.record_detail.presentation.module} onChange={(event) => {
        const { regions, ...detail } = profile.record_detail
        setProfile({ ...profile, record_detail: { ...detail, ...(event.target.value !== 'kernel' && regions ? { regions } : {}), presentation: { module: event.target.value } } })
      }}>
        {providers(RECORD_PRESENTATION_CONTRACT, profile.record_detail.presentation.module).map((key) => <option key={key}>{key}</option>)}
      </NativeSelect>
      <p className="text-sm text-muted-foreground">{t('providerHint')}</p>
      {profile.record_detail.presentation.module !== 'kernel' && (
        <fieldset className="grid gap-3 rounded-md border p-3">
          <legend>{t('sections')}</legend>
          <p className="text-sm text-muted-foreground">{t('sectionsHint')}</p>
          <ol className="grid gap-2">
            {(main ?? []).map((item, index) => <li key={sourceKey(item)} className="flex flex-wrap items-center gap-2">
              <span className="min-w-0 break-all">{sourceKey(item)}</span>
              <Button type="button" size="sm" variant="outline" disabled={index === 0} aria-label={t('moveUp', { key: sourceKey(item) })} onClick={() => {
                const next = [...(main ?? [])]; [next[index - 1], next[index]] = [next[index], next[index - 1]]
                setProfile({ ...profile, record_detail: { ...profile.record_detail, regions: { main: next } } })
              }}>{t('up')}</Button>
              <Button type="button" size="sm" variant="outline" aria-label={t('removeSource', { key: sourceKey(item) })} onClick={() => setProfile({ ...profile, record_detail: { ...profile.record_detail, regions: { main: main?.filter((_, position) => position !== index) ?? [] } } })}>{t('remove')}</Button>
            </li>)}
          </ol>
          <Label htmlFor={`${id}-source-module`}>{t('sourceModule')}</Label>
          <Input id={`${id}-source-module`} value={source.module} onChange={(event) => setSource({ ...source, module: event.target.value })} />
          <Label htmlFor={`${id}-source-key`}>{t('sourceKey')}</Label>
          <Input id={`${id}-source-key`} value={source.key} onChange={(event) => setSource({ ...source, key: event.target.value })} />
          <Button type="button" variant="outline" disabled={!source.module || !source.key || main?.some((item) => sourceKey(item) === sourceKey(source))} onClick={() => {
            setProfile({ ...profile, record_detail: { ...profile.record_detail, regions: { main: [...(main ?? []), source] } } }); setSource({ module: '', key: '' })
          }}>{t('addSource')}</Button>
        </fieldset>
      )}
      <p className="text-sm text-muted-foreground">{t('draftOnly')}</p>
      <div className="flex gap-2">
        <Button type="button" variant="outline" onClick={onCancel}>{t('cancel')}</Button>
        <Button type="submit" disabled={invalid}>{t('useDraft')}</Button>
      </div>
    </form>
  )
}
