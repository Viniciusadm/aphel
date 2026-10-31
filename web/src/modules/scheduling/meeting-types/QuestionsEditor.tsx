import { Plus, Trash2 } from 'lucide-react'
import { Controller, useFieldArray, type Control, type FieldErrors, type UseFormRegister } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { useContactFields } from '../api'
import { FieldMessage } from '../lib/FieldMessage'
import { QUESTION_KINDS } from '../types'
import { MAX_QUESTIONS, NO_CONTACT_FIELD, emptyQuestion, type MeetingTypeFormValues } from './form'

interface QuestionsEditorProps {
  control: Control<MeetingTypeFormValues>
  register: UseFormRegister<MeetingTypeFormValues>
  errors: FieldErrors<MeetingTypeFormValues>
}

export function QuestionsEditor({ control, register, errors }: QuestionsEditorProps) {
  const { t } = useTranslation('scheduling')
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
                <div className="grid flex-1 gap-2">
                  <Label htmlFor={`question-${index}-label`}>{t('meetingTypes.form.questionLabel')}</Label>
                  <Input placeholder={t('placeholders.question')}
                    id={`question-${index}-label`}
                    aria-invalid={questionErrors?.label ? true : undefined}
                    {...register(`questions.${index}.label`)}
                  />
                </div>
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
                <div className="grid gap-2">
                  <Label htmlFor={`question-${index}-kind`}>{t('meetingTypes.form.questionKind')}</Label>
                  <Controller
                    control={control}
                    name={`questions.${index}.kind`}
                    render={({ field: input }) => (
                      <Select value={input.value} onValueChange={input.onChange}>
                        <SelectTrigger id={`question-${index}-kind`}>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          {QUESTION_KINDS.map((kind) => (
                            <SelectItem key={kind} value={kind}>
                              {t(`questionKinds.${kind}`)}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    )}
                  />
                </div>
                <div className="grid gap-2">
                  <Label htmlFor={`question-${index}-field`}>{t('meetingTypes.form.contactField')}</Label>
                  <Controller
                    control={control}
                    name={`questions.${index}.contactField`}
                    render={({ field: input }) => (
                      <Select value={input.value} onValueChange={input.onChange}>
                        <SelectTrigger id={`question-${index}-field`}>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value={NO_CONTACT_FIELD}>{t('meetingTypes.form.noContactField')}</SelectItem>
                          {(contactFields.data ?? []).map((contactField) => (
                            <SelectItem key={contactField.key} value={contactField.key}>
                              {contactField.label}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    )}
                  />
                </div>
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
