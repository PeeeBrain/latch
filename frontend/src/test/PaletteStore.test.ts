import { describe, expect, test } from 'vitest'
import {
  createInitialState,
  paletteReducer,
  type PaletteAction,
  type PaletteState,
} from '../components/palette/PaletteStore'
import type { CredentialPreview } from '../api/types'

const entry: CredentialPreview = {
  id: 'entry-1',
  title: 'Example',
  username: 'user',
  url: null,
  icon_url: null,
}

function run(state: PaletteState, ...actions: PaletteAction[]): PaletteState {
  return actions.reduce(paletteReducer, state)
}

describe('paletteReducer', () => {
  test('starts in the requested mode with no credential changes', () => {
    expect(createInitialState('search')).toEqual({
      view: { mode: 'search' },
      credentialsChanged: 0,
    })
  })

  test('GO_TO_SEARCH wipes transient state from any view', () => {
    const generator = run(
      createInitialState('add-entry'),
      { type: 'MODE_CHANGE', mode: 'add-entry', prefillTitle: 'GitHub' },
      { type: 'OPEN_GENERATOR' },
      { type: 'APPLY_GENERATED_PASSWORD', password: 'hunter2' }
    )

    const result = run(generator, { type: 'GO_TO_SEARCH' })

    expect(result.view).toEqual({ mode: 'search' })
  })

  test('MODE_CHANGE resets prior transient state', () => {
    const withPrefill = run(createInitialState('search'), {
      type: 'MODE_CHANGE',
      mode: 'add-entry',
      prefillTitle: 'GitHub',
    })

    const result = run(withPrefill, {
      type: 'MODE_CHANGE',
      mode: 'add-entry',
      prefillTitle: 'GitLab',
    })

    expect(result.view).toEqual({ mode: 'add-entry', prefillTitle: 'GitLab' })
  })

  test('OPEN_GENERATOR remembers edit-entry and returns the password to it', () => {
    const editing = run(createInitialState('search'), {
      type: 'MODE_CHANGE',
      mode: 'edit-entry',
      entry,
    })

    const applying = run(editing, { type: 'OPEN_GENERATOR' })
    expect(applying.view.mode).toBe('password-generator')

    const result = run(applying, {
      type: 'APPLY_GENERATED_PASSWORD',
      password: 'generated-1',
    })

    expect(result.view).toEqual({
      mode: 'edit-entry',
      entry,
      generatedPassword: 'generated-1',
    })
  })

  test('OPEN_GENERATOR keeps add-entry prefill when applying', () => {
    const adding = run(createInitialState('add-entry'), {
      type: 'MODE_CHANGE',
      mode: 'add-entry',
      prefillTitle: 'GitHub',
    })

    const result = run(
      adding,
      { type: 'OPEN_GENERATOR' },
      { type: 'APPLY_GENERATED_PASSWORD', password: 'generated-2' }
    )

    expect(result.view).toEqual({
      mode: 'add-entry',
      prefillTitle: 'GitHub',
      generatedPassword: 'generated-2',
    })
  })

  test('OPEN_GENERATOR from search applies into a new entry', () => {
    const result = run(
      createInitialState('search'),
      { type: 'OPEN_GENERATOR' },
      { type: 'APPLY_GENERATED_PASSWORD', password: 'generated-3' }
    )

    expect(result.view).toEqual({
      mode: 'add-entry',
      prefillTitle: undefined,
      generatedPassword: 'generated-3',
    })
  })

  test('CANCEL_GENERATOR returns to the view that opened it', () => {
    const fromSearch = run(createInitialState('search'), { type: 'OPEN_GENERATOR' })
    expect(run(fromSearch, { type: 'CANCEL_GENERATOR' }).view).toEqual({ mode: 'search' })

    const editing = run(createInitialState('search'), {
      type: 'MODE_CHANGE',
      mode: 'edit-entry',
      entry,
    })
    const fromEdit = run(editing, { type: 'OPEN_GENERATOR' })
    expect(run(fromEdit, { type: 'CANCEL_GENERATOR' }).view).toEqual({
      mode: 'edit-entry',
      entry,
    })
  })

  test('ESCAPE follows the hierarchy back to search', () => {
    const settings = run(createInitialState('search'), { type: 'MODE_CHANGE', mode: 'settings' })
    expect(run(settings, { type: 'ESCAPE' }).view).toEqual({ mode: 'search' })

    const health = run(createInitialState('search'), { type: 'MODE_CHANGE', mode: 'vault-health' })
    expect(run(health, { type: 'ESCAPE' }).view).toEqual({ mode: 'search' })

    for (const mode of ['health-weak', 'health-reused', 'health-breached'] as const) {
      const view = run(createInitialState('search'), { type: 'MODE_CHANGE', mode })
      expect(run(view, { type: 'ESCAPE' }).view).toEqual({ mode: 'vault-health' })
    }

    expect(run(createInitialState('search'), { type: 'ESCAPE' }).view).toEqual({ mode: 'search' })
  })

  test('CREDENTIALS_CHANGED increments the counter and keeps the view', () => {
    const result = run(createInitialState('search'), { type: 'CREDENTIALS_CHANGED' })

    expect(result.credentialsChanged).toBe(1)
    expect(result.view).toEqual({ mode: 'search' })
  })

  test('entry-scoped modes without an entry fall back to search', () => {
    for (const mode of ['actions', 'edit-entry', 'delete-confirm'] as const) {
      const result = run(createInitialState('search'), { type: 'MODE_CHANGE', mode })
      expect(result.view).toEqual({ mode: 'search' })
    }
  })

  test('MODE_CHANGE to password-generator opens the generator', () => {
    const result = run(createInitialState('search'), {
      type: 'MODE_CHANGE',
      mode: 'password-generator',
    })

    expect(result.view).toEqual({
      mode: 'password-generator',
      returnTo: { mode: 'search' },
    })
  })

  test('APPLY_GENERATED_PASSWORD outside the generator is a no-op', () => {
    const result = run(createInitialState('search'), {
      type: 'APPLY_GENERATED_PASSWORD',
      password: 'ignored',
    })

    expect(result.view).toEqual({ mode: 'search' })
  })
})
