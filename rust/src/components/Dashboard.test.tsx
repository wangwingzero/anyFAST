import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { Dashboard } from './Dashboard'
import type { Endpoint, EndpointResult, Progress } from '../types'

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
    progress: mockProgress,
    bindingCount: 0,
    onStart: vi.fn(),
    onStop: vi.fn(),
    onApply: vi.fn(),
    onApplyAll: vi.fn(),
    onClearBindings: vi.fn(),
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

  it('shows start button when not running', () => {
    render(<Dashboard {...defaultProps} />)

    expect(screen.getByText('开始测速')).toBeInTheDocument()
    expect(screen.queryByText('停止')).not.toBeInTheDocument()
  })

  it('shows stop button when running', () => {
    render(<Dashboard {...defaultProps} isRunning={true} />)

    expect(screen.getByText('停止')).toBeInTheDocument()
    expect(screen.queryByText('开始测速')).not.toBeInTheDocument()
  })

  it('calls onStart when clicking start button', () => {
    const onStart = vi.fn()
    render(<Dashboard {...defaultProps} onStart={onStart} />)

    fireEvent.click(screen.getByText('开始测速'))
    expect(onStart).toHaveBeenCalledTimes(1)
  })

  it('calls onStop when clicking stop button', () => {
    const onStop = vi.fn()
    render(<Dashboard {...defaultProps} isRunning={true} onStop={onStop} />)

    fireEvent.click(screen.getByText('停止'))
    expect(onStop).toHaveBeenCalledTimes(1)
  })

  it('disables start button when no endpoints', () => {
    render(<Dashboard {...defaultProps} endpoints={[]} />)

    const startButton = screen.getByText('开始测速')
    expect(startButton.closest('button')).toBeDisabled()
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

  it('calls onApplyAll when clicking apply all button', () => {
    const onApplyAll = vi.fn()
    render(<Dashboard {...defaultProps} results={mockResults} onApplyAll={onApplyAll} />)

    fireEvent.click(screen.getByText('一键全部应用'))
    expect(onApplyAll).toHaveBeenCalledTimes(1)
  })

  it('disables apply all when no available results', () => {
    render(<Dashboard {...defaultProps} results={[]} />)

    const applyAllButton = screen.getByText('一键全部应用')
    expect(applyAllButton.closest('button')).toBeDisabled()
  })

  it('calls onClearBindings when clicking clear button', () => {
    const onClearBindings = vi.fn()
    render(<Dashboard {...defaultProps} onClearBindings={onClearBindings} />)

    fireEvent.click(screen.getByText('清除绑定'))
    expect(onClearBindings).toHaveBeenCalledTimes(1)
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
})
