import { Link } from 'react-router'
import { getRecordResource } from '@/features/records/resources'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { AI_KEY, type AiSource } from './api'


export function Sources({ sources }: { sources: AiSource[] }) {
  const t = useModuleTranslation(AI_KEY)
  if (sources.length === 0) return null
  const counts = new Map<string, number>()
  for (const source of sources) counts.set(source.record_type, (counts.get(source.record_type) ?? 0) + 1)
  const linked = sources.filter((source) => getRecordResource(source.record_type)?.detailPath && source.label)

  return (
    <details className="text-xs text-muted-foreground">
      <summary className="cursor-pointer select-none">
        {t('sources.title', {
          summary: [...counts.entries()]
            .map(([type, count]) => t('sources.count', { count, type: t(`sources.types.${type}`, { defaultValue: type }) }))
            .join(', '),
        })}
      </summary>
      {linked.length > 0 && (
        <ul className="mt-1 flex flex-wrap gap-1.5">
          {linked.map((source) => {
            const path = getRecordResource(source.record_type)?.detailPath?.(source.record_id)
            return (
              <li key={`${source.record_type}-${source.record_id}`}>
                {path ? (
                  <Link className="rounded-full border px-2 py-0.5 hover:bg-accent" to={path}>
                    {source.label}
                  </Link>
                ) : (
                  <span className="rounded-full border px-2 py-0.5">{source.label}</span>
                )}
              </li>
            )
          })}
        </ul>
      )}
    </details>
  )
}
