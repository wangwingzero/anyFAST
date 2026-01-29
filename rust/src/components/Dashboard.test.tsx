import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { Dashboard, WorkingIndicator } from './Dashboard'
import type { Endpoint, EndpointResult, Progress } from '../types'

// WorkingIndicator 组件测试
// Requirements: 5.1, 5.2, 5.3, 5.4
describe('WorkingIndicator', () => {
  it('renders with working state correctly', () => {
    render(<WorkingIndicator isWorking={true} bindingCount={3} />)

    // Requirement 5.4: 显示工作状态文字提示
    expect(screen.getByText('工作中')).toBeInTheDocument()
    expect(screen.getByTestId('working-indicator')).toBeInTheDocument()
    expect(screen.getByTestId('working-indicator-dot')).toBeInTheDocument()
    expect(screen.getByTestId('working-indicator-text')).toBeInTheDocument()
  })

  it('renders with stopped state correctly', () => {
    render(<WorkingIndicator isWorking={false} bindingCount={0} />)

    // Requirement 5.4: 显示停止状态文字提示
    expect(screen.getByText('已停止')).toBeInTheDocument()
  })

  it('applies pulse animation class when working', () => {
    render(<WorkingIndicator isWorking={true} bindingCount={0} />)

    // Requirement 5.1: 工作状态时应用脉冲动画 CSS 类
    const dot = screen.getByTestId('working-indicator-dot')
    expect(dot).toHaveClass('working-indicator-pulse')
    expect(dot).toHaveClass('bg-apple-green')
  })

  it('removes pulse animation class when stopped', () => {
    render(<WorkingIndicator isWorking={false} bindingCount={0} />)

    // Requirement 5.3: 停止状态时移除脉冲动画 CSS 类
    const dot = screen.getByTestId('working-indicator-dot')
    expect(dot).not.toHaveClass('working-indicator-pulse')
    expect(dot).toHaveClass('bg-apple-gray-400')
  })

  it('displays binding count when greater than zero', () => {
    render(<WorkingIndicator isWorking={true} bindingCount={5} />)

    expect(screen.getByTestId('working-indicator-binding-count')).toBeInTheDocument()
    expect(screen.getByText('(5 绑定)')).toBeInTheDocument()
  })

  it('hides binding count when zero', () => {
    render(<WorkingIndicator isWorking={true} bindingCount={0} />)

    expect(screen.queryByTestId('working-indicator-binding-count')).not.toBeInTheDocument()
  })

  it('has correct aria-label for accessibility', () => {
    const { rerender } = render(<WorkingIndicator isWorking={true} bindingCount={0} />)
    
    expect(screen.getByTestId('working-indicator')).toHaveAttribute('aria-label', '工作状态: 工作中')

    rerender(<WorkingIndicator isWorking={false} bindingCount={0} />)
    expect(screen.getByTestId('working-indicator')).toHaveAttribute('aria-label', '工作状态: 已停止')
  })

  it('applies correct styles when working', () => {
    render(<WorkingIndicator isWorking={true} bindingCount={0} />)

    const indicator = screen.getByTestId('working-indicator')
    expect(indicator).toHaveClass('bg-apple-green/10')
    expect(indicator).toHaveClass('border-apple-green/30')

    const text = screen.getByTestId('working-indicator-text')
    expect(text).toHaveClass('text-apple-green')
  })

  it('applies correct styles when stopped', () => {
    render(<WorkingIndicator isWorking={false} bindingCount={0} />)

    const indicator = screen.getByTestId('working-indicator')
    expect(indicator).toHaveClass('bg-apple-gray-100')
    expect(indicator).toHaveClass('border-apple-gray-200')

    const text = screen.getByTestId('working-indicator-text')
    expect(text).toHaveClass('text-apple-gray-500')
  })
})

describe('Dashboard', () => {
  const mockEndpoints: Endpoint[] = [
    { name: 'Test 1', url: 'https://test1.com/v1', domain: 'test1.com', enabled: true },
    { name: 'Test 2', url: 'https://test2.com/v1', domain: 'test2.com', enabled: true },
  ]

  const mockResults: EndpointResult[] = [
    {
      endpoint: mockEndpoints[0],
      ip: '1.2.3.4',
      latency: 100,
      ttfb: 100,
      success: true,
      original_ip: '5.6.7.8',
      original_latency: 200,
      speedup_percent: 50,
      use_original: false,
    },
    {
      endpoint: mockEndpoints[1],
      ip: '2.3.4.5',
      latency: 150,
      ttfb: 150,
      success: true,
      original_ip: '6.7.8.9',
      original_latency: 300,
      speedup_percent: 50,
      use_original: false,
    },
  ]

  const mockProgress: Progress = {
    current: 0,
    total: 0,
    message: '就绪',
  }

  const defaultProps = {
    endpoints: mockEndpoints,
    results: [],
    isRunning: false,
    isWorking: false,
    progress: mockProgress,
    bindingCount: 0,
    onApply: vi.fn(),
    onToggleWorkflow: vi.fn(),
  }

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders header correctly', () => {
    render(<Dashboard {...defaultProps} />)

    expect(screen.getByText('仪表盘')).toBeInTheDocument()
    expect(screen.getByText('测试中转站端点延迟')).toBeInTheDocument()
  })

  it('renders status cards with initial values', () => {
    render(<Dashboard {...defaultProps} />)

    // 紧凑布局使用简短标签
    expect(screen.getByText('已测')).toBeInTheDocument()
    expect(screen.getByText('可用')).toBeInTheDocument()
    expect(screen.getByText('绑定')).toBeInTheDocument()
  })

  it('shows empty state when no endpoints', () => {
    render(<Dashboard {...defaultProps} endpoints={[]} />)

    expect(screen.getByText('请先添加端点')).toBeInTheDocument()
  })

  it('shows start button when not working', () => {
    render(<Dashboard {...defaultProps} />)

    // ToggleButton shows "启动" when isWorking is false
    expect(screen.getByText('启动')).toBeInTheDocument()
    expect(screen.queryByText('停止')).not.toBeInTheDocument()
  })

  it('shows stop button when working', () => {
    render(<Dashboard {...defaultProps} isWorking={true} />)

    // ToggleButton shows "停止" when isWorking is true
    expect(screen.getByText('停止')).toBeInTheDocument()
    expect(screen.queryByText('启动')).not.toBeInTheDocument()
  })

  it('calls onToggleWorkflow when clicking toggle button (start)', () => {
    const onToggleWorkflow = vi.fn()
    render(<Dashboard {...defaultProps} onToggleWorkflow={onToggleWorkflow} />)

    fireEvent.click(screen.getByText('启动'))
    expect(onToggleWorkflow).toHaveBeenCalledTimes(1)
  })

  it('calls onToggleWorkflow when clicking toggle button (stop)', () => {
    const onToggleWorkflow = vi.fn()
    render(<Dashboard {...defaultProps} isWorking={true} onToggleWorkflow={onToggleWorkflow} />)

    fireEvent.click(screen.getByText('停止'))
    expect(onToggleWorkflow).toHaveBeenCalledTimes(1)
  })

  it('disables toggle button when no endpoints', () => {
    render(<Dashboard {...defaultProps} endpoints={[]} />)

    const toggleButton = screen.getByText('启动')
    expect(toggleButton.closest('button')).toBeDisabled()
  })

  it('renders results table with data', () => {
    render(<Dashboard {...defaultProps} results={mockResults} />)

    // Test 1 and Test 2 appear in both endpoint list and results table
    expect(screen.getAllByText('Test 1').length).toBeGreaterThanOrEqual(1)
    expect(screen.getAllByText('Test 2').length).toBeGreaterThanOrEqual(1)
    expect(screen.getAllByText('test1.com').length).toBeGreaterThanOrEqual(1)
    expect(screen.getAllByText('test2.com').length).toBeGreaterThanOrEqual(1)
    expect(screen.getByText('100ms')).toBeInTheDocument()
    expect(screen.getByText('150ms')).toBeInTheDocument()
  })

  it('shows apply button for successful results', () => {
    render(<Dashboard {...defaultProps} results={mockResults} />)

    const applyButtons = screen.getAllByText('应用')
    expect(applyButtons).toHaveLength(2)
  })

  it('calls onApply when clicking apply button', () => {
    const onApply = vi.fn()
    render(<Dashboard {...defaultProps} results={mockResults} onApply={onApply} />)

    const applyButtons = screen.getAllByText('应用')
    fireEvent.click(applyButtons[0])
    expect(onApply).toHaveBeenCalledWith(mockResults[0])
  })

  it('shows progress bar when running', () => {
    const progress: Progress = { current: 1, total: 2, message: '正在测试...' }
    render(<Dashboard {...defaultProps} isRunning={true} progress={progress} />)

    expect(screen.getByText('正在测试...')).toBeInTheDocument()
  })

  it('shows speedup percentage for optimized results', () => {
    render(<Dashboard {...defaultProps} results={mockResults} />)

    // Look for speedup indicators
    const speedupBadges = screen.getAllByText(/↑ 50%/)
    expect(speedupBadges).toHaveLength(2)
  })

  it('shows failed result correctly', () => {
    const failedResult: EndpointResult = {
      endpoint: mockEndpoints[0],
      ip: '1.2.3.4',
      latency: 9999,
      ttfb: 9999,
      success: false,
      error: '连接超时',
      original_ip: '',
      original_latency: 0,
      speedup_percent: 0,
      use_original: false,
    }

    render(<Dashboard {...defaultProps} results={[failedResult]} />)

    // 失败时显示错误信息，但不显示应用按钮
    expect(screen.queryByText('应用')).not.toBeInTheDocument()
  })

  it('does not show apply button for use_original results', () => {
    const useOriginalResult: EndpointResult = {
      endpoint: mockEndpoints[0],
      ip: '5.6.7.8',
      latency: 100,
      ttfb: 100,
      success: true,
      original_ip: '5.6.7.8',
      original_latency: 100,
      speedup_percent: 0,
      use_original: true,
    }

    render(<Dashboard {...defaultProps} results={[useOriginalResult]} />)

    expect(screen.getByText('原始最优')).toBeInTheDocument()
    expect(screen.queryByText('应用')).not.toBeInTheDocument()
  })

  it('updates binding count display', () => {
    render(<Dashboard {...defaultProps} bindingCount={5} />)

    // Find the binding count card value
    expect(screen.getByText('5')).toBeInTheDocument()
  })

  // WorkingIndicator integration tests in Dashboard
  // Requirement 5.4: Dashboard 在状态栏区域显示当前工作状态文字提示
  it('renders WorkingIndicator in status bar when not working', () => {
    render(<Dashboard {...defaultProps} isWorking={false} />)

    // WorkingIndicator should be present and show stopped state
    expect(screen.getByTestId('working-indicator')).toBeInTheDocument()
    expect(screen.getByText('已停止')).toBeInTheDocument()
  })

  it('renders WorkingIndicator in status bar when working', () => {
    render(<Dashboard {...defaultProps} isWorking={true} />)

    // WorkingIndicator should be present and show working state
    expect(screen.getByTestId('working-indicator')).toBeInTheDocument()
    expect(screen.getByText('工作中')).toBeInTheDocument()
  })

  it('WorkingIndicator shows binding count in Dashboard', () => {
    render(<Dashboard {...defaultProps} isWorking={true} bindingCount={3} />)

    // WorkingIndicator should show binding count
    expect(screen.getByText('(3 绑定)')).toBeInTheDocument()
  })

  it('WorkingIndicator updates when isWorking changes', () => {
    const { rerender } = render(<Dashboard {...defaultProps} isWorking={false} />)
    
    expect(screen.getByText('已停止')).toBeInTheDocument()
    
    rerender(<Dashboard {...defaultProps} isWorking={true} />)
    
    expect(screen.getByText('工作中')).toBeInTheDocument()
  })
})
