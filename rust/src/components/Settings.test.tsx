import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { Settings } from './Settings'
import type { Endpoint, AppConfig } from '../types'

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

describe('Settings', () => {
  const mockEndpoints: Endpoint[] = [
    { name: 'Test 1', url: 'https://test1.com/v1', domain: 'test1.com', enabled: true },
    { name: 'Test 2', url: 'https://test2.com/v1', domain: 'test2.com', enabled: false },
  ]

  const mockConfig: AppConfig = {
    mode: 'auto',
    check_interval: 30,
    slow_threshold: 50,
    failure_threshold: 3,
    test_count: 3,
    minimize_to_tray: true,
    close_to_tray: true,
    clear_on_exit: false,
    cloudflare_ips: ['1.2.3.4'],
    endpoints: mockEndpoints,
  }

  const defaultProps = {
    endpoints: mockEndpoints,
    config: mockConfig,
    onEndpointsChange: vi.fn(),
    onConfigChange: vi.fn(),
  }

  beforeEach(async () => {
    vi.clearAllMocks()
    const { invoke } = await import('@tauri-apps/api/core')
    vi.mocked(invoke).mockResolvedValue(undefined)
  })

  it('renders header correctly', () => {
    render(<Settings {...defaultProps} />)

    expect(screen.getByText('设置')).toBeInTheDocument()
    expect(screen.getByText('配置运行参数')).toBeInTheDocument()
  })

  it('shows mode selection', () => {
    render(<Settings {...defaultProps} />)

    expect(screen.getByText('手动模式')).toBeInTheDocument()
    expect(screen.getByText('自动模式')).toBeInTheDocument()
  })

  it('shows auto mode options when auto is selected', () => {
    render(<Settings {...defaultProps} />)

    // Auto mode is default, should show additional options
    expect(screen.getByText('检查间隔')).toBeInTheDocument()
    expect(screen.getByText('慢速阈值')).toBeInTheDocument()
    expect(screen.getByText('失败阈值')).toBeInTheDocument()
  })

  it('hides auto mode options when manual is selected', () => {
    const manualConfig = { ...mockConfig, mode: 'manual' as const }
    render(<Settings {...defaultProps} config={manualConfig} />)

    // Select manual mode
    fireEvent.click(screen.getByText('手动模式'))

    // Auto mode options should be hidden
    expect(screen.queryByText('检查间隔')).not.toBeInTheDocument()
  })

  it('shows Cloudflare IPs section', () => {
    render(<Settings {...defaultProps} />)

    expect(screen.getByText('Cloudflare 优选 IP')).toBeInTheDocument()
    expect(screen.getByText(/自定义优选 IP/)).toBeInTheDocument()
  })

  it('shows minimize to tray toggle', () => {
    render(<Settings {...defaultProps} />)

    expect(screen.getByText('最小化时隐藏到托盘')).toBeInTheDocument()
  })

  it('auto-saves when changing mode', async () => {
    const onConfigChange = vi.fn()
    const { invoke } = await import('@tauri-apps/api/core')

    render(<Settings {...defaultProps} onConfigChange={onConfigChange} />)

    fireEvent.click(screen.getByText('手动模式'))

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('save_config', expect.any(Object))
    })
  })

  it('calls onConfigChange after auto-save', async () => {
    const onConfigChange = vi.fn()
    const { invoke } = await import('@tauri-apps/api/core')
    vi.mocked(invoke).mockResolvedValue(undefined)

    render(<Settings {...defaultProps} onConfigChange={onConfigChange} />)

    fireEvent.click(screen.getByText('手动模式'))

    await waitFor(() => {
      expect(onConfigChange).toHaveBeenCalled()
    })
  })

  it('shows restore defaults button', () => {
    render(<Settings {...defaultProps} />)

    expect(screen.getByText('恢复默认值')).toBeInTheDocument()
  })
})
