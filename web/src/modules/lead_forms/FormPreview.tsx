import { useModuleTranslation } from '@/kernel/modules/i18n'
import { LEAD_FORMS_KEY } from './api'
import { parseOptions, type LeadFormValues } from './form'

interface FormPreviewProps {
  values: LeadFormValues
}

const inputClass = 'w-full rounded-md border border-input bg-background px-2.5 py-2 text-sm'

export function FormPreview({ values }: FormPreviewProps) {
  const t = useModuleTranslation(LEAD_FORMS_KEY)
  return (
    <div className="grid gap-3 rounded-md border bg-background p-4" aria-label={t('preview.label')}>
      <p className="text-base font-semibold break-words">{values.name || t('preview.untitled')}</p>
      {values.fields.map((field, index) => {
        const label = `${field.label || t('preview.unnamedField')}${field.required ? ' *' : ''}`
        const id = `lead-form-preview-${index}`
        if (field.kind === 'checkbox') {
          return (
            <label key={id} className="flex items-start gap-2 text-sm">
              <input type="checkbox" className="mt-0.5 size-4" disabled />
              <span className="break-words">{label}</span>
            </label>
          )
        }
        return (
          <div key={id} className="grid gap-1">
            <label htmlFor={id} className="text-sm font-semibold break-words">
              {label}
            </label>
            {field.kind === 'textarea' ? (
              <textarea id={id} className={`${inputClass} min-h-20`} placeholder={field.placeholder} readOnly />
            ) : field.kind === 'select' ? (
              <select id={id} className={inputClass} disabled>
                <option>{field.placeholder || t('preview.selectPrompt')}</option>
                {parseOptions(field.options).map((option) => (
                  <option key={option.value}>{option.label}</option>
                ))}
              </select>
            ) : (
              <input
                id={id}
                className={inputClass}
                type={field.kind === 'email' ? 'email' : field.kind === 'phone' ? 'tel' : 'text'}
                placeholder={field.placeholder}
                readOnly
              />
            )}
          </div>
        )
      })}
      <div>
        <span className="inline-flex rounded-md bg-primary-solid px-4 py-2 text-sm font-semibold text-primary-foreground">
          {values.submit_label || t('preview.submit')}
        </span>
      </div>
      <p className="rounded-md bg-muted px-3 py-2 text-xs text-muted-foreground break-words">
        {t('preview.afterSubmit', {
          message: values.redirect_url
            ? t('preview.redirect', { url: values.redirect_url })
            : values.success_message || t('preview.defaultSuccess'),
        })}
      </p>
    </div>
  )
}
