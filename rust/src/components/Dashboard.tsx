import { useState } from 'react'
import { Play, Square, CheckCircle2, Zap, Globe, Link2, TrendingUp, TrendingDown, Minus, Plus, X, Loader2, Activity } from 'lucide-react'
import { Endpoint, EndpointResult, Progress, EndpointHealth } from '../types'

// WorkingIndicator 组件 Props 接口
// Requirements: 5.1, 5.2, 5.3, 5.4
interface WorkingIndicatorProps {
  isWorking: boolean
  bindingCount: number
}

// WorkingIndicator 组件 - 工作状态指示器
// Requirement 5.1: 工作状态时显示脉冲动画效果
// Requirement 5.3: 停止状态时停止动画并显示静态样式
// Requirement 5.4: 显示当前工作状态文字提示
export function WorkingIndicator({ isWorking, bindingCount }: WorkingIndicatorProps) {
  // 根据 isWorking 状态决定显示内容
  const statusText = isWorking ? '工作中' : '已停止'
  
  // 根据状态设置样式
  // Requirement 5.1: 工作状态时应用脉冲动画 CSS 类
  // Requirement 5.3: 停止状态时移除脉冲动画 CSS 类
  const indicatorStyles = isWorking
    ? 'bg-apple-green/10 border-apple-green/30'
    : 'bg-apple-gray-100 border-apple-gray-200'
  
  const dotStyles = isWorking
    ? 'bg-apple-green working-indicator-pulse'
    : 'bg-apple-gray-400'
  
  const textStyles = isWorking
    ? 'text-apple-green'
    : 'text-apple-gray-500'
  
  const iconStyles = isWorking
    ? 'text-apple-green'
    : 'text-apple-gray-400'

  return (
    <div
      className={`flex items-center gap-2 px-3 py-1.5 rounded-xl border transition-all duration-300 ${indicatorStyles}`}
      data-testid="working-indicator"
      aria-label={`工作状态: ${statusText}`}
    >
      {/* 状态指示点 - 工作时有脉冲动画 */}
      <span
        className={`w-2 h-2 rounded-full transition-all duration-300 ${dotStyles}`}
        data-testid="working-indicator-dot"
      />
      
      {/* 状态图标 */}
      <Activity
        className={`w-4 h-4 transition-colors duration-300 ${iconStyles}`}
        data-testid="working-indicator-icon"
      />
      
      {/* 状态文字 */}
      <span
        className={`text-sm font-medium transition-colors duration-300 ${textStyles}`}
        data-testid="working-indicator-text"
      >
        {statusText}
      </span>
      
      {/* 绑定数量显示 */}
      {bindingCount > 0 && (
        <span
          className="text-xs text-apple-gray-400 ml-1"
          data-testid="working-indicator-binding-count"
        >
          ({bindingCount} 绑定)
        </span>
      )}
    </div>
  )
}

// ToggleButton 组件 Props 接口
interface ToggleButtonProps {
  isWorking: boolean
  isLoading: boolean
  disabled: boolean
  onClick: () => void
}

// ToggleButton 组件 - 启动/停止切换按钮
// Requirements: 2.3, 2.4, 2.5
export function ToggleButton({ isWorking, isLoading, disabled, onClick }: ToggleButtonProps) {
  // 根据 isWorking 状态决定显示内容
  // Requirement 2.4: 停止状态时显示"启动"文字和启动图标
  // Requirement 2.5: 启动状态时显示"停止"文字和停止图标
  const buttonText = isWorking ? '停止' : '启动'
  const ButtonIcon = isWorking ? Square : Play
  
  // 根据状态设置按钮样式
  // 启动状态（isWorking=true）: 红色按钮 + 活跃动画效果
  // 停止状态（isWorking=false）: 绿色按钮
  // Requirement 5.2: 工作状态时显示醒目的活跃状态样式
  const buttonStyles = isWorking
    ? 'bg-apple-red shadow-apple-red/20 hover:opacity-90 toggle-button-active'
    : 'bg-apple-green shadow-apple-green/20 hover:opacity-90'

  return (
    <button
      onClick={onClick}
      disabled={disabled || isLoading}
      className={`flex-1 sm:flex-none flex items-center justify-center gap-2 px-4 py-2 text-white text-sm font-medium rounded-xl shadow-lg btn-press transition-all disabled:opacity-50 disabled:cursor-not-allowed ${buttonStyles}`}
      data-testid="toggle-button"
      aria-label={buttonText}
    >
      {isLoading ? (
        <Loader2 className="w-4 h-4 animate-spin" data-testid="toggle-button-loading" />
      ) : (
        <ButtonIcon className="w-4 h-4" data-testid={isWorking ? 'toggle-button-stop-icon' : 'toggle-button-start-icon'} />
      )}
      <span data-testid="toggle-button-text">{buttonText}</span>
    </button>
  )
}

interface DashboardProps {
  endpoints: Endpoint[]
  results: EndpointResult[]
  isRunning: boolean
  isWorking: boolean  // 工作状态
  progress: Progress
  bindingCount: number
  healthStatus?: EndpointHealth[]
  onApply: (result: EndpointResult) => void
  onToggleWorkflow: () => void  // 切换工作流
  onEndpointsChange?: (endpoints: Endpoint[]) => void
  onSaveConfig?: (endpoints: Endpoint[]) => void
}

export function Dashboard({
  endpoints,
  results,
  isRunning,
  isWorking,
  progress,
  bindingCount,
  healthStatus,
  onApply,
  onToggleWorkflow,
  onEndpointsChange,
  onSaveConfig,
}: DashboardProps) {
  const [showAddForm, setShowAddForm] = useState(false)
  const [newUrl, setNewUrl] = useState('')
  const [newName, setNewName] = useState('')

  const testedCount = results.length
  const availableCount = results.filter((r) => r.success).length
  const enabledEndpoints = endpoints.filter((e) => e.enabled)
  const enabledCount = enabledEndpoints.length

  const addEndpoint = () => {
    if (!newUrl.trim() || !onEndpointsChange) return
    const domain = newUrl.replace(/^https?:\/\//, '').split('/')[0]
    const name = newName.trim() || domain
    const newEndpoint: Endpoint = { name, url: newUrl, domain, enabled: true }
    const newEndpoints = [...endpoints, newEndpoint]
    onEndpointsChange(newEndpoints)
    onSaveConfig?.(newEndpoints)
    setNewUrl('')
    setNewName('')
    setShowAddForm(false)
  }

  const removeEndpoint = (index: number) => {
    if (!onEndpointsChange) return
    const newEndpoints = endpoints.filter((_, i) => i !== index)
    onEndpointsChange(newEndpoints)
    onSaveConfig?.(newEndpoints)
  }

  const toggleEndpoint = (index: number) => {
    if (!onEndpointsChange) return
    const newEndpoints = [...endpoints]
    newEndpoints[index] = { ...newEndpoints[index], enabled: !newEndpoints[index].enabled }
    onEndpointsChange(newEndpoints)
    onSaveConfig?.(newEndpoints)
  }

  return (
    <div className="h-full flex flex-col p-4 lg:p-6 overflow-y-auto">
      {/* Header */}
      <div className="mb-4 lg:mb-6">
        <h1 className="text-xl lg:text-2xl font-semibold text-apple-gray-600">仪表盘</h1>
        <p className="text-sm text-apple-gray-400 mt-1">测试中转站端点延迟</p>
      </div>

      {/* Compact Status Bar */}
      <div className="flex items-center gap-3 mb-4 flex-wrap">
        <CompactStatus icon={<Globe className="w-4 h-4" />} label="已测" value={testedCount} color="blue" />
        <CompactStatus icon={<CheckCircle2 className="w-4 h-4" />} label="可用" value={availableCount} color="green" />
        <CompactStatus icon={<Link2 className="w-4 h-4" />} label="绑定" value={bindingCount} color="orange" />
        
        {/* Requirement 5.4: 在状态栏区域显示当前工作状态文字提示 */}
        <WorkingIndicator isWorking={isWorking} bindingCount={bindingCount} />
      </div>

      {/* Endpoints Management */}
      <div className="bg-white/70 backdrop-blur-sm rounded-2xl p-3 mb-3 shadow-sm border border-gray-100">
        <div className="flex items-center justify-between mb-3">
          <h2 className="text-sm font-medium text-apple-gray-600">端点列表</h2>
          <div className="flex items-center gap-2">
            <span className="text-xs text-apple-gray-400">{enabledCount}/{endpoints.length} 启用</span>
            <button
              onClick={() => setShowAddForm(!showAddForm)}
              className="flex items-center gap-1 px-2 py-1 text-xs font-medium rounded-lg bg-apple-blue/10 text-apple-blue hover:bg-apple-blue/20 transition-colors"
            >
              <Plus className="w-3 h-3" />
              添加
            </button>
          </div>
        </div>

        {/* Add Form */}
        {showAddForm && (
          <div className="flex flex-col sm:flex-row gap-2 mb-3 p-3 bg-apple-gray-50 rounded-xl">
            <input
              type="text"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder="名称（可选）"
              className="w-full sm:w-24 px-2 py-1.5 text-xs bg-white border border-apple-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-apple-blue/30"
            />
            <input
              type="text"
              value={newUrl}
              onChange={(e) => setNewUrl(e.target.value)}
              placeholder="URL (https://example.com/v1)"
              className="flex-1 px-2 py-1.5 text-xs bg-white border border-apple-gray-200 rounded-lg font-mono focus:outline-none focus:ring-2 focus:ring-apple-blue/30"
              onKeyDown={(e) => e.key === 'Enter' && addEndpoint()}
            />
            <div className="flex gap-2">
              <button
                onClick={addEndpoint}
                disabled={!newUrl.trim()}
                className="flex-1 sm:flex-none px-3 py-1.5 text-xs font-medium rounded-lg bg-apple-green text-white hover:opacity-90 transition-opacity disabled:opacity-50"
              >
                确定
              </button>
              <button
                onClick={() => { setShowAddForm(false); setNewUrl(''); setNewName('') }}
                className="flex-1 sm:flex-none px-2 py-1.5 text-xs font-medium rounded-lg bg-apple-gray-200 text-apple-gray-600 hover:bg-apple-gray-300 transition-colors"
              >
                取消
              </button>
            </div>
          </div>
        )}

        {/* Endpoints List */}
        <div className="flex flex-wrap gap-2">
          {endpoints.map((endpoint, index) => (
            <div
              key={index}
              className={`flex items-center gap-2 px-3 py-1.5 rounded-full text-xs transition-colors ${
                endpoint.enabled
                  ? 'bg-apple-blue/10 text-apple-blue'
                  : 'bg-apple-gray-100 text-apple-gray-400'
              }`}
            >
              <button
                onClick={() => toggleEndpoint(index)}
                className="hover:opacity-70 transition-opacity"
                title={endpoint.enabled ? '点击禁用' : '点击启用'}
              >
                <span className={`inline-block w-2 h-2 rounded-full ${endpoint.enabled ? 'bg-apple-green' : 'bg-apple-gray-300'}`} />
              </button>
              <span className="font-medium">{endpoint.name}</span>
              <span className="text-apple-gray-400 font-mono">{endpoint.domain}</span>
              <button
                onClick={() => removeEndpoint(index)}
                className="p-0.5 hover:bg-apple-red/20 hover:text-apple-red rounded transition-colors"
                title="删除端点"
              >
                <X className="w-3 h-3" />
              </button>
            </div>
          ))}
          {endpoints.length === 0 && (
            <p className="text-xs text-apple-gray-400">暂无端点，点击"添加"按钮添加</p>
          )}
        </div>
      </div>

      {/* Control Panel */}
      <div className="bg-white/70 backdrop-blur-sm rounded-2xl p-3 mb-3 shadow-sm border border-gray-100">
        <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3">
          <div className="w-full sm:w-auto">
            <p className="text-sm text-apple-gray-500">{progress.message}</p>
            {isRunning && progress.total > 0 && (
              <div className="mt-2 w-full sm:w-64 h-1.5 bg-apple-gray-200 rounded-full overflow-hidden">
                <div
                  className="h-full bg-apple-blue rounded-full transition-all duration-300"
                  style={{ width: `${(progress.current / progress.total) * 100}%` }}
                />
              </div>
            )}
          </div>
          <div className="flex gap-2 w-full sm:w-auto">
            {/* Requirement 2.3: 单一的启动/停止切换按钮 */}
            <ToggleButton
              isWorking={isWorking}
              isLoading={isRunning}
              disabled={enabledCount === 0}
              onClick={onToggleWorkflow}
            />
          </div>
        </div>
      </div>

      {/* Results Table */}
      <div className="flex-1 bg-white/70 backdrop-blur-sm rounded-2xl shadow-sm border border-gray-100 overflow-hidden flex flex-col min-h-0">
        <div className="px-3 lg:px-4 py-3 border-b border-apple-gray-200 flex items-center justify-between flex-shrink-0">
          <h2 className="text-sm font-medium text-apple-gray-600">测速结果</h2>
          <span className="text-xs text-apple-gray-400">
            {availableCount}/{enabledCount} 可用
          </span>
        </div>

        {/* Table Container with horizontal scroll */}
        <div className="flex-1 overflow-auto min-h-0">
          {/* Table Header */}
          <div className="grid grid-cols-[32px_minmax(60px,1fr)_minmax(80px,1fr)_90px_60px_80px_60px] lg:grid-cols-[40px_1fr_1fr_120px_80px_100px_80px] gap-1 lg:gap-2 px-3 lg:px-4 py-2 text-xs text-apple-gray-400 border-b border-apple-gray-100 sticky top-0 bg-white/90 backdrop-blur-sm">
            <span>#</span>
            <span>名称</span>
            <span>域名</span>
            <span>IP</span>
            <span>延迟</span>
            <span className="hidden sm:block">加速效果</span>
            <span></span>
          </div>

        {/* Table Body */}
        <div className="flex-1">
          {enabledEndpoints.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full min-h-[200px] text-apple-gray-400">
              <Zap className="w-12 h-12 mb-3 opacity-30" />
              <p className="text-sm">请先添加端点</p>
            </div>
          ) : (
            enabledEndpoints.map((endpoint, index) => {
              const result = results.find(r => r.endpoint.domain === endpoint.domain)
              const health = healthStatus?.find(h => h.domain === endpoint.domain)
              return (
                <ResultRow
                  key={endpoint.domain}
                  rank={index + 1}
                  endpoint={endpoint}
                  result={result}
                  health={health}
                  onApply={result ? () => onApply(result) : undefined}
                />
              )
            })
          )}
        </div>
        </div>
      </div>

    </div>
  )
}

function CompactStatus({
  icon,
  label,
  value,
  color,
}: {
  icon: React.ReactNode
  label: string
  value: number
  color: 'blue' | 'green' | 'orange'
}) {
  const colorMap = {
    blue: 'text-apple-blue bg-apple-blue/10',
    green: 'text-apple-green bg-apple-green/10',
    orange: 'text-apple-orange bg-apple-orange/10',
  }

  return (
    <div className="flex items-center gap-2 px-3 py-1.5 bg-white/70 backdrop-blur-sm rounded-xl border border-gray-100">
      <div className={`w-7 h-7 rounded-lg flex items-center justify-center ${colorMap[color]}`}>
        {icon}
      </div>
      <div className="flex items-baseline gap-1">
        <span className="text-lg font-semibold text-apple-gray-600">{value}</span>
        <span className="text-xs text-apple-gray-400">{label}</span>
      </div>
    </div>
  )
}

function ResultRow({
  rank,
  endpoint,
  result,
  health,
  onApply,
}: {
  rank: number
  endpoint: Endpoint
  result?: EndpointResult
  health?: EndpointHealth
  onApply?: () => void
}) {
  // 优先使用健康检查的最优 IP，否则使用测速结果
  const displayIp = health?.best_ip || result?.ip || '-'
  const displayLatency = health?.best_latency || result?.latency
  const isLive = !!health?.best_ip  // 是否有实时数据

  // 未测试状态
  if (!result && !health) {
    return (
      <div
        className="grid grid-cols-[32px_minmax(60px,1fr)_minmax(80px,1fr)_90px_60px_80px_60px] lg:grid-cols-[40px_1fr_1fr_120px_80px_100px_80px] gap-1 lg:gap-2 px-3 lg:px-4 py-2.5 lg:py-3 items-center border-b border-apple-gray-100 last:border-0 bg-apple-gray-50/50"
      >
        <span className="text-xs lg:text-sm text-apple-gray-300">{rank}</span>
        <span className="text-xs lg:text-sm font-medium text-apple-gray-400 truncate">
          {endpoint.name}
        </span>
        <span className="text-xs lg:text-sm text-apple-gray-300 font-mono truncate">
          {endpoint.domain}
        </span>
        <span className="text-xs lg:text-sm text-apple-gray-300">-</span>
        <span className="text-xs lg:text-sm text-apple-gray-300">待测试</span>
        <span className="hidden sm:block text-xs lg:text-sm text-apple-gray-300">-</span>
        <span></span>
      </div>
    )
  }

  const latency = displayLatency || 0
  const latencyColor = latency > 0
    ? latency < 200
      ? 'text-apple-green'
      : latency < 500
        ? 'text-apple-gray-600'
        : latency < 1000
          ? 'text-apple-orange'
          : 'text-apple-red'
    : 'text-apple-red'

  // 加速效果显示
  const renderSpeedupBadge = () => {
    if (!result?.success) return null

    // 如果没有原始延迟数据（旧数据兼容）
    if (!result.original_latency || result.original_latency <= 0) {
      return <span className="text-apple-gray-400 text-xs">-</span>
    }

    if (result.use_original) {
      return (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-apple-gray-200 text-apple-gray-500 text-xs">
          <Minus className="w-3 h-3" />
          原始最优
        </span>
      )
    }

    if (result.speedup_percent > 0) {
      return (
        <span
          className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-apple-green/10 text-apple-green text-xs"
          title={`原始延迟: ${result.original_latency.toFixed(0)}ms → 优化延迟: ${result.latency.toFixed(0)}ms`}
        >
          <TrendingUp className="w-3 h-3" />
          ↑ {result.speedup_percent.toFixed(0)}%
        </span>
      )
    } else {
      return (
        <span
          className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-apple-red/10 text-apple-red text-xs"
          title={`原始延迟: ${result.original_latency.toFixed(0)}ms → 优化延迟: ${result.latency.toFixed(0)}ms`}
        >
          <TrendingDown className="w-3 h-3" />
          ↓ {Math.abs(result.speedup_percent).toFixed(0)}%
        </span>
      )
    }
  }

  return (
    <div
      className="grid grid-cols-[32px_minmax(60px,1fr)_minmax(80px,1fr)_90px_60px_80px_60px] lg:grid-cols-[40px_1fr_1fr_120px_80px_100px_80px] gap-1 lg:gap-2 px-3 lg:px-4 py-2.5 lg:py-3 items-center border-b border-apple-gray-100 last:border-0 hover:bg-apple-gray-50 transition-colors"
    >
      <span className="text-xs lg:text-sm text-apple-gray-400">{rank}</span>
      <span className="text-xs lg:text-sm font-medium text-apple-gray-600 truncate">
        {endpoint.name}
      </span>
      <span className="text-xs lg:text-sm text-apple-gray-400 font-mono truncate">
        {endpoint.domain}
      </span>
      <span className={`text-xs lg:text-sm font-mono truncate ${isLive ? 'text-apple-blue' : 'text-apple-gray-400'}`} title={isLive ? '实时最优 IP' : '测速结果 IP'}>
        {isLive && <span className="inline-block w-1.5 h-1.5 rounded-full bg-apple-green mr-1 animate-pulse" />}
        {displayIp}
      </span>
      <span className={`text-xs lg:text-sm font-medium ${latencyColor}`}>
        {latency > 0 ? `${latency.toFixed(0)}ms` : (result?.error || '失败')}
      </span>
      <div className="hidden sm:block">
        {renderSpeedupBadge()}
      </div>
      <div>
        {result?.success && !result.use_original && onApply && (
          <button
            onClick={onApply}
            className="px-2 lg:px-3 py-1 text-xs font-medium rounded-lg btn-press transition-colors bg-apple-blue text-white hover:bg-apple-blue-hover"
          >
            应用
          </button>
        )}
      </div>
    </div>
  )
}
