import { modulesApi, type ModuleInfo, type ModuleDataExport } from '@/kernel/modules/api'
import { downloadJson } from '@/modules/config_packages/ui/lib/download'

function fileDate(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value.slice(0, 10)
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  return `${date.getFullYear()}-${month}-${day}`
}

export async function exportModuleData(key: string): Promise<ModuleDataExport> {
  const result = await modulesApi.exportData(key)
  downloadJson(`${key}-export-${fileDate(result.exported_at)}.json`, result)
  return result
}

export function exportCoversDisable(module: ModuleInfo, exportedAt: string | null = null): boolean {
  const exported = exportedAt ?? module.data_exported_at
  if (!exported || !module.disabled_at) return false
  return new Date(exported).getTime() >= new Date(module.disabled_at).getTime()
}
