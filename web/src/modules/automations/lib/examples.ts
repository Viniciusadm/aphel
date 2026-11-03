import type { TFunction } from 'i18next'
import type { FilterNode } from '@/features/records/types'
import type { CreateAutomationPayload, StepSpec } from '../types'

export type ExampleKey = 'follow_up_proposal' | 'stale_deal' | 'wait_then_task' | 'welcome_contact' | 'deal_won'

export interface ExampleContext {
  t: TFunction<'automations'>
  userId: string
  stages: { id: string; name: string }[]
}

export interface AutomationExample {
  key: ExampleKey
  recordType: string
  trigger: string
  build: (context: ExampleContext) => CreateAutomationPayload
}

const OPEN_DEAL: FilterNode = { op: 'and', conditions: [{ field: 'status', operator: 'eq', value: 'open' }] }

export function asTemplate(text: string): string {
  return text.replace(/\[\[([^\]]+)\]\]/g, '{{$1}}')
}

function proposalStage(stages: ExampleContext['stages']): string | undefined {
  return stages.find((stage) => /proposta|proposal/i.test(stage.name))?.id
}

function definition(steps: StepSpec[]) {
  return { start: steps[0]?.id, steps }
}

export const AUTOMATION_EXAMPLES: AutomationExample[] = [
  {
    key: 'follow_up_proposal',
    recordType: 'deals.deal',
    trigger: 'deal.stage_changed',
    build: ({ t, stages }) => {
      const stage = proposalStage(stages)
      const taskTitle = asTemplate(t('examples.follow_up_proposal.taskTitle'))
      return {
        name: t('examples.follow_up_proposal.name'),
        description: t('examples.follow_up_proposal.description'),
        record_type: 'deals.deal',
        trigger: stage ? { type: 'deal.stage_changed', to_stage_id: stage } : { type: 'deal.stage_changed' },
        definition: definition([
          {
            id: 'ainda_aberta',
            label: t('examples.follow_up_proposal.conditionLabel'),
            type: 'condition',
            filter: {
              op: 'and',
              conditions: [
                { field: 'status', operator: 'eq', value: 'open' },
                {
                  related: {
                    relationship: 'tasks',
                    none: true,
                    filter: {
                      op: 'and',
                      conditions: [
                        { field: 'status', operator: 'eq', value: 'open' },
                        { field: 'title', operator: 'contains', value: t('examples.follow_up_proposal.taskMarker') },
                      ],
                    },
                  },
                },
              ],
            },
            next: 'criar_tarefa',
          },
          {
            id: 'criar_tarefa',
            type: 'action',
            action: 'create_task',
            config: { title: taskTitle, assignee: { type: 'record_owner' }, due_in: '2d' },
          },
        ]),
      }
    },
  },
  {
    key: 'stale_deal',
    recordType: 'deals.deal',
    trigger: 'time.record_stale',
    build: ({ t, userId }) => ({
      name: t('examples.stale_deal.name'),
      description: t('examples.stale_deal.description'),
      record_type: 'deals.deal',
      trigger: { type: 'time.record_stale', mode: 'no_update', days: 7 },
      definition: definition([
        { id: 'ainda_aberta', type: 'condition', filter: OPEN_DEAL, next: 'notificar_gestor' },
        {
          id: 'notificar_gestor',
          label: t('examples.stale_deal.actionLabel'),
          type: 'action',
          action: 'send_notification',
          config: {
            recipients: [{ type: 'user', user_id: userId }],
            title: asTemplate(t('examples.stale_deal.notificationTitle')),
            body: asTemplate(t('examples.stale_deal.notificationBody')),
          },
        },
      ]),
    }),
  },
  {
    key: 'wait_then_task',
    recordType: 'deals.deal',
    trigger: 'record.created',
    build: ({ t }) => ({
      name: t('examples.wait_then_task.name'),
      description: t('examples.wait_then_task.description'),
      record_type: 'deals.deal',
      trigger: { type: 'record.created' },
      definition: definition([
        {
          id: 'esperar_2d',
          label: t('examples.wait_then_task.waitLabel'),
          type: 'wait',
          duration: '2d',
          recheck: OPEN_DEAL,
          next: 'criar_tarefa',
        },
        {
          id: 'criar_tarefa',
          type: 'action',
          action: 'create_task',
          config: { title: asTemplate(t('examples.wait_then_task.taskTitle')), assignee: { type: 'record_owner' }, due_in: '1d' },
        },
      ]),
    }),
  },
  {
    key: 'welcome_contact',
    recordType: 'people.person',
    trigger: 'record.created',
    build: ({ t }) => ({
      name: t('examples.welcome_contact.name'),
      description: t('examples.welcome_contact.description'),
      record_type: 'people.person',
      trigger: { type: 'record.created' },
      definition: definition([
        {
          id: 'primeiro_contato',
          type: 'action',
          action: 'create_task',
          config: { title: asTemplate(t('examples.welcome_contact.taskTitle')), assignee: { type: 'record_owner' }, due_in: '1d' },
        },
      ]),
    }),
  },
  {
    key: 'deal_won',
    recordType: 'deals.deal',
    trigger: 'deal.won',
    build: ({ t }) => ({
      name: t('examples.deal_won.name'),
      description: t('examples.deal_won.description'),
      record_type: 'deals.deal',
      trigger: { type: 'deal.won' },
      definition: definition([
        {
          id: 'avisar_responsavel',
          type: 'action',
          action: 'send_notification',
          config: {
            recipients: [{ type: 'record_owner' }],
            title: asTemplate(t('examples.deal_won.notificationTitle')),
            body: asTemplate(t('examples.deal_won.notificationBody')),
          },
        },
      ]),
    }),
  },
]
