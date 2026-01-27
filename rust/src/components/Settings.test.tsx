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
    expect(screen.getByText('配置端点和运行参数')).toBeInTheDocument()
  })

  it('renders existing endpoints', () => {
    render(<Settings {...defaultProps} />)

    expect(screen.getByText('Test 1')).toBeInTheDocument()
    expect(screen.getByText('Test 2')).toBeInTheDocument()
    expect(screen.getByText('https://test1.com/v1')).toBeInTheDocument()
    expect(screen.getByText('https://test2.com/v1')).toBeInTheDocument()
  })

  it('shows endpoint enabled status', () => {
    render(<Settings {...defaultProps} />)

    const checkboxes = screen.getAllByRole('checkbox')
    // First checkbox should be checked (Test 1 is enabled)
    expect(checkboxes[0]).toBeChecked()
    // Second checkbox should not be checked (Test 2 is disabled)
    expect(checkboxes[1]).not.toBeChecked()
  })

  it('toggles endpoint enabled status', () => {
    const onEndpointsChange = vi.fn()
    render(<Settings {...defaultProps} onEndpointsChange={onEndpointsChange} />)

    const checkboxes = screen.getAllByRole('checkbox')
    fireEvent.click(checkboxes[0])

    expect(onEndpointsChange).toHaveBeenCalled()
    const newEndpoints = onEndpointsChange.mock.calls[0][0]
    expect(newEndpoints[0].enabled).toBe(false)
  })

  it('removes endpoint when clicking delete', async () => {
    const onEndpointsChange = vi.fn()
    render(<Settings {...defaultProps} onEndpointsChange={onEndpointsChange} />)

    // Find all buttons with trash icon class
    const allButtons = screen.getAllByRole('button')
    const deleteButton = allButtons.find(btn =>
      btn.className.includes('text-apple-gray-400') &&
      btn.className.includes('hover:text-apple-red')
    )

    if (deleteButton) {
      fireEvent.click(deleteButton)
      expect(onEndpointsChange).toHaveBeenCalled()
      const newEndpoints = onEndpointsChange.mock.calls[0][0]
      expect(newEndpoints).toHaveLength(1)
    }
  })

  it('adds new endpoint', () => {
    const onEndpointsChange = vi.fn()
    render(<Settings {...defaultProps} onEndpointsChange={onEndpointsChange} />)

    const urlInput = screen.getByPlaceholderText('URL (https://example.com/v1)')
    const nameInput = screen.getByPlaceholderText('名称')
    const addButton = screen.getByText('添加')

    fireEvent.change(nameInput, { target: { value: 'New Endpoint' } })
    fireEvent.change(urlInput, { target: { value: 'https://new.com/api' } })
    fireEvent.click(addButton)

    expect(onEndpointsChange).toHaveBeenCalled()
    const newEndpoints = onEndpointsChange.mock.calls[0][0]
    expect(newEndpoints).toHaveLength(3)
    expect(newEndpoints[2].name).toBe('New Endpoint')
    expect(newEndpoints[2].domain).toBe('new.com')
  })

  it('disables add button when URL is empty', () => {
    render(<Settings {...defaultProps} />)

    const addButton = screen.getByText('添加')
    expect(addButton.closest('button')).toBeDisabled()
  })

  it('enables add button when URL is provided', () => {
    render(<Settings {...defaultProps} />)

    const urlInput = screen.getByPlaceholderText('URL (https://example.com/v1)')
    fireEvent.change(urlInput, { target: { value: 'https://test.com' } })

    const addButton = screen.getByText('添加')
    expect(addButton.closest('button')).not.toBeDisabled()
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

  it('saves settings when clicking save', async () => {
    const onConfigChange = vi.fn()
    const { invoke } = await import('@tauri-apps/api/core')

    render(<Settings {...defaultProps} onConfigChange={onConfigChange} />)

    fireEvent.click(screen.getByText('保存设置'))

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('save_config', expect.any(Object))
    })
  })

  it('calls onConfigChange after successful save', async () => {
    const onConfigChange = vi.fn()
    const { invoke } = await import('@tauri-apps/api/core')
    vi.mocked(invoke).mockResolvedValue(undefined)

    render(<Settings {...defaultProps} onConfigChange={onConfigChange} />)

    fireEvent.click(screen.getByText('保存设置'))

    await waitFor(() => {
      expect(onConfigChange).toHaveBeenCalled()
    })
  })

  it('shows restore defaults button', () => {
    render(<Settings {...defaultProps} />)

    expect(screen.getByText('恢复默认端点')).toBeInTheDocument()
  })

  it('restores default endpoints', () => {
    const onEndpointsChange = vi.fn()
    // Start with empty endpoints
    render(
      <Settings
        {...defaultProps}
        endpoints={[]}
        onEndpointsChange={onEndpointsChange}
      />
    )

    fireEvent.click(screen.getByText('恢复默认端点'))

    expect(onEndpointsChange).toHaveBeenCalled()
    const newEndpoints = onEndpointsChange.mock.calls[0][0]
    expect(newEndpoints.length).toBeGreaterThan(0)
  })

  it('extracts domain from URL correctly', () => {
    const onEndpointsChange = vi.fn()
    render(<Settings {...defaultProps} onEndpointsChange={onEndpointsChange} />)

    const urlInput = screen.getByPlaceholderText('URL (https://example.com/v1)')
    fireEvent.change(urlInput, { target: { value: 'https://my-domain.example.com/api/v1' } })
    fireEvent.click(screen.getByText('添加'))

    const newEndpoints = onEndpointsChange.mock.calls[0][0]
    expect(newEndpoints[2].domain).toBe('my-domain.example.com')
  })
})
