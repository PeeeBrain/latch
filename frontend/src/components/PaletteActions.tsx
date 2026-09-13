import { LucideIcon } from 'lucide-react'
import { Key, User, ArrowLeft, LogOut, Edit, Dice1, Shield, ShieldCheck, Trash2 } from 'lucide-react'

export interface Action {
  id: string
  title: string
  subtitle?: string
  icon: LucideIcon
  handler: () => void | Promise<void>
}

export function createUtilityActions(
  onPasswordGenerator: () => void,
  onVaultHealth: () => void
): Action[] {
  return [
    {
      id: 'password-generator',
      title: 'Password Generator',
      subtitle: 'Generate secure passwords',
      icon: Dice1,
      handler: onPasswordGenerator,
    },
    {
      id: 'vault-health',
      title: 'Vault Health',
      subtitle: 'Check password security',
      icon: Shield,
      handler: onVaultHealth,
    },
  ]
}

export interface EntryActionOptions {
  entryTitle: string
  hasTotp: boolean
  onCopyPassword: () => void | Promise<void>
  onCopyUsername: () => void | Promise<void>
  onCopyTotp: () => void | Promise<void>
  onEdit: () => void | Promise<void>
  onLock: () => void
  onBack: () => void
  onDelete: () => void | Promise<void>
}

export function createEntryActions(options: EntryActionOptions): Action[] {
  const { entryTitle, hasTotp } = options

  const copyTotpActions: Action[] = hasTotp
    ? [
        {
          id: 'copy-totp',
          title: 'Copy 2FA Code',
          subtitle: entryTitle,
          icon: ShieldCheck,
          handler: options.onCopyTotp,
        },
      ]
    : []

  return [
    {
      id: 'copy-password',
      title: 'Copy Password',
      subtitle: entryTitle,
      icon: Key,
      handler: () => options.onCopyPassword(),
    },
    {
      id: 'copy-username',
      title: 'Copy Username',
      subtitle: entryTitle,
      icon: User,
      handler: () => options.onCopyUsername(),
    },
    ...copyTotpActions,
    {
      id: 'edit',
      title: 'Edit Entry',
      subtitle: entryTitle,
      icon: Edit,
      handler: () => options.onEdit(),
    },
    {
      id: 'delete',
      title: 'Delete credential',
      subtitle: `"${entryTitle}"`,
      icon: Trash2,
      handler: () => options.onDelete(),
    },
    {
      id: 'back',
      title: 'Back to Search',
      icon: ArrowLeft,
      handler: options.onBack,
    },
    {
      id: 'lock',
      title: 'Lock Vault',
      icon: LogOut,
      handler: options.onLock,
    },
  ]
}



