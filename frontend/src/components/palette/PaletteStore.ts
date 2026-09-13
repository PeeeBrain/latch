import type { CredentialPreview, PaletteMode } from '../../api/types'

export interface AddEntryView {
  mode: 'add-entry'
  prefillTitle?: string
  generatedPassword?: string
}

export interface EditEntryView {
  mode: 'edit-entry'
  entry: CredentialPreview
  generatedPassword?: string
}

export type PaletteView =
  | { mode: 'search' }
  | { mode: 'actions'; entry: CredentialPreview }
  | AddEntryView
  | EditEntryView
  | { mode: 'delete-confirm'; entry: CredentialPreview }
  | { mode: 'password-generator'; returnTo: PaletteView }
  | { mode: 'auth-selector' }
  | { mode: 'oauth-setup' }
  | { mode: 'oauth-login' }
  | { mode: 'biometric-setup' }
  | { mode: 'biometric-login' }
  | { mode: 'migrate' }
  | { mode: 'settings' }
  | { mode: 'vault-health' }
  | { mode: 'health-weak' }
  | { mode: 'health-reused' }
  | { mode: 'health-breached' }

export interface PaletteState {
  view: PaletteView
  credentialsChanged: number
}

export type PaletteAction =
  | { type: 'MODE_CHANGE'; mode: PaletteMode; entry?: CredentialPreview; prefillTitle?: string }
  | { type: 'OPEN_GENERATOR' }
  | { type: 'APPLY_GENERATED_PASSWORD'; password: string }
  | { type: 'CANCEL_GENERATOR' }
  | { type: 'CREDENTIALS_CHANGED' }
  | { type: 'ESCAPE' }
  | { type: 'GO_TO_SEARCH' }

export function createInitialState(mode: PaletteMode): PaletteState {
  return { view: viewForMode(mode), credentialsChanged: 0 }
}

function viewForMode(
  mode: PaletteMode,
  entry?: CredentialPreview,
  prefillTitle?: string
): PaletteView {
  switch (mode) {
    case 'search':
      return { mode: 'search' }
    case 'add-entry':
      return { mode: 'add-entry', prefillTitle }
    case 'actions':
      return entry ? { mode: 'actions', entry } : { mode: 'search' }
    case 'edit-entry':
      return entry ? { mode: 'edit-entry', entry } : { mode: 'search' }
    case 'delete-confirm':
      return entry ? { mode: 'delete-confirm', entry } : { mode: 'search' }
    case 'password-generator':
      return { mode: 'password-generator', returnTo: { mode: 'search' } }
    case 'auth-selector':
      return { mode: 'auth-selector' }
    case 'oauth-setup':
      return { mode: 'oauth-setup' }
    case 'oauth-login':
      return { mode: 'oauth-login' }
    case 'biometric-setup':
      return { mode: 'biometric-setup' }
    case 'biometric-login':
      return { mode: 'biometric-login' }
    case 'migrate':
      return { mode: 'migrate' }
    case 'settings':
      return { mode: 'settings' }
    case 'vault-health':
      return { mode: 'vault-health' }
    case 'health-weak':
      return { mode: 'health-weak' }
    case 'health-reused':
      return { mode: 'health-reused' }
    case 'health-breached':
      return { mode: 'health-breached' }
  }
}

function openGenerator(state: PaletteState): PaletteState {
  if (state.view.mode === 'password-generator') return state
  return { ...state, view: { mode: 'password-generator', returnTo: state.view } }
}

function applyGeneratedPassword(state: PaletteState, password: string): PaletteState {
  const { view } = state
  if (view.mode !== 'password-generator') return state

  const returnTo = view.returnTo
  if (returnTo.mode === 'edit-entry') {
    return {
      ...state,
      view: { mode: 'edit-entry', entry: returnTo.entry, generatedPassword: password },
    }
  }

  return {
    ...state,
    view: {
      mode: 'add-entry',
      prefillTitle: returnTo.mode === 'add-entry' ? returnTo.prefillTitle : undefined,
      generatedPassword: password,
    },
  }
}

function cancelGenerator(state: PaletteState): PaletteState {
  if (state.view.mode !== 'password-generator') return state
  return { ...state, view: state.view.returnTo }
}

function escape(state: PaletteState): PaletteState {
  switch (state.view.mode) {
    case 'settings':
    case 'vault-health':
      return { ...state, view: { mode: 'search' } }
    case 'health-weak':
    case 'health-reused':
    case 'health-breached':
      return { ...state, view: { mode: 'vault-health' } }
    default:
      return state
  }
}

export function paletteReducer(state: PaletteState, action: PaletteAction): PaletteState {
  switch (action.type) {
    case 'MODE_CHANGE':
      if (action.mode === 'password-generator') return openGenerator(state)
      return { ...state, view: viewForMode(action.mode, action.entry, action.prefillTitle) }
    case 'OPEN_GENERATOR':
      return openGenerator(state)
    case 'APPLY_GENERATED_PASSWORD':
      return applyGeneratedPassword(state, action.password)
    case 'CANCEL_GENERATOR':
      return cancelGenerator(state)
    case 'GO_TO_SEARCH':
      return { ...state, view: { mode: 'search' } }
    case 'CREDENTIALS_CHANGED':
      return { ...state, credentialsChanged: state.credentialsChanged + 1 }
    case 'ESCAPE':
      return escape(state)
    default: {
      const exhaustiveCheck: never = action
      return exhaustiveCheck
    }
  }
}
