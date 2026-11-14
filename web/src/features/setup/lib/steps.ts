export const DRAFT_STEPS = ['organization', 'business', 'features', 'account'] as const
export type DraftStep = (typeof DRAFT_STEPS)[number]

export function isDraftStep(value: unknown): value is DraftStep {
  return typeof value === 'string' && (DRAFT_STEPS as readonly string[]).includes(value)
}
