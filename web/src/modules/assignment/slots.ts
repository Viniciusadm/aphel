export interface DistributionRuleSelectProps {
  recordType: string
  value: string | null
  onChange: (ruleId: string | null) => void
  disabled?: boolean
}

declare module '@/kernel/modules/slots' {
  interface SlotPropsMap {
    'assignment.ruleSelect': DistributionRuleSelectProps
  }
}
