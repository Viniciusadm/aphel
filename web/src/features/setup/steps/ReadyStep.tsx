import { CircleAlert, CircleCheck, FileSpreadsheet, SlidersHorizontal, UsersRound } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { SetupStepFrame } from '@/kernel/setup'

export interface ReadyResult {
  ok: boolean
  text: string
}

export function ReadyStep({ results = [], onFinish }: { results?: ReadyResult[]; onFinish: () => void }) {
  const { t } = useTranslation('setup')
  const next = [
    { key: 'import', icon: FileSpreadsheet, text: t('ready.next.import') },
    { key: 'invite', icon: UsersRound, text: t('ready.next.invite') },
    { key: 'customize', icon: SlidersHorizontal, text: t('ready.next.customize') },
  ]
  return (
    <SetupStepFrame title={t('ready.title')} description={t('ready.description')} primary={{ label: t('actions.finish'), onClick: onFinish }}>
      {results.length > 0 && (
        <ul className="grid gap-2" aria-label={t('ready.done')}>
          {results.map((result) => (
            <li
              key={result.text}
              className={
                result.ok
                  ? 'flex items-start gap-3 rounded-lg border border-success/30 bg-success/5 p-3 text-sm'
                  : 'flex items-start gap-3 rounded-lg border border-warning/40 bg-warning/10 p-3 text-sm'
              }
            >
              {result.ok ? (
                <CircleCheck className="mt-0.5 size-4 shrink-0 text-success" aria-hidden />
              ) : (
                <CircleAlert className="mt-0.5 size-4 shrink-0 text-warning" aria-hidden />
              )}
              <span>{result.text}</span>
            </li>
          ))}
        </ul>
      )}
      <section className="grid gap-2" aria-labelledby="setup-next">
        <h2 id="setup-next" className="text-sm font-semibold">
          {t('ready.nextTitle')}
        </h2>
        <ul className="grid gap-2">
          {next.map(({ key, icon: Icon, text }) => (
            <li key={key} className="flex items-start gap-3 rounded-lg border bg-card p-3 text-sm">
              <Icon className="mt-0.5 size-4 shrink-0 text-muted-foreground" aria-hidden />
              <span>{text}</span>
            </li>
          ))}
        </ul>
      </section>
    </SetupStepFrame>
  )
}
