import type { PackageTemplate } from '../api'
import type { OnboardingRecommendation } from '../types'

export interface TemplateOption {
  template: PackageTemplate
  reasons: string[]
  recommended: boolean
}

export function rankTemplates(
  templates: PackageTemplate[],
  recommendation: OnboardingRecommendation | null,
): TemplateOption[] {
  const byKey = new Map(templates.map((template) => [template.key, template]))
  const ranked: TemplateOption[] = []
  const seen = new Set<string>()
  const suggestions = recommendation ? [recommendation.recommended, ...recommendation.alternatives] : []
  for (const suggestion of suggestions) {
    const template = byKey.get(suggestion.key)
    if (!template || seen.has(template.key)) continue
    seen.add(template.key)
    ranked.push({
      template,
      reasons: suggestion.reasons,
      recommended: ranked.length === 0 && suggestion.key === recommendation?.recommended.key,
    })
  }
  for (const template of templates) {
    if (seen.has(template.key)) continue
    ranked.push({ template, reasons: [], recommended: false })
  }
  return ranked
}
