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

  it('should display usage update message', () => {
    const mockMessage = {
      type: 'usage_update',
      requests_today: 150
    }
    
    vi.mocked(useWebSocketContext).mockReturnValue({
      isConnected: true,
      lastMessage: mockMessage,
      sendMessage: vi.fn(),
      subscribe: vi.fn()
    })

    render(<RealTimeIndicator />)

    expect(screen.getByText('Live Updates')).toBeInTheDocument()
    expect(screen.getByText('API key usage updated (150 today)')).toBeInTheDocument()
  })

  it('should display system stats message', () => {
    const mockMessage = {
      type: 'system_stats',
      total_requests_today: 500
    }
    
    vi.mocked(useWebSocketContext).mockReturnValue({
      isConnected: true,
      lastMessage: mockMessage,
      sendMessage: vi.fn(),
      subscribe: vi.fn()
    })

    render(<RealTimeIndicator />)

    expect(screen.getByText('System stats: 500 requests today')).toBeInTheDocument()
  })

  it('should display success message', () => {
    const mockMessage = {
      type: 'success',
      message: 'Authentication successful'
    }
    
    vi.mocked(useWebSocketContext).mockReturnValue({
      isConnected: true,
      lastMessage: mockMessage,
      sendMessage: vi.fn(),
      subscribe: vi.fn()
    })

    render(<RealTimeIndicator />)

    expect(screen.getByText('Authentication successful')).toBeInTheDocument()
  })

  it('should display error message', () => {
    const mockMessage = {
      type: 'error',
      message: 'Connection failed'
    }
    
    vi.mocked(useWebSocketContext).mockReturnValue({
      isConnected: true,
      lastMessage: mockMessage,
      sendMessage: vi.fn(),
      subscribe: vi.fn()
    })

    render(<RealTimeIndicator />)

    expect(screen.getByText('Error: Connection failed')).toBeInTheDocument()
  })

  it('should display generic update message for unknown types', () => {
    const mockMessage = {
      type: 'unknown_type'
    }
    
    vi.mocked(useWebSocketContext).mockReturnValue({
      isConnected: true,
      lastMessage: mockMessage,
      sendMessage: vi.fn(),
      subscribe: vi.fn()
    })

    render(<RealTimeIndicator />)

    expect(screen.getByText('Real-time update received')).toBeInTheDocument()
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
    expect(container.firstChild).toHaveClass('flex', 'items-center', 'space-x-2')
  })

  it('should not display last update info when no message', () => {
    vi.mocked(useWebSocketContext).mockReturnValue({
      isConnected: true,
      lastMessage: null,
      sendMessage: vi.fn(),
      subscribe: vi.fn()
    })

    render(<RealTimeIndicator />)

    expect(screen.getByText('Live Updates')).toBeInTheDocument()
    
    // Should not have any update info text
    const updateInfo = screen.queryByText(/real-time update/i)
    expect(updateInfo).not.toBeInTheDocument()
  })
})