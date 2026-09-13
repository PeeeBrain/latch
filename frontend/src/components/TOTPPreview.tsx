import { ShieldCheck } from 'lucide-react'
import { useTOTPToken } from '../hooks/useTOTPToken'
import { type CredentialPreview } from '../api/types'

interface TOTPPreviewProps {
  entry: CredentialPreview
}

function TOTPPreview({ entry }: TOTPPreviewProps) {
  const { token, remainingSeconds } = useTOTPToken(entry.has_totp ? entry.id : null)

  if (!entry.has_totp) {
    return null
  }

  return (
    <div className="flex items-center justify-between px-4 py-2 border-t border-theme-border bg-theme-surface/60">
      <span className="inline-flex items-center gap-2 text-[11px] font-medium uppercase tracking-wider text-theme-text-secondary">
        <ShieldCheck size={14} />
        Two-Factor Code
      </span>
      <span className="inline-flex items-center gap-3">
        <span className="font-mono text-[15px] tracking-[0.25em] text-theme-text">
          {token ?? '———'}
        </span>
        <span className="min-w-[34px] text-center text-[11px] font-medium tabular-nums px-[6px] py-[2px] bg-theme-bg border border-theme-border text-theme-text-secondary">
          {remainingSeconds}s
        </span>
      </span>
    </div>
  )
}

export default TOTPPreview
