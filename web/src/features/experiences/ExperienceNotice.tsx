import { useTranslation } from 'react-i18next'
import { Alert } from '@/components/ui/alert'
import type { ExperienceDiagnostic } from './resolution'

export function ExperienceNotice({ diagnostics }: { diagnostics: ExperienceDiagnostic[] }) {
  const { t } = useTranslation('layout')
  const unique = diagnostics.filter((item, index, all) => all.findIndex((other) => other.code === item.code && other.identity === item.identity) === index)
  if (!unique.length) return null
  return <Alert role="status" className="mb-4">
    <ul>{unique.map(({ code, identity }, index) => <li key={`${code}:${identity}:${index}`}>{t(`experience.${code}`, { identity })}</li>)}</ul>
  </Alert>
}
