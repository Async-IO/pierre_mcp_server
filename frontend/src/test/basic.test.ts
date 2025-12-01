// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { describe, it, expect } from 'vitest'

describe('Basic Test Setup', () => {
  it('should run a simple test', () => {
    expect(1 + 1).toBe(2)
  })

  it('should have access to global fetch mock', () => {
    expect(global.fetch).toBeDefined()
    expect(typeof global.fetch).toBe('function')
  })

  it('should have access to WebSocket mock', () => {
    expect(global.WebSocket).toBeDefined()
    expect(typeof global.WebSocket).toBe('function')
  })

  it('should have localStorage available', () => {
    localStorage.setItem('test', 'value')
    expect(localStorage.getItem('test')).toBe('value')
    localStorage.removeItem('test')
  })
})