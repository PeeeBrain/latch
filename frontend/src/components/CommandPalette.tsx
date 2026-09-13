import { useReducer, useEffect, useCallback } from 'react'
import { api } from '../api/client'
import { type PaletteMode, type CredentialPreview } from '../api/types'
import { useKeyboardShortcuts } from '../hooks/useKeyboardShortcuts'
import {
  createInitialState,
  paletteReducer,
} from './palette/PaletteStore'
import SearchMode from './modes/SearchMode'
import EntryActions from './modes/EntryActions'
import AddCredential from './modes/AddCredential'
import DeleteConfirm from './modes/DeleteConfirm'
import AuthSelector from './AuthSelector'
import OAuthSignIn from './OAuthSignIn'
import BiometricSignIn from './BiometricSignIn'
import MigrateVault from './MigrateVault'
import Settings from './Settings'
import PasswordGenerator from './PasswordGenerator'
import VaultHealth from './VaultHealth'
import WeakPasswordsList from './health/WeakPasswordsList'
import ReusedPasswordsList from './health/ReusedPasswordsList'
import BreachedCredentialsList from './health/BreachedCredentialsList'

export type { PaletteMode } from '../api/types'

interface CommandPaletteProps {
  initialMode: PaletteMode
}

function CommandPalette({ initialMode }: CommandPaletteProps) {
  const [state, dispatch] = useReducer(paletteReducer, initialMode, createInitialState)
  const { view } = state

  useEffect(() => {
    dispatch({ type: 'MODE_CHANGE', mode: initialMode })
  }, [initialMode])

  const handleCredentialsChanged = useCallback(() => {
    dispatch({ type: 'CREDENTIALS_CHANGED' })
  }, [])

  const handleLock = useCallback(async () => {
    try {
      await api.lockVault()
      const authMethod = await api.getAuthMethod()
      dispatch({
        type: 'MODE_CHANGE',
        mode: authMethod === 'biometric-keychain' ? 'biometric-login' : 'oauth-login',
      })
    } catch (error) {
      console.error('Failed to lock vault:', error)
    }
  }, [])

  const handleModeChange = useCallback(
    (newMode: PaletteMode, entry?: CredentialPreview, title?: string) => {
      dispatch({ type: 'MODE_CHANGE', mode: newMode, entry, prefillTitle: title })
    },
    []
  )

  const handleOAuthSuccess = useCallback(() => {
    dispatch({ type: 'GO_TO_SEARCH' })
  }, [])

  const handleOAuthError = useCallback((_errorMsg: string) => {
  }, [])

  const openEntry = useCallback((entryId: string) => {
    dispatch({
      type: 'MODE_CHANGE',
      mode: 'edit-entry',
      entry: { id: entryId, title: '', username: '' },
    })
  }, [])

  const handleShortcutEscape = useCallback(() => {
    dispatch({ type: 'ESCAPE' })
  }, [])

  const escapeEnabled =
    view.mode === 'settings' ||
    view.mode === 'vault-health' ||
    view.mode === 'health-weak' ||
    view.mode === 'health-reused' ||
    view.mode === 'health-breached'

  useKeyboardShortcuts({
    onEscape: handleShortcutEscape,
    enabled: escapeEnabled,
  })

  const renderMode = () => {
    switch (view.mode) {
      case 'search':
        return (
          <SearchMode
            onModeChange={handleModeChange}
            onLock={handleLock}
            searchTrigger={state.credentialsChanged}
          />
        )

      case 'actions':
        return (
          <EntryActions
            entry={view.entry}
            onModeChange={handleModeChange}
            onLock={handleLock}
          />
        )

      case 'add-entry':
        return (
          <AddCredential
            editEntry={null}
            prefillTitle={view.prefillTitle ?? ''}
            generatedPassword={view.generatedPassword ?? ''}
            onModeChange={handleModeChange}
            onCredentialsChanged={handleCredentialsChanged}
          />
        )

      case 'edit-entry':
        return (
          <AddCredential
            editEntry={view.entry}
            prefillTitle=""
            generatedPassword={view.generatedPassword ?? ''}
            onModeChange={handleModeChange}
            onCredentialsChanged={handleCredentialsChanged}
          />
        )

      case 'delete-confirm':
        return (
          <DeleteConfirm
            entry={view.entry}
            onModeChange={handleModeChange}
            onCredentialsChanged={handleCredentialsChanged}
          />
        )

      case 'auth-selector':
        return (
          <AuthSelector
            onOAuthSelect={() => dispatch({ type: 'MODE_CHANGE', mode: 'oauth-setup' })}
            onBiometricSelect={() => dispatch({ type: 'MODE_CHANGE', mode: 'biometric-setup' })}
          />
        )

      case 'oauth-setup':
        return (
          <OAuthSignIn mode="setup" onSuccess={handleOAuthSuccess} onError={handleOAuthError} />
        )

      case 'oauth-login':
        return (
          <OAuthSignIn mode="login" onSuccess={handleOAuthSuccess} onError={handleOAuthError} />
        )

      case 'biometric-setup':
        return (
          <BiometricSignIn mode="setup" onSuccess={handleOAuthSuccess} onError={handleOAuthError} />
        )

      case 'biometric-login':
        return (
          <BiometricSignIn mode="login" onSuccess={handleOAuthSuccess} onError={handleOAuthError} />
        )

      case 'migrate':
        return <MigrateVault onSuccess={handleOAuthSuccess} onError={handleOAuthError} />

      case 'settings':
        return <Settings />

      case 'password-generator':
        return (
          <PasswordGenerator
            onPasswordSelect={(password) =>
              dispatch({ type: 'APPLY_GENERATED_PASSWORD', password })
            }
            onCancel={() => dispatch({ type: 'CANCEL_GENERATOR' })}
            initialLength={16}
          />
        )

      case 'vault-health':
        return (
          <VaultHealth
            onWeakPasswords={() => dispatch({ type: 'MODE_CHANGE', mode: 'health-weak' })}
            onReusedPasswords={() => dispatch({ type: 'MODE_CHANGE', mode: 'health-reused' })}
            onBreachedCredentials={() => dispatch({ type: 'MODE_CHANGE', mode: 'health-breached' })}
          />
        )

      case 'health-weak':
        return <WeakPasswordsList onSelectEntry={openEntry} />

      case 'health-reused':
        return <ReusedPasswordsList onSelectEntry={openEntry} />

      case 'health-breached':
        return <BreachedCredentialsList onSelectEntry={openEntry} />

      default: {
        const exhaustiveCheck: never = view
        return exhaustiveCheck
      }
    }
  }

  return (
    <div className="w-full h-auto max-h-none bg-theme-bg border-none shadow-none flex flex-col overflow-visible">
      {renderMode()}
    </div>
  )
}

export default CommandPalette
