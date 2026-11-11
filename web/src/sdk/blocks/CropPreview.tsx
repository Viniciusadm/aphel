import { Field } from '@/components/ui/field'
import { fileDownloadUrl } from '@/features/files'
import { useModuleTranslation } from '@/kernel/modules/i18n'

export function CropPreview({ file, x, y, onChange, disabled }: { file: string; x: number; y: number; onChange: (x: number, y: number) => void; disabled?: boolean }) {
  const t = useModuleTranslation('site-blocks')
  return <div className="grid gap-3">
    <img src={fileDownloadUrl(file)} alt="" style={{ width: '100%', aspectRatio: '16/9', objectFit: 'cover', objectPosition: `${x}% ${y}%`, borderRadius: 'var(--radius)' }} />
    <Field label={t('crop.horizontal')}>{(control) => <input {...control} type="range" min={0} max={100} value={x} disabled={disabled} onChange={(event) => onChange(Number(event.target.value), y)} />}</Field>
    <Field label={t('crop.vertical')}>{(control) => <input {...control} type="range" min={0} max={100} value={y} disabled={disabled} onChange={(event) => onChange(x, Number(event.target.value))} />}</Field>
  </div>
}
