import { useState } from 'react'
import { useClipboardGuard } from '../../hooks/useClipboardGuard'
import { useKeyboardNav } from '../../hooks/useKeyboardNav'
import PaletteList from '../PaletteList'
import { createEntryActions } from '../PaletteActions'
import { type PaletteMode, type CredentialPreview } from '../../api/types'
import { api } from '../../api/client'

interface EntryActionsProps {
  entry: CredentialPreview
  onModeChange: (mode: PaletteMode, entry?: CredentialPreview, prefillTitle?: string) => void
  onLock: () => void
}

function EntryActions({ entry, onModeChange, onLock }: EntryActionsProps) {
  const [selectedIndex, setSelectedIndex] = useState(0)
  const [error, setError] = useState('')
  const { copy } = useClipboardGuard()

  const handleCopySecret = async (load: () => Promise<string>) => {
    try {
      await copy(await load())
      onModeChange('search')
    } catch (err) {
      const errMsg = err instanceof Error ? err.message : String(err)
      if (errMsg.includes('locked')) {
        onModeChange('oauth-login')
      }
      setError(errMsg)
    }
  }

  const handleCopyPassword = () => handleCopySecret(() => api.copyField(entry.id, 'password'))

  const handleCopyTotp = () =>
    handleCopySecret(async () => (await api.getTotpToken(entry.id)).token)

  const handleCopyUsername = async () => {
    if (entry.username) {
      await copy(entry.username)
      onModeChange('search')
    }
  }

  const handleEdit = async () => {
    try {
      const fullEntry = await api.getFullEntry(entry.id)
      onModeChange('edit-entry', {
        id: fullEntry.id,
        title: fullEntry.title,
        username: fullEntry.username,
        icon_url: fullEntry.icon_url,
      } as CredentialPreview, fullEntry.title)
    } catch (error) {
      console.error('Failed to load entry for editing:', error)
    }
  }

  const onBack = () => {
    onModeChange('search')
  }

  const handleDelete = () => {
    onModeChange('delete-confirm', entry)
  }

  const actions = createEntryActions({
    entryTitle: entry.title,
    hasTotp: entry.has_totp ?? false,
    onCopyPassword: handleCopyPassword,
    onCopyUsername: handleCopyUsername,
    onCopyTotp: handleCopyTotp,
    onEdit: handleEdit,
    onLock,
    onBack,
    onDelete: handleDelete,
  })

  const handleEnterKey = () => {
    actions[selectedIndex].handler()
  }

  const handleEscape = () => {
    onModeChange('search')
  }

  useKeyboardNav({
    itemCount: actions.length,
    selectedIndex,
    onSelectedIndexChange: setSelectedIndex,
    onEnter: handleEnterKey,
    onEscape: handleEscape,
    enabled: true,
  })

  return (
    <>
      <PaletteList
        items={actions}
        selectedIndex={selectedIndex}
        onSelect={(_item, index) => {
          setSelectedIndex(index)
          actions[index].handler()
        }}
      />
      {error && <div className="px-3 py-3 bg-theme-danger border-b border-red-900/20 text-theme-text text-sm">{error}</div>}
      <div className="px-3 py-2 border-t-2 border-theme-accent bg-theme-bg flex items-center justify-evenly w-full">
        <span className="text-[11px] text-theme-text-secondary inline-flex items-center gap-[5px] whitespace-nowrap">
          <kbd className="inline-block px-[5px] py-[2px] bg-theme-surface border border-theme-border font-theme text-[10px] font-medium text-theme-text-secondary">↑↓</kbd> Navigate <kbd className="inline-block px-[5px] py-[2px] bg-theme-surface border border-theme-border font-theme text-[10px] font-medium text-theme-text-secondary">Enter</kbd> Execute <kbd className="inline-block px-[5px] py-[2px] bg-theme-surface border border-theme-border font-theme text-[10px] font-medium text-theme-text-secondary">Esc</kbd> Back
        </span>
      </div>
    </>
  )
}

export default EntryActions



