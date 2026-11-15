import { FileJson, Upload } from 'lucide-react'
import { useId, useRef, useState, type DragEvent } from 'react'
import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import type { PackageJson } from '@/sdk'
import { readPackageFile, type PackageFileError } from '@/modules/config_packages/ui/lib/package-file'

interface PackageFileDropProps {
  disabled: boolean
  hint: string
  chooseLabel: string
  fileLabel: string
  errorText: (error: PackageFileError) => string
  onChoose: (pkg: PackageJson) => void
}

export function PackageFileDrop({ disabled, hint, chooseLabel, fileLabel, errorText, onChoose }: PackageFileDropProps) {
  const inputId = useId()
  const inputRef = useRef<HTMLInputElement>(null)
  const [dragging, setDragging] = useState(false)
  const [error, setError] = useState<PackageFileError | null>(null)
  const [fileName, setFileName] = useState<string | null>(null)

  const handleFile = async (file: File | undefined) => {
    if (!file) return
    setFileName(file.name)
    setError(null)
    const result = await readPackageFile(file)
    if (!result.ok) {
      setError(result.error)
      return
    }
    onChoose(result.package)
  }

  const drop = (event: DragEvent<HTMLDivElement>) => {
    event.preventDefault()
    setDragging(false)
    if (!disabled) void handleFile(event.dataTransfer.files[0])
  }

  return (
    <div className="grid gap-3">
      <div
        className={cn(
          'flex flex-col items-center gap-3 rounded-lg border border-dashed p-6 text-center transition-colors',
          dragging && 'border-primary bg-accent',
        )}
        onDragOver={(event) => {
          event.preventDefault()
          setDragging(true)
        }}
        onDragLeave={() => setDragging(false)}
        onDrop={drop}
      >
        <FileJson className="size-8 text-muted-foreground" aria-hidden="true" />
        <p className="text-sm text-muted-foreground">{hint}</p>
        <input
          ref={inputRef}
          id={inputId}
          type="file"
          accept=".json,application/json"
          className="sr-only"
          disabled={disabled}
          aria-label={fileLabel}
          aria-invalid={error ? true : undefined}
          aria-describedby={error ? `${inputId}-error` : undefined}
          onChange={(event) => {
            void handleFile(event.target.files?.[0])
            event.target.value = ''
          }}
        />
        <Button type="button" variant="outline" disabled={disabled} onClick={() => inputRef.current?.click()}>
          <Upload aria-hidden="true" />
          {chooseLabel}
        </Button>
        {fileName && <p className="max-w-full truncate text-xs text-muted-foreground">{fileName}</p>}
      </div>
      <div aria-live="assertive">{error && <SharedFieldError id={`${inputId}-error`}>{errorText(error)}</SharedFieldError>}</div>
    </div>
  )
}
