export type RuleErrorFormatter = (messages: string[]) => string

const formatters = new Map<string, RuleErrorFormatter>()

export function registerRuleError(code: string, formatter: RuleErrorFormatter) {
  formatters.set(code, formatter)
}

export function formatRuleError(code: string, messages: string[]): string | null {
  return formatters.get(code)?.(messages) ?? null
}
