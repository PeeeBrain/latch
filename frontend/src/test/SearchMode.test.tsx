import { render, fireEvent, waitFor } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import SearchMode from '../components/modes/SearchMode'
import { type CredentialPreview } from '../api/types'

vi.mock('../api/client', () => ({ api: { getTotpToken: vi.fn() } }))
vi.mock('../hooks/useSearch', () => ({ useSearch: vi.fn() }))

import { api } from '../api/client'
import { useSearch } from '../hooks/useSearch'

const getTotpToken = vi.mocked(api.getTotpToken)
const useSearchMock = vi.mocked(useSearch)

const totpEntry: CredentialPreview = {
  id: 'entry-1',
  title: 'Example',
  username: 'user',
  has_totp: true,
}
const plainEntry: CredentialPreview = { id: 'entry-2', title: 'Plain', username: 'user' }

function mockResults(results: CredentialPreview[]) {
  useSearchMock.mockReturnValue({
    searchResults: results,
    setSearchResults: vi.fn(),
    isLoading: false,
    handleSearch: vi.fn(),
  })
}

describe('SearchMode 2FA shortcut', () => {
  beforeEach(() => {
    getTotpToken.mockReset()
    vi.mocked(navigator.clipboard.writeText).mockReset()
  })

  it('copies the selected credential 2FA code when Ctrl+T is pressed', async () => {
    getTotpToken.mockResolvedValue({ token: '654321', remaining_seconds: 20 })
    mockResults([totpEntry])
    render(<SearchMode onModeChange={vi.fn()} onLock={vi.fn()} searchTrigger={0} />)

    fireEvent.keyDown(window, { key: 't', ctrlKey: true })

    await waitFor(() => expect(navigator.clipboard.writeText).toHaveBeenCalledWith('654321'))
    expect(getTotpToken).toHaveBeenCalledWith('entry-1')
  })

  it('does not fetch a 2FA code for credentials without TOTP', async () => {
    mockResults([plainEntry])
    render(<SearchMode onModeChange={vi.fn()} onLock={vi.fn()} searchTrigger={0} />)

    fireEvent.keyDown(window, { key: 't', ctrlKey: true })
    await new Promise((resolve) => setTimeout(resolve, 0))

    expect(navigator.clipboard.writeText).not.toHaveBeenCalled()
    expect(getTotpToken).not.toHaveBeenCalled()
  })
})
