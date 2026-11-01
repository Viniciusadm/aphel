import { Button } from '@/components/ui/button'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { NativeSelect } from '@/components/ui/native-select'
import { Textarea } from '@/components/ui/textarea'
import { ToggleField } from '@/components/ui/toggle-field'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { LEAD_FORMS_KEY } from './api'
import { parseOptions, type LeadFormValues } from './form'

interface FormPreviewProps {
  values: LeadFormValues
}

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
            <ToggleField key={id} id={id} label={label} checked={false} onCheckedChange={() => {}} disabled />
          )
        }
        return (
          <Field key={id} id={id} label={label}>
            {field.kind === 'textarea' ? (
              <Textarea id={id} className="min-h-20" placeholder={field.placeholder} readOnly />
            ) : field.kind === 'select' ? (
              <NativeSelect id={id} disabled>
                <option>{field.placeholder || t('preview.selectPrompt')}</option>
                {parseOptions(field.options).map((option) => (
                  <option key={option.value}>{option.label}</option>
                ))}
              </NativeSelect>
            ) : (
              <Input
                id={id}
                type={field.kind === 'email' ? 'email' : field.kind === 'phone' ? 'tel' : 'text'}
                placeholder={field.placeholder}
                readOnly
              />
            )}
          </Field>
        )
      })}
      <div>
        <Button asChild><span>
          {values.submit_label || t('preview.submit')}
        </span></Button>
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
