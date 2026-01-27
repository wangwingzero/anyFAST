import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { Sidebar } from './Sidebar'

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

describe('Sidebar', () => {
  const mockOnNavigate = vi.fn()

  beforeEach(async () => {
    mockOnNavigate.mockClear()
    const { invoke } = await import('@tauri-apps/api/core')
    vi.mocked(invoke).mockResolvedValue(true)
  })

  it('renders logo and title', () => {
    render(<Sidebar currentView="dashboard" onNavigate={mockOnNavigate} />)

    expect(screen.getByText('anyFAST')).toBeInTheDocument()
  })

  it('renders all navigation items', () => {
    render(<Sidebar currentView="dashboard" onNavigate={mockOnNavigate} />)

    expect(screen.getByText('仪表盘')).toBeInTheDocument()
    expect(screen.getByText('历史统计')).toBeInTheDocument()
    expect(screen.getByText('运行日志')).toBeInTheDocument()
    expect(screen.getByText('设置')).toBeInTheDocument()
  })

  it('highlights active navigation item', () => {
    render(<Sidebar currentView="dashboard" onNavigate={mockOnNavigate} />)

    const dashboardButton = screen.getByText('仪表盘').closest('button')
    expect(dashboardButton).toHaveClass('bg-apple-blue')
  })

  it('calls onNavigate when clicking dashboard', () => {
    render(<Sidebar currentView="settings" onNavigate={mockOnNavigate} />)

    fireEvent.click(screen.getByText('仪表盘'))
    expect(mockOnNavigate).toHaveBeenCalledWith('dashboard')
  })

  it('calls onNavigate when clicking history', () => {
    render(<Sidebar currentView="dashboard" onNavigate={mockOnNavigate} />)

    fireEvent.click(screen.getByText('历史统计'))
    expect(mockOnNavigate).toHaveBeenCalledWith('history')
  })

  it('calls onNavigate when clicking logs', () => {
    render(<Sidebar currentView="dashboard" onNavigate={mockOnNavigate} />)

    fireEvent.click(screen.getByText('运行日志'))
    expect(mockOnNavigate).toHaveBeenCalledWith('logs')
  })

  it('calls onNavigate when clicking settings', () => {
    render(<Sidebar currentView="dashboard" onNavigate={mockOnNavigate} />)

    fireEvent.click(screen.getByText('设置'))
    expect(mockOnNavigate).toHaveBeenCalledWith('settings')
  })

  it('shows admin status when admin', async () => {
    const { invoke } = await import('@tauri-apps/api/core')
    vi.mocked(invoke).mockResolvedValue(true)

    render(<Sidebar currentView="dashboard" onNavigate={mockOnNavigate} />)

    await waitFor(() => {
      expect(screen.getByText('管理员模式')).toBeInTheDocument()
    })
  })

  it('shows non-admin warning when not admin', async () => {
    const { invoke } = await import('@tauri-apps/api/core')
    vi.mocked(invoke).mockResolvedValue(false)

    render(<Sidebar currentView="dashboard" onNavigate={mockOnNavigate} />)

    await waitFor(() => {
      expect(screen.getByText('需要管理员权限')).toBeInTheDocument()
    })
  })

  it('shows loading state initially', async () => {
    const { invoke } = await import('@tauri-apps/api/core')
    vi.mocked(invoke).mockImplementation(() => new Promise(() => {})) // Never resolves

    render(<Sidebar currentView="dashboard" onNavigate={mockOnNavigate} />)

    expect(screen.getByText('检查权限...')).toBeInTheDocument()
  })

  it('handles error when checking admin status', async () => {
    const { invoke } = await import('@tauri-apps/api/core')
    vi.mocked(invoke).mockRejectedValue(new Error('Permission check failed'))

    render(<Sidebar currentView="dashboard" onNavigate={mockOnNavigate} />)

    await waitFor(() => {
      expect(screen.getByText('需要管理员权限')).toBeInTheDocument()
    })
  })
})
