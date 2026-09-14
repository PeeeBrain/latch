import { render, screen, fireEvent, waitFor, act } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import AddCredential from '../components/modes/AddCredential'

vi.mock('../api/client', () => ({
  api: {
    addEntry: vi.fn(),
    updateEntry: vi.fn(),
    getFullEntry: vi.fn(),
    listAliasConfigs: vi.fn(),
    generateEmailMask: vi.fn(),
  },
}))

import { api } from '../api/client'

const addEntry = vi.mocked(api.addEntry)
const updateEntry = vi.mocked(api.updateEntry)
const getFullEntry = vi.mocked(api.getFullEntry)
const listAliasConfigs = vi.mocked(api.listAliasConfigs)
const generateEmailMask = vi.mocked(api.generateEmailMask)

const TOTP_PLACEHOLDER = '2FA secret or otpauth:// link (optional)...'
const EDIT_TOTP_PLACEHOLDER = 'Edit 2FA secret or otpauth:// link (blank keeps current)...'
const SIMPLELOGIN = { provider_id: 'simplelogin', description: 'Personal' }
const DUCKDUCKGO = { provider_id: 'duckduckgo', description: null }

function renderForm(
  options: {
    onModeChange?: (mode: string) => void
    configs?: unknown[]
    defaultProviderId?: string | null
  } = {},
) {
  listAliasConfigs.mockResolvedValue({
    configs: (options.configs ?? []) as never,
    default_provider_id: options.defaultProviderId ?? null,
  })
  return render(
    <AddCredential
      editEntry={null}
      prefillTitle=""
      generatedPassword=""
      onModeChange={(options.onModeChange ?? vi.fn()) as never}
      onCredentialsChanged={vi.fn()}
    />,
  )
}

beforeEach(() => {
  listAliasConfigs.mockResolvedValue({ configs: [], default_provider_id: null })
})

function renderEditForm() {
  return render(
    <AddCredential
      editEntry={{ id: 'entry-1', title: 'GitHub', username: 'user', has_totp: true }}
      prefillTitle=""
      generatedPassword=""
      onModeChange={vi.fn()}
      onCredentialsChanged={vi.fn()}
    />,
  )
}

async function waitForEditForm() {
  await waitFor(() => expect(getFullEntry).toHaveBeenCalledWith('entry-1'))
  await waitFor(() =>
    expect(
      (screen.getByPlaceholderText('Edit website title...') as HTMLInputElement).value,
    ).toBe('GitHub'),
  )
}

function fillCredential() {
  fireEvent.change(screen.getByPlaceholderText('Website title...'), { target: { value: 'GitHub' } })
  fireEvent.change(screen.getByPlaceholderText('Username or email...'), { target: { value: 'user' } })
  fireEvent.change(screen.getByPlaceholderText('Password...'), { target: { value: 'hunter2' } })
}

describe('AddCredential 2FA secret', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    addEntry.mockResolvedValue('new-id')
  })

  it('saves the entered 2FA secret with a new credential', async () => {
    renderForm()
    fillCredential()
    fireEvent.change(screen.getByPlaceholderText(TOTP_PLACEHOLDER), {
      target: { value: 'JBSWY3DPEHPK3PXP' },
    })

    fireEvent.keyDown(window, { key: 'Enter' })

    await waitFor(() => expect(addEntry).toHaveBeenCalledTimes(1))
    expect(addEntry).toHaveBeenCalledWith(
      expect.objectContaining({ title: 'GitHub', totpSecret: 'JBSWY3DPEHPK3PXP' }),
    )
  })

  it('leaves 2FA unset when the field is blank', async () => {
    renderForm()
    fillCredential()

    fireEvent.keyDown(window, { key: 'Enter' })

    await waitFor(() => expect(addEntry).toHaveBeenCalledTimes(1))
    expect(addEntry).toHaveBeenCalledWith(expect.objectContaining({ totpSecret: undefined }))
  })

  it('prefills title and username from a pasted otpauth URI', () => {
    renderForm()

    const totpInput = screen.getByPlaceholderText(TOTP_PLACEHOLDER)
    fireEvent.change(totpInput, {
      target: {
        value:
          'otpauth://totp/ACME%20Co:john@example.com?secret=JBSWY3DPEHPK3PXP&issuer=ACME%20Co',
      },
    })

    expect((totpInput as HTMLInputElement).value).toBe('JBSWY3DPEHPK3PXP')
    expect((screen.getByPlaceholderText('Website title...') as HTMLInputElement).value).toBe(
      'ACME Co',
    )
    expect((screen.getByPlaceholderText('Username or email...') as HTMLInputElement).value).toBe(
      'john@example.com',
    )
  })

  it('keeps the stored 2FA secret when the edit field is untouched', async () => {
    getFullEntry.mockResolvedValue({
      id: 'entry-1',
      title: 'GitHub',
      username: 'user',
      password: 'hunter2',
    })
    updateEntry.mockResolvedValue()
    renderEditForm()
    await waitForEditForm()

    fireEvent.keyDown(window, { key: 'Enter' })

    await waitFor(() => expect(updateEntry).toHaveBeenCalledTimes(1))
    expect(updateEntry).toHaveBeenCalledWith(expect.objectContaining({ totpSecret: undefined }))
  })

  it('clears the stored 2FA secret when the edit field is emptied', async () => {
    getFullEntry.mockResolvedValue({
      id: 'entry-1',
      title: 'GitHub',
      username: 'user',
      password: 'hunter2',
    })
    updateEntry.mockResolvedValue()
    renderEditForm()
    await waitForEditForm()

    const totpInput = screen.getByPlaceholderText(EDIT_TOTP_PLACEHOLDER)
    fireEvent.change(totpInput, { target: { value: 'JBSWY3DPEHPK3PXP' } })
    fireEvent.change(totpInput, { target: { value: '' } })
    fireEvent.keyDown(window, { key: 'Enter' })

    await waitFor(() => expect(updateEntry).toHaveBeenCalledTimes(1))
    expect(updateEntry).toHaveBeenCalledWith(expect.objectContaining({ totpSecret: '' }))
  })
})

describe('AddCredential email alias masks', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    addEntry.mockResolvedValue('new-id')
    listAliasConfigs.mockResolvedValue({
      configs: [SIMPLELOGIN, DUCKDUCKGO],
      default_provider_id: 'simplelogin',
    })
    generateEmailMask.mockResolvedValue('mask@simplelogin.com')
  })

  const usernameInput = () =>
    screen.getByPlaceholderText('Username or email...') as HTMLInputElement

  it('generates a mask for the default provider and prefills the username', async () => {
    renderForm({ configs: [SIMPLELOGIN, DUCKDUCKGO], defaultProviderId: 'simplelogin' })

    fireEvent.click(await screen.findByRole('button', { name: 'Generate email mask' }))
    fireEvent.click(await screen.findByRole('button', { name: 'Default (SimpleLogin)' }))

    await waitFor(() => expect(generateEmailMask).toHaveBeenCalledWith('simplelogin'))
    await waitFor(() => expect(usernameInput().value).toBe('mask@simplelogin.com'))
  })

  it('generates a mask from a specific provider', async () => {
    generateEmailMask.mockResolvedValue('abc@duck.com')
    renderForm({ configs: [SIMPLELOGIN, DUCKDUCKGO], defaultProviderId: 'simplelogin' })

    fireEvent.click(await screen.findByRole('button', { name: 'Generate email mask' }))
    fireEvent.click(await screen.findByRole('button', { name: 'DuckDuckGo' }))

    await waitFor(() => expect(generateEmailMask).toHaveBeenCalledWith('duckduckgo'))
    await waitFor(() => expect(usernameInput().value).toBe('abc@duck.com'))
  })

  it('saves the alias provider id with the credential', async () => {
    renderForm({ configs: [SIMPLELOGIN], defaultProviderId: 'simplelogin' })

    fireEvent.click(await screen.findByRole('button', { name: 'Generate email mask' }))
    fireEvent.click(await screen.findByRole('button', { name: 'Default (SimpleLogin)' }))
    await waitFor(() => expect(usernameInput().value).toBe('mask@simplelogin.com'))

    fireEvent.change(screen.getByPlaceholderText('Website title...'), { target: { value: 'Acme' } })
    fireEvent.change(screen.getByPlaceholderText('Password...'), { target: { value: 'hunter2' } })
    fireEvent.keyDown(window, { key: 'Enter' })

    await waitFor(() => expect(addEntry).toHaveBeenCalledTimes(1))
    expect(addEntry).toHaveBeenCalledWith(
      expect.objectContaining({ aliasProviderId: 'simplelogin' }),
    )
  })

  it('generates a mask with the default provider on Ctrl+E', async () => {
    renderForm({ configs: [SIMPLELOGIN], defaultProviderId: 'simplelogin' })
    await screen.findByRole('button', { name: 'Generate email mask' })

    fireEvent.keyDown(window, { key: 'e', ctrlKey: true })

    await waitFor(() => expect(generateEmailMask).toHaveBeenCalledWith('simplelogin'))
    await waitFor(() => expect(usernameInput().value).toBe('mask@simplelogin.com'))
  })

  it('guides to settings on Ctrl+E when no default provider exists', async () => {
    const onModeChange = vi.fn()
    renderForm({ configs: [SIMPLELOGIN], defaultProviderId: null, onModeChange })
    await screen.findByRole('button', { name: 'Generate email mask' })

    fireEvent.keyDown(window, { key: 'e', ctrlKey: true })

    expect(await screen.findByText('No default alias provider configured')).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Open Settings' }))
    expect(onModeChange).toHaveBeenCalledWith('settings')
  })

  it('ignores a second generation request while one is pending', async () => {
    let resolveMask: (value: string) => void = () => {}
    generateEmailMask.mockImplementation(
      () =>
        new Promise<string>((resolve) => {
          resolveMask = resolve
        }),
    )
    renderForm({ configs: [SIMPLELOGIN, DUCKDUCKGO], defaultProviderId: 'simplelogin' })

    fireEvent.click(await screen.findByRole('button', { name: 'Generate email mask' }))
    fireEvent.click(await screen.findByRole('button', { name: 'SimpleLogin' }))
    fireEvent.click(screen.getByRole('button', { name: 'DuckDuckGo' }))

    expect(generateEmailMask).toHaveBeenCalledTimes(1)

    await act(async () => resolveMask('mask@simplelogin.com'))
    await waitFor(() => expect(usernameInput().value).toBe('mask@simplelogin.com'))
  })

  it('drops the provider metadata when the username is edited manually', async () => {
    renderForm({ configs: [SIMPLELOGIN], defaultProviderId: 'simplelogin' })

    fireEvent.click(await screen.findByRole('button', { name: 'Generate email mask' }))
    fireEvent.click(await screen.findByRole('button', { name: 'Default (SimpleLogin)' }))
    await waitFor(() => expect(usernameInput().value).toBe('mask@simplelogin.com'))

    fireEvent.change(usernameInput(), { target: { value: 'someone@example.com' } })
    fireEvent.change(screen.getByPlaceholderText('Website title...'), { target: { value: 'Acme' } })
    fireEvent.change(screen.getByPlaceholderText('Password...'), { target: { value: 'hunter2' } })
    fireEvent.keyDown(window, { key: 'Enter' })

    await waitFor(() => expect(addEntry).toHaveBeenCalledTimes(1))
    expect(addEntry).toHaveBeenCalledWith(
      expect.objectContaining({ aliasProviderId: undefined }),
    )
  })

  it('shows generation errors below the username field', async () => {
    generateEmailMask.mockRejectedValue(new Error('Invalid token'))
    renderForm({ configs: [SIMPLELOGIN], defaultProviderId: 'simplelogin' })

    fireEvent.click(await screen.findByRole('button', { name: 'Generate email mask' }))
    fireEvent.click(await screen.findByRole('button', { name: 'Default (SimpleLogin)' }))

    expect(await screen.findByText('Invalid token')).toBeTruthy()
  })
})

