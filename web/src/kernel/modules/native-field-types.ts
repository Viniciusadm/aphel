import {
  NativeCheckboxInput,
  NativeCurrencyInput,
  NativeDateInput,
  NativeDateTimeInput,
  NativeEmailInput,
  NativeFileInput,
  NativeFormulaInput,
  NativeImageInput,
  NativeLongTextInput,
  NativeMultiSelectInput,
  NativeNumberInput,
  NativePhoneInput,
  NativeRecordInput,
  NativeSelectInput,
  NativeTextInput,
  NativeUrlInput,
  NativeUserInput,
} from '@/components/fields/native-inputs'
import { TextValue } from '@/features/records/display/FieldValueDisplay'
import {
  CheckboxValue,
  EmailValue,
  FileValue,
  LongTextValue,
  MultiSelectValue,
  NumericValue,
  PhoneValue,
  RecordFieldValue,
  SelectValue,
  UrlValue,
  UserFieldValue,
} from '@/features/records/display/native-displays'
import { registerNativeFieldTypes } from './field-types'

export function installNativeFieldTypes() {
  registerNativeFieldTypes({
    text: { Input: NativeTextInput, Display: TextValue },
    long_text: { Input: NativeLongTextInput, Display: LongTextValue },
    number: { Input: NativeNumberInput, Display: NumericValue },
    currency: { Input: NativeCurrencyInput, Display: NumericValue },
    date: { Input: NativeDateInput, Display: TextValue },
    datetime: { Input: NativeDateTimeInput, Display: TextValue },
    select: { Input: NativeSelectInput, Display: SelectValue },
    multi_select: { Input: NativeMultiSelectInput, Display: MultiSelectValue },
    checkbox: { Input: NativeCheckboxInput, Display: CheckboxValue },
    url: { Input: NativeUrlInput, Display: UrlValue },
    email: { Input: NativeEmailInput, Display: EmailValue },
    phone: { Input: NativePhoneInput, Display: PhoneValue },
    user: { Input: NativeUserInput, Display: UserFieldValue },
    record: { Input: NativeRecordInput, Display: RecordFieldValue },
    file: { Input: NativeFileInput, Display: FileValue },
    image: { Input: NativeImageInput, Display: FileValue },
    formula: { Input: NativeFormulaInput },
  })
}
