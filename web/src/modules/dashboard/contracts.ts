import type { LayoutPeriod } from '@/sdk'

declare module '@/kernel/modules/contract-components' {
  interface ContractComponentMap {
    'dashboard.PeriodSelector': { value: LayoutPeriod; onChange: (period: LayoutPeriod) => void }
    'dashboard.CompareToggle': { checked: boolean; onChange: (checked: boolean) => void }
  }
}
