import { Check, Clock, X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { formatDateTime, useRecordFormatting } from '@/sdk'
import { cn } from '@/lib/utils'
import type {ApprovalRequestStep} from '@/sdk'
import { ApprovalStepBadge } from './StatusBadge'

export interface ApprovalStepsProps {
  steps: ApprovalRequestStep[]
  compact?: boolean
}

export function ApprovalSteps({ steps, compact = false }: ApprovalStepsProps) {
  const { t } = useTranslation('approvals')
  const formatting = useRecordFormatting()
  return (
    <ol className="grid gap-2" aria-label={t('request.steps')}>
      {steps.map((step, index) => (
        <li
          key={step.position}
          className={cn(
            'grid gap-2 rounded-md border px-3 py-2',
            step.state === 'active' && 'border-warning bg-warning/5',
          )}
        >
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-sm font-medium">
              {step.name?.trim() || t('request.stepNumber', { number: index + 1 })}
            </span>
            <span className="text-xs text-muted-foreground">
              {t(`approverKinds.${step.approver_kind}`)} · {t(`rules.${step.rule}`)}
            </span>
            <span className="ml-auto">
              <ApprovalStepBadge state={step.state} />
            </span>
          </div>
          {step.approvers.length > 0 && (
            <ul className="grid gap-1.5">
              {step.approvers.map((approver) => (
                <li key={approver.user_id} className="grid gap-0.5 text-sm">
                  <span className="flex flex-wrap items-center gap-1.5">
                    {approver.decision === 'approved' ? (
                      <Check className="size-3.5 text-success" aria-hidden="true" />
                    ) : approver.decision === 'rejected' ? (
                      <X className="size-3.5 text-destructive" aria-hidden="true" />
                    ) : (
                      <Clock className="size-3.5 text-muted-foreground" aria-hidden="true" />
                    )}
                    <span>{approver.display_name}</span>
                    <span className="text-xs text-muted-foreground">
                      {approver.decision ? t(`decisions.${approver.decision}`) : t('decisions.waiting')}
                      {approver.decided_at && ` · ${formatDateTime(approver.decided_at, formatting)}`}
                    </span>
                  </span>
                  {!compact && approver.comment && (
                    <p className="ml-5 text-xs whitespace-pre-line text-muted-foreground">{approver.comment}</p>
                  )}
                </li>
              ))}
            </ul>
          )}
        </li>
      ))}
    </ol>
  )
}
