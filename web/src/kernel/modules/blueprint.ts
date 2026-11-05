import { useQuery } from '@tanstack/react-query'
import { api } from '@/api'

export interface InstalledBlueprint {
  key: string
  name: string
  home: string | null
}

export function useInstalledBlueprint() {
  return useQuery({
    queryKey: ['blueprint'],
    queryFn: async () =>
      (await api.get<{ data: { blueprint: InstalledBlueprint | null } }>('/v1/blueprint')).data.blueprint,
    staleTime: 5 * 60_000,
  })
}
