import { render, screen, waitFor } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import TOTPPreview from '../components/TOTPPreview'

vi.mock('../api/client', () => ({ api: { getTotpToken: vi.fn() } }))

import { api } from '../api/client'

const getTotpToken = vi.mocked(api.getTotpToken)

describe('TOTPPreview', () => {
  beforeEach(() => {
    getTotpToken.mockReset()
  })

  it('renders nothing and fetches no token when TOTP is not configured', () => {
    const { container } = render(
      <TOTPPreview entry={{ id: 'entry-1', title: 'Example', username: 'user' }} />,
    )

    expect(container.innerHTML).toBe('')
    expect(getTotpToken).not.toHaveBeenCalled()
  })

  it('shows the generated token and live countdown for a TOTP credential', async () => {
    getTotpToken.mockResolvedValue({ token: '123456', remaining_seconds: 23 })

    render(
      <TOTPPreview entry={{ id: 'entry-1', title: 'Example', username: 'user', has_totp: true }} />,
    )

    await waitFor(() => expect(screen.queryByText('123456')).not.toBeNull())
    expect(screen.queryByText('23s')).not.toBeNull()
    expect(getTotpToken).toHaveBeenCalledWith('entry-1')
  })
})
