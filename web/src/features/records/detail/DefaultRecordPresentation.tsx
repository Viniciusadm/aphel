import { Pencil, Share2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Slot } from '@/kernel/modules/Slot'
import { useMediaQuery } from '../calendar/useMediaQuery'
import { FieldValueDisplay, UserValue } from '../display/FieldValueDisplay'
import { ShareRecordDialog } from '../sharing/ShareRecordDialog'
import { RecordActionItem } from './RecordActionItem'
import type { FieldDefinition } from '../types'
import { RecordFieldsPanel } from './RecordFieldsPanel'
import { RecordHeader } from './RecordHeader'
import { RecordMobileView } from './mobile/RecordMobileView'
import { RecordPageShell } from './RecordPageShell'
import { RecordOverview } from './RecordOverview'
import { RecordTabs } from './RecordTabs'
import type { RecordTab } from './types'
import type { RecordPresentationProps } from '@/kernel/modules/record-presentation'

const MOBILE_QUERY = '(max-width: 767px)'

export interface DefaultRecordPresentationProps extends RecordPresentationProps {}

function hasValue(value: unknown) {
  if (value === null || value === undefined) return false
  if (typeof value === 'string') return value.trim() !== ''
  return !Array.isArray(value) || value.length > 0
}

export function DefaultRecordPresentation({ model, defaultTab, className }: DefaultRecordPresentationProps) {
  const { t } = useTranslation('records-detail')
  const { t: ts } = useTranslation('sharing')
  const [sharing, setSharing] = useState(false)
  const {
    recordType, record, title, subtitle, owner, fields, viewLayout,
    visibleFields: visibleSummaryFields, summaryFieldKeys, readOnly, readOnlyFieldKeys,
    back: contextualBack, edit, saveField, tabs: resolvedTabs, slotProps, actions, primaryAction: customPrimary,
  } = model
  const priorityKeys = summaryFieldKeys
  const summaryFields = visibleSummaryFields
    .filter((field) => priorityKeys.length === 0 || priorityKeys.includes(field.key) || field.is_required)
    .sort((left, right) => {
      const priority = (field: FieldDefinition) => {
        const index = priorityKeys.indexOf(field.key)
        return index < 0 ? priorityKeys.length : index
      }
      return priority(left) - priority(right)
    })
    .slice(0, 8)
  const summaryPanel = <RecordFieldsPanel fields={summaryFields} record={record} readOnly={readOnly} readOnlyFieldKeys={readOnlyFieldKeys} onSaveField={saveField} />
  const hasSummary = resolvedTabs.some((section) => section.summary)
  const detailPanel = <div className="grid min-w-0 gap-4"><RecordFieldsPanel title={hasSummary ? undefined : ''} columns={hasSummary ? 1 : 2} fields={fields} layout={viewLayout} record={record} readOnly={readOnly} readOnlyFieldKeys={readOnlyFieldKeys} onSaveField={saveField}><Slot name="record.fields.panel" props={slotProps} /></RecordFieldsPanel><Slot name="record.detail.sidebar" props={slotProps} /></div>
  const destinations: RecordTab[] = [...resolvedTabs, { key: 'details', label: t('fields.title'), content: detailPanel }]
  const overview = destinations.some((section) => section.summary) ? <RecordOverview sections={destinations} navigation={false} /> : detailPanel
  const workspaceSections = hasSummary
    ? [{ key: 'overview', label: t('overview.title'), content: overview }, ...destinations]
    : [{ key: 'overview', label: t('fields.title'), content: detailPanel }, ...resolvedTabs]

  const isMobile = useMediaQuery(MOBILE_QUERY)
  const subtitleNode =
    subtitle && hasValue(subtitle.value) ? (
      subtitle.field ? (
        <FieldValueDisplay field={subtitle.field} value={subtitle.value} />
      ) : (
        String(subtitle.value)
      )
    ) : undefined
  const ownerNode = typeof owner.value === 'string' && owner.value ? <UserValue userId={owner.value} /> : undefined

  const shareDialog = sharing ? <ShareRecordDialog recordType={recordType} recordId={record.id} open onOpenChange={setSharing} /> : null

  if (isMobile) {
    return (
      <>
      <RecordMobileView
        className={className}
        title={title}
        titleAccessory={<Slot name="record.detail.title" props={slotProps} />}
        headerSlot={<Slot name="record.detail.header" props={slotProps} />}
        subtitle={subtitleNode}
        owner={ownerNode}
        back={contextualBack}
        tabs={destinations}
        onEdit={readOnly ? undefined : edit}
        actions={
          <>
            {customPrimary}
            <RecordActionItem icon={Share2} label={ts('button.share')} onSelect={() => setSharing(true)} />
            {actions}
            <Slot name="record.detail.actions" props={slotProps} />
          </>
        }
        overview={<RecordOverview sections={destinations} fields={summaryPanel} />}
        summaryPanel={
          <RecordFieldsPanel
            fields={summaryFields}
            record={record}
            readOnly={readOnly}
            readOnlyFieldKeys={readOnlyFieldKeys}
            onSaveField={saveField}
          />
        }
        detailsPanel={
          <div className="grid min-w-0 gap-4">
            <RecordFieldsPanel
              title=""
              className="border-0 p-0"
              fields={fields}
              layout={viewLayout}
              record={record}
              readOnly={readOnly}
              readOnlyFieldKeys={readOnlyFieldKeys}
              onSaveField={saveField}
            >
              <Slot name="record.fields.panel" props={slotProps} />
            </RecordFieldsPanel>
            <Slot name="record.detail.sidebar" props={slotProps} />
          </div>
        }
      />
      {shareDialog}
      </>
    )
  }

  return (
    <>
    <RecordPageShell
      className={className}
      header={
        <>
        <RecordHeader
          title={title}
          titleAccessory={<Slot name="record.detail.title" props={slotProps} />}
          back={contextualBack}
          primaryAction={customPrimary ?? (!readOnly ? <Button size="sm" onClick={edit}><Pencil aria-hidden />{t('header.edit')}</Button> : undefined)}
          secondaryAction={customPrimary && !readOnly ? <Button size="sm" variant="outline" onClick={edit}><Pencil aria-hidden />{t('header.edit')}</Button> : undefined}
          actions={
            <>
              <RecordActionItem icon={Share2} label={ts('button.share')} onSelect={() => setSharing(true)} />
              {actions}
              <Slot name="record.detail.actions" props={slotProps} />
            </>
          }
          subtitle={
            subtitle && hasValue(subtitle.value) ? (
              subtitle.field ? (
                <FieldValueDisplay field={subtitle.field} value={subtitle.value} />
              ) : (
                String(subtitle.value)
              )
            ) : undefined
          }
          owner={typeof owner.value === 'string' && owner.value ? <UserValue userId={owner.value} /> : undefined}
        />
        <Slot name="record.detail.header" props={slotProps} />
        </>
      }
      sidebar={hasSummary ? summaryPanel : undefined}
    >
      <RecordTabs tabs={workspaceSections} defaultTab={defaultTab ?? 'overview'} />
    </RecordPageShell>
    {shareDialog}
    </>
  )
}
