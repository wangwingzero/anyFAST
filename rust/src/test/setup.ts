import '@testing-library/jest-dom'
import { vi } from 'vitest'

// Mock window.crypto for jsdom
Object.defineProperty(globalThis, 'crypto', {
  value: {
    randomUUID: () => `test-uuid-${Math.random().toString(36).substring(7)}`,
    getRandomValues: (arr: Uint8Array) => {
      for (let i = 0; i < arr.length; i++) {
        arr[i] = Math.floor(Math.random() * 256)
      }
      return arr
    },
  },
})

// Mock ResizeObserver
globalThis.ResizeObserver = vi.fn().mockImplementation(() => ({
  observe: vi.fn(),
  unobserve: vi.fn(),
  disconnect: vi.fn(),
}))

// Mock matchMedia
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
})

// Mock Tauri IPC
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

// Mock Tauri event API - 需要完整 mock 以避免 transformCallback 错误
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockImplementation(() => Promise.resolve(() => {})),
  emit: vi.fn().mockImplementation(() => Promise.resolve()),
  once: vi.fn().mockImplementation(() => Promise.resolve(() => {})),
}))

// Mock Tauri shell plugin
vi.mock('@tauri-apps/plugin-shell', () => ({
  open: vi.fn().mockResolvedValue(undefined),
}))

// Mock Tauri updater plugin
vi.mock('@tauri-apps/plugin-updater', () => ({
  check: vi.fn().mockResolvedValue(null),
}))

// Mock Tauri process plugin
vi.mock('@tauri-apps/plugin-process', () => ({
  relaunch: vi.fn().mockResolvedValue(undefined),
  exit: vi.fn().mockResolvedValue(undefined),
}))

// Mock window.__TAURI_INTERNALS__ for any direct usage
Object.defineProperty(window, '__TAURI_INTERNALS__', {
  value: {
    transformCallback: vi.fn(),
    invoke: vi.fn(),
    metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
  },
  writable: true,
})
