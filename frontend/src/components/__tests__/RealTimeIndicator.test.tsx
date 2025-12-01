// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import RealTimeIndicator from '../RealTimeIndicator'
import { useWebSocketContext } from '../../hooks/useWebSocketContext'

// Mock the useWebSocketContext hook
vi.mock('../../hooks/useWebSocketContext', () => ({
  useWebSocketContext: vi.fn()
}))

describe('RealTimeIndicator Component', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('should show disconnected state when not connected', () => {
    vi.mocked(useWebSocketContext).mockReturnValue({
      isConnected: false,
      lastMessage: null,
      sendMessage: vi.fn(),
      subscribe: vi.fn()
    })

    render(<RealTimeIndicator />)

    expect(screen.getByText('Disconnected')).toBeInTheDocument()

    // Should have red indicator
    const indicator = screen.getByText('Disconnected').previousElementSibling
    expect(indicator).toHaveClass('bg-red-500')
  })

  it('should show connected state when connected', () => {
    vi.mocked(useWebSocketContext).mockReturnValue({
      isConnected: true,
      lastMessage: null,
      sendMessage: vi.fn(),
      subscribe: vi.fn()
    })

    render(<RealTimeIndicator />)

    expect(screen.getByText('Live Updates')).toBeInTheDocument()

    // Should have green indicator with pulse animation
    const indicator = screen.getByText('Live Updates').previousElementSibling
    expect(indicator).toHaveClass('bg-green-500', 'animate-pulse')
  })

  it('should apply custom className', () => {
    vi.mocked(useWebSocketContext).mockReturnValue({
      isConnected: true,
      lastMessage: null,
      sendMessage: vi.fn(),
      subscribe: vi.fn()
    })

    const { container } = render(<RealTimeIndicator className="custom-class" />)

    // The className should be applied to the root div
    expect(container.firstChild).toHaveClass('custom-class')
    expect(container.firstChild).toHaveClass('flex', 'items-center', 'space-x-1')
  })

  it('should show Live Updates when connected regardless of messages', () => {
    vi.mocked(useWebSocketContext).mockReturnValue({
      isConnected: true,
      lastMessage: { type: 'usage_update', requests_today: 150 },
      sendMessage: vi.fn(),
      subscribe: vi.fn()
    })

    render(<RealTimeIndicator />)

    expect(screen.getByText('Live Updates')).toBeInTheDocument()
  })
})
