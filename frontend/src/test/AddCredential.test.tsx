import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import AddCredential from '../components/modes/AddCredential'

vi.mock('../api/client', () => ({
  api: { addEntry: vi.fn(), updateEntry: vi.fn(), getFullEntry: vi.fn() },
}))

import { api } from '../api/client'

const addEntry = vi.mocked(api.addEntry)
const updateEntry = vi.mocked(api.updateEntry)
const getFullEntry = vi.mocked(api.getFullEntry)

const TOTP_PLACEHOLDER = '2FA secret or otpauth:// link (optional)...'
const EDIT_TOTP_PLACEHOLDER = 'Edit 2FA secret or otpauth:// link (blank keeps current)...'

function renderForm() {
  return render(
    <AddCredential
      editEntry={null}
      prefillTitle=""
      generatedPassword=""
      onModeChange={vi.fn()}
      onCredentialsChanged={vi.fn()}
    />,
  )
}

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

