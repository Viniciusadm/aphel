import { Kanban, ListChecks } from 'lucide-react'
import i18n from '@/i18n'
import { SETTINGS_MANAGE } from '@/sdk'
import { defineModule } from '@/kernel/modules/define'
import { StageGuidanceRecordTab } from './record-tabs'
import { PIPELINE_STAGE_REFERENCE_OPTIONS } from './reference-options'
import { isStagedRecord } from './traits'
import { RottingBadge } from './rotting/RottingBadge'
import { RottingFilterToggle } from './rotting/RottingFilterToggle'
import { pipelinesSettingsRoute } from './routes'
import './slots'

export const STAGE_GUIDANCE_TAB_KEY = 'stage-guidance'

const pipelinesModule = defineModule({
  key: 'workflow',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.provideComponent('workflow.RottingBadge', RottingBadge)
    app.ruleError('stage_checklist_incomplete', (messages) =>
      i18n.t('guidance.errors.checklistIncomplete', { ns: 'workflow', items: messages.join(', ') }),
    )
    app.settings('organization', {
      group: 'records',
      path: 'workflow',
      label: 'settings:tabs.workflow',
      description: 'settings:items.workflow.description',
      icon: Kanban,
      permission: SETTINGS_MANAGE,
      order: 100,
      routes: [pipelinesSettingsRoute],
    })
    app.referenceOptions('pipeline_stage', PIPELINE_STAGE_REFERENCE_OPTIONS)
    app.recordSection(isStagedRecord, {
      key: STAGE_GUIDANCE_TAB_KEY,
      label: 'guidance.panel.tab',
      icon: ListChecks,
      order: 10,
      render: StageGuidanceRecordTab,
    })
    app.slot('record.list.toolbar', {
      key: 'rotting-filter',
      recordTypes: isStagedRecord,
      render: RottingFilterToggle,
    })
  },
})

export default pipelinesModule
