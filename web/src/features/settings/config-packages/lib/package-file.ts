import type { PackageJson } from '../api'

export const MAX_PACKAGE_BYTES = 2 * 1024 * 1024

export type PackageFileError = 'tooLarge' | 'invalidJson' | 'notObject'

export type PackageFileResult = { ok: true; package: PackageJson } | { ok: false; error: PackageFileError }

export async function readPackageFile(file: File): Promise<PackageFileResult> {
  if (file.size > MAX_PACKAGE_BYTES) return { ok: false, error: 'tooLarge' }
  let parsed: unknown
  try {
    parsed = JSON.parse(await file.text())
  } catch {
    return { ok: false, error: 'invalidJson' }
  }
  if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) return { ok: false, error: 'notObject' }
  return { ok: true, package: parsed as PackageJson }
}
