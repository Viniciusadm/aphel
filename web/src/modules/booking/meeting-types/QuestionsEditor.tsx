import { NativeSelect } from '@/components/ui/native-select'
import { Plus, Trash2 } from 'lucide-react'
import { Controller, useFieldArray, type Control, type FieldErrors, type UseFormRegister } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { useContactFields } from '../api'
import { FieldMessage } from '../lib/FieldMessage'
import { QUESTION_KINDS } from '../types'
import { MAX_QUESTIONS, NO_CONTACT_FIELD, emptyQuestion, type MeetingTypeFormValues } from './form'
import { Field } from '@/components/ui/field'

interface QuestionsEditorProps {
  control: Control<MeetingTypeFormValues>
  register: UseFormRegister<MeetingTypeFormValues>
  errors: FieldErrors<MeetingTypeFormValues>
}

export function QuestionsEditor({ control, register, errors }: QuestionsEditorProps) {
  const { t } = useTranslation('booking')
  const questions = useFieldArray({ control, name: 'questions' })
  const contactFields = useContactFields()

  return (
    <fieldset className="grid gap-3">
      <legend className="text-sm font-medium">{t('meetingTypes.form.questions')}</legend>
      <p className="text-sm text-muted-foreground">{t('meetingTypes.form.questionsHint')}</p>
      <FieldMessage message={errors.questions?.message ?? errors.questions?.root?.message} />
      {questions.fields.length === 0 && (
        <p className="text-sm text-muted-foreground">{t('meetingTypes.form.noQuestions')}</p>
      )}
      <ul className="grid gap-2">
        {questions.fields.map((field, index) => {
          const questionErrors = errors.questions?.[index]
          return (
            <li key={field.id} className="grid gap-3 rounded-lg border p-3">
              <div className="flex items-end gap-2">
                <Field label={t('meetingTypes.form.questionLabel')} className="flex-1">
                  {(control) => <Input placeholder={t('placeholders.question')}
                    {...control}
                    aria-invalid={questionErrors?.label ? true : undefined}
                    {...register(`questions.${index}.label`)}
                  />}
                </Field>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  aria-label={t('meetingTypes.form.removeQuestion')}
                  onClick={() => questions.remove(index)}
                >
                  <Trash2 />
                </Button>
              </div>
              <FieldMessage message={questionErrors?.label?.message} />
              <div className="grid gap-3 sm:grid-cols-[10rem_1fr_auto] sm:items-end">
                <Field label={t('meetingTypes.form.questionKind')}>
                  {(fieldControl) => <Controller
                    control={control}
                    name={`questions.${index}.kind`}
                    render={({ field: input }) => (
                      <NativeSelect value={input.value} onValueChange={input.onChange} id={fieldControl.id}>
                          {QUESTION_KINDS.map((kind) => (
                            <option key={kind} value={kind}>
                              {t(`questionKinds.${kind}`)}
                            </option>
                          ))}
                        </NativeSelect>
                    )}
                  />}
                </Field>
                <Field label={t('meetingTypes.form.contactField')}>
                  {(fieldControl) => <Controller
                    control={control}
                    name={`questions.${index}.contactField`}
                    render={({ field: input }) => (
                      <NativeSelect value={input.value} onValueChange={input.onChange} id={fieldControl.id}>
                          <option value={NO_CONTACT_FIELD}>{t('meetingTypes.form.noContactField')}</option>
                          {(contactFields.data ?? []).map((contactField) => (
                            <option key={contactField.key} value={contactField.key}>
                              {contactField.label}
                            </option>
                          ))}
                        </NativeSelect>
                    )}
                  />}
                </Field>
                <Controller
                  control={control}
                  name={`questions.${index}.required`}
                  render={({ field: input }) => (
                    <label className="flex h-9 items-center gap-2 text-sm">
                      <Checkbox checked={input.value} onCheckedChange={(checked) => input.onChange(checked === true)} />
                      {t('meetingTypes.form.required')}
                    </label>
                  )}
                />
              </div>
            </li>
          )
        })}
      </ul>
      <div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={questions.fields.length >= MAX_QUESTIONS}
          onClick={() => questions.append(emptyQuestion())}
        >
          <Plus />
          {t('meetingTypes.form.addQuestion')}
        </Button>
      </div>
    </fieldset>
  )
}
