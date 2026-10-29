import { ApiTokensSection } from './sessions/ApiTokensSection'
import { PasswordSection } from './sessions/PasswordSection'
import { SessionsSection } from './sessions/SessionsSection'
import { TwoFactorSection } from './two-factor/TwoFactorSection'

export function SecurityPage() {
  return (
    <div className="flex flex-col gap-8">
      <PasswordSection />
      <TwoFactorSection />
      <SessionsSection />
      <ApiTokensSection />
    </div>
  )
}
