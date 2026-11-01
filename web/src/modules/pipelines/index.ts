import { Kanban, ListChecks } from 'lucide-react'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { defineModule } from '@/kernel/modules/define'
import { StageGuidanceRecordTab } from './record-tabs'
import { PIPELINE_STAGE_REFERENCE_OPTIONS } from './reference-options'
import { DEAL_RECORD_TYPE } from './resource'
import { rottingOf } from './rotting/lib'
import { RottingDealBadge } from './rotting/RottingDealBadge'
import { RottingFilterToggle } from './rotting/RottingFilterToggle'
import { pipelinesSettingsRoute } from './routes'
import './slots'

export const STAGE_GUIDANCE_TAB_KEY = 'stage-guidance'

const pipelinesModule = defineModule({
  key: 'pipelines',
  required: true,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settings('organization', {
      group: 'sales',
      path: 'pipelines',
      label: 'settings:tabs.pipelines',
      description: 'settings:items.pipelines.description',
      icon: Kanban,
      permission: SETTINGS_MANAGE,
      order: 100,
      routes: [pipelinesSettingsRoute],
    })
    app.referenceOptions('pipeline_stage', PIPELINE_STAGE_REFERENCE_OPTIONS)
    app.recordSection(DEAL_RECORD_TYPE, {
      key: STAGE_GUIDANCE_TAB_KEY,
      label: 'guidance.panel.tab',
      icon: ListChecks,
      order: 10,
      render: StageGuidanceRecordTab,
    })
    app.slot('record.list.toolbar', {
      key: 'rotting-filter',
      recordTypes: [DEAL_RECORD_TYPE],
      render: RottingFilterToggle,
    })
    app.slot('deals.badges', { key: 'rotting', render: RottingDealBadge })
    app.filter('deals.tone', (tone, { deal }) => (rottingOf(deal).isRotting ? 'warning' : tone))
  },
})

export default pipelinesModule
