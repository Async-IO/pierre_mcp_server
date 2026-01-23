// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import '@testing-library/jest-dom'

// Mock fetch for API calls
global.fetch = vi.fn()

// Mock IntersectionObserver (not available in jsdom)
class MockIntersectionObserver implements IntersectionObserver {
  root: Element | Document | null = null
  rootMargin: string = ''
  thresholds: ReadonlyArray<number> = []

  constructor(private callback: IntersectionObserverCallback) {
    // Store callback for potential manual triggering in tests
    void this.callback
  }

  observe(target: Element): void {
    // No-op in test environment - parameter intentionally unused
    void target
  }

  unobserve(target: Element): void {
    // No-op in test environment - parameter intentionally unused
    void target
  }

  disconnect(): void {
    // No-op in test environment
  }

  takeRecords(): IntersectionObserverEntry[] {
    return []
  }
}

global.IntersectionObserver = MockIntersectionObserver

// Mock WebSocket
class MockWebSocket {
  url: string
  readyState: number = WebSocket.CONNECTING
  onopen: ((event: Event) => void) | null = null
  onclose: ((event: CloseEvent) => void) | null = null
  onmessage: ((event: MessageEvent) => void) | null = null
  onerror: ((event: Event) => void) | null = null

  constructor(url: string) {
    this.url = url
    // Simulate connection after a tick
    setTimeout(() => {
      this.readyState = WebSocket.OPEN
      if (this.onopen) {
        this.onopen(new Event('open'))
      }
    }, 0)
  }

  send(_data: string) {
    // Mock send - parameter intentionally unused
    void _data;
  }

  close() {
    this.readyState = WebSocket.CLOSED
    if (this.onclose) {
      this.onclose(new CloseEvent('close'))
    }
  }
}

global.WebSocket = MockWebSocket as typeof WebSocket

// Mock Chart.js
vi.mock('chart.js', () => ({
  Chart: {
    register: vi.fn(),
  },
  CategoryScale: vi.fn(),
  LinearScale: vi.fn(),
  PointElement: vi.fn(),
  LineElement: vi.fn(),
  BarElement: vi.fn(),
  Title: vi.fn(),
  Tooltip: vi.fn(),
  Legend: vi.fn(),
  ArcElement: vi.fn(),
}))

vi.mock('react-chartjs-2', () => ({
  Line: vi.fn(() => 'Line Chart'),
  Bar: vi.fn(() => 'Bar Chart'),
  Doughnut: vi.fn(() => 'Doughnut Chart'),
}))