import type { Team } from '../api'

export function descendantIds(teams: Team[], teamId: string): Set<string> {
  const result = new Set<string>([teamId])
  let added = true
  while (added) {
    added = false
    for (const team of teams) {
      if (team.parent_team_id && result.has(team.parent_team_id) && !result.has(team.id)) {
        result.add(team.id)
        added = true
      }
    }
  }
  return result
}

export function parentOptions(teams: Team[], editing: Team | null): Team[] {
  if (!editing) return teams
  const excluded = descendantIds(teams, editing.id)
  return teams.filter((team) => !excluded.has(team.id))
}
