import { render, screen, fireEvent, waitFor, within } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'

vi.mock('../api/client', () => ({
  api: {
    listAliasConfigs: vi.fn(),
    saveAliasConfig: vi.fn(),
    deleteAliasConfig: vi.fn(),
    setDefaultAliasProvider: vi.fn(),
  },
}))

import { api } from '../api/client'
import AliasSettings from '../components/AliasSettings'

const listAliasConfigs = vi.mocked(api.listAliasConfigs)
const saveAliasConfig = vi.mocked(api.saveAliasConfig)
const deleteAliasConfig = vi.mocked(api.deleteAliasConfig)
const setDefaultAliasProvider = vi.mocked(api.setDefaultAliasProvider)

const SIMPLELOGIN = {
  provider_id: 'simplelogin',
  description: 'Personal aliases',
}
const DUCKDUCKGO = {
  provider_id: 'duckduckgo',
  description: null,
}

beforeEach(() => {
  vi.clearAllMocks()
  listAliasConfigs.mockResolvedValue({ configs: [], default_provider_id: null })
  saveAliasConfig.mockResolvedValue()
  deleteAliasConfig.mockResolvedValue()
  setDefaultAliasProvider.mockResolvedValue()
})

async function renderWith(
  configs: { provider_id: string; description?: string | null }[],
  defaultProviderId: string | null = null,
) {
  listAliasConfigs.mockResolvedValue({ configs, default_provider_id: defaultProviderId })
  render(<AliasSettings />)
  await waitFor(() => expect(listAliasConfigs).toHaveBeenCalled())
  return listAliasConfigs
}

describe('AliasSettings', () => {
  it('lists saved alias providers with their description', async () => {
    await renderWith([SIMPLELOGIN, DUCKDUCKGO])

    const list = within(screen.getByRole('list'))
    expect(list.getByText('Personal aliases')).toBeTruthy()
    expect(list.getByText('SimpleLogin')).toBeTruthy()
    expect(list.getByText('DuckDuckGo')).toBeTruthy()
  })

  it('adds an integration through the form', async () => {
    await renderWith([])

    fireEvent.change(screen.getByLabelText('Provider type'), {
      target: { value: 'duckduckgo' },
    })
    fireEvent.change(screen.getByPlaceholderText('Description (optional)'), {
      target: { value: 'Work' },
    })
    fireEvent.change(screen.getByPlaceholderText('API access token'), {
      target: { value: 'token-123' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Save integration' }))

    await waitFor(() =>
      expect(saveAliasConfig).toHaveBeenCalledWith('duckduckgo', 'token-123', 'Work'),
    )
  })

  it('deletes a saved integration', async () => {
    await renderWith([SIMPLELOGIN])

    fireEvent.click(screen.getByRole('button', { name: 'Delete' }))

    await waitFor(() => expect(deleteAliasConfig).toHaveBeenCalledWith('simplelogin'))
  })

  it('sets a provider as the default', async () => {
    await renderWith([SIMPLELOGIN, DUCKDUCKGO])

    const [makeDefault] = screen.getAllByRole('button', { name: 'Make default' })
    fireEvent.click(makeDefault)

    await waitFor(() =>
      expect(setDefaultAliasProvider).toHaveBeenCalledWith('simplelogin'),
    )
  })

  it('shows the default provider badge and hides its make-default action', async () => {
    await renderWith([SIMPLELOGIN, DUCKDUCKGO], 'duckduckgo')

    expect(screen.getByText('DEFAULT')).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Make default' })).toBeTruthy()
  })

  it('surfaces save errors', async () => {
    await renderWith([])
    saveAliasConfig.mockRejectedValue(new Error('Invalid token'))

    fireEvent.change(screen.getByPlaceholderText('API access token'), {
      target: { value: 'bad' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Save integration' }))

    await waitFor(() => expect(screen.getByText('Invalid token')).toBeTruthy())
  })
})
