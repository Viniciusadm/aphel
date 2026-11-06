import { Pencil } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import {
  Button,
  FieldValueDisplay,
  RecordFieldsPanel,
  RecordHeader,
  RecordPageShell,
  RecordTabs,
  ShareRecordButton,
  Slot,
  UserValue,
  type RecordPresentationProps,
  type RecordTab,
} from '@/sdk'

export function WorkRecordPresentation({ model, defaultTab, className }: RecordPresentationProps) {
  const { t } = useTranslation('records-detail')
  const primary = model.mainSections ?? []
  const remaining = model.tabs.filter((section) => !primary.includes(section))
  const sections: RecordTab[] = [
    {
      key: 'overview',
      label: t('overview.title'),
      content: (
        <div className="grid min-w-0 gap-6">
          {primary.map((section) => (
            <section key={section.source ? `${section.source.module}:${section.source.key}` : section.key} aria-label={section.label} className="min-w-0 rounded-lg border bg-card p-4">
              {section.content}
            </section>
          ))}
        </div>
      ),
    },
    ...primary,
    ...remaining,
  ]
  const subtitle = model.subtitle
  const subtitleValue = subtitle?.value

  return (
    <RecordPageShell
      className={className}
      header={
        <>
          <RecordHeader
            title={model.title}
            back={model.back}
            titleAccessory={<Slot name="record.detail.title" props={model.slotProps} />}
            subtitle={subtitleValue == null || subtitleValue === '' ? undefined : subtitle?.field
              ? <FieldValueDisplay field={subtitle.field} value={subtitleValue} />
              : String(subtitleValue)}
            owner={typeof model.owner.value === 'string' && model.owner.value ? <UserValue userId={model.owner.value} /> : undefined}
            primaryAction={!model.readOnly ? <Button size="sm" onClick={model.edit}><Pencil aria-hidden />{t('header.edit')}</Button> : undefined}
            actions={<><ShareRecordButton recordType={model.recordType} recordId={model.recordId} />{model.actions}<Slot name="record.detail.actions" props={model.slotProps} /></>}
          />
          <Slot name="record.detail.header" props={model.slotProps} />
        </>
      }
      sidebar={
        <div className="grid min-w-0 gap-4">
          <RecordFieldsPanel fields={model.fields} layout={model.viewLayout} record={model.record} readOnly={model.readOnly} readOnlyFieldKeys={model.readOnlyFieldKeys} onSaveField={model.saveField}>
            <Slot name="record.fields.panel" props={model.slotProps} />
          </RecordFieldsPanel>
          <Slot name="record.detail.sidebar" props={model.slotProps} />
        </div>
      }
    >
      <div className="grid min-w-0 gap-4">
        <RecordTabs tabs={sections} defaultTab={defaultTab ?? 'overview'} />
      </div>
    </RecordPageShell>
  )
}

export function BrokenRecordSection(): never {
  throw new Error('Development fixture: record section failed')
}

export function BrokenRecordPresentation(): never {
  throw new Error('Development fixture: record presentation failed')
}

WorkRecordPresentation.experience = { recordTypes: ['deals.deal'], regions: ['main'] as const }

BrokenRecordPresentation.experience = WorkRecordPresentation.experience
