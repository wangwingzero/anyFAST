import { useState } from 'react'
import { CheckCircle2, Zap, Globe, Link2, TrendingUp, TrendingDown, Minus, Plus, X, Loader2, Copy, Check, RefreshCw, Trash2, Link, Unlink } from 'lucide-react'
import { Endpoint, EndpointResult, Progress } from '../types'

// 可复制文本组件
function CopyableText({ text, className }: { text: string; className?: string }) {
  const [copied, setCopied] = useState(false)

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(text)
      setCopied(true)
      setTimeout(() => setCopied(false), 1500)
    } catch (err) {
      console.error('复制失败:', err)
    }
  }

  return (
    <button
      onClick={handleCopy}
      className={`group flex items-center gap-1 cursor-pointer hover:text-apple-blue transition-colors ${className || ''}`}
      title="点击复制"
    >
      <span className="truncate">{text}</span>
      {copied ? (
        <Check className="w-3 h-3 text-apple-green flex-shrink-0" />
      ) : (
        <Copy className="w-3 h-3 opacity-0 group-hover:opacity-50 flex-shrink-0 transition-opacity" />
      )}
    </button>
  )
}

interface DashboardProps {
  endpoints: Endpoint[]
  results: EndpointResult[]
  isRunning: boolean
  progress: Progress
  bindingCount: number
  testingDomains: Set<string>
  onApply: (result: EndpointResult) => void
  onApplyAll: () => void
  onUnbindAll: () => void
  onUnbindEndpoint: (domain: string) => void
  onRetest: () => void
  onTestSingle: (endpoint: Endpoint) => void
  onEndpointsChange?: (endpoints: Endpoint[]) => void
  onSaveConfig?: (endpoints: Endpoint[]) => void
}

export function Dashboard({
  endpoints,
  results,
  isRunning,
  progress: _progress,
  bindingCount,
  testingDomains,
  onApply,
  onApplyAll,
  onUnbindAll,
  onUnbindEndpoint,
  onRetest,
  onTestSingle,
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

  const removeEndpointByDomain = (domain: string) => {
    if (!onEndpointsChange) return
    const newEndpoints = endpoints.filter((e) => e.domain !== domain)
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

      {/* Compact Status Bar + Control */}
      <div className="flex items-center justify-between gap-3 mb-4 flex-wrap">
        <div className="flex items-center gap-3 flex-wrap">
          <CompactStatus icon={<Globe className="w-4 h-4" />} label="已测" value={testedCount} color="blue" />
          <CompactStatus icon={<CheckCircle2 className="w-4 h-4" />} label="可用" value={availableCount} color="green" />
          <CompactStatus icon={<Link2 className="w-4 h-4" />} label="绑定" value={bindingCount} color="orange" />
        </div>

        <div className="flex items-center gap-2">
          {/* 全局测速 */}
          <button
            onClick={onRetest}
            disabled={enabledCount === 0 || isRunning}
            className="flex items-center gap-2 px-3 py-2 text-sm font-medium rounded-xl bg-apple-blue/10 text-apple-blue hover:bg-apple-blue/20 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <RefreshCw className={`w-4 h-4 ${isRunning ? 'animate-spin' : ''}`} />
            测速
          </button>
          {/* 全部绑定 */}
          <button
            onClick={onApplyAll}
            disabled={availableCount === 0 || isRunning}
            className="flex items-center gap-2 px-3 py-2 text-sm font-medium rounded-xl bg-apple-green/10 text-apple-green hover:bg-apple-green/20 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <Link className="w-4 h-4" />
            全部绑定
          </button>
          {/* 全部解绑 */}
          {bindingCount > 0 && (
            <button
              onClick={onUnbindAll}
              disabled={isRunning}
              className="flex items-center gap-2 px-3 py-2 text-sm font-medium rounded-xl bg-apple-red/10 text-apple-red hover:bg-apple-red/20 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <Unlink className="w-4 h-4" />
              全部解绑
            </button>
          )}
        </div>
      </div>

      {/* Results Table */}
      <div className="flex-[3] bg-white/70 backdrop-blur-sm rounded-2xl shadow-sm border border-gray-100 overflow-hidden flex flex-col min-h-[300px] mb-3">
        <div className="px-3 lg:px-4 py-3 border-b border-apple-gray-200 flex items-center justify-between flex-shrink-0">
          <h2 className="text-sm font-medium text-apple-gray-600">测速结果</h2>
          <span className="text-xs text-apple-gray-400">
            {availableCount}/{enabledCount} 可用
          </span>
        </div>

        {/* Table Container with horizontal scroll */}
        <div className="flex-1 overflow-auto min-h-0">
          {/* Table Header */}
          <div className="grid grid-cols-[32px_minmax(60px,1fr)_minmax(80px,1fr)_90px_60px_80px_120px] lg:grid-cols-[40px_1fr_1fr_120px_80px_100px_150px] gap-1 lg:gap-2 px-3 lg:px-4 py-2 text-xs text-apple-gray-400 border-b border-apple-gray-100 sticky top-0 bg-white/90 backdrop-blur-sm">
            <span>#</span>
            <span>名称</span>
            <span>域名</span>
            <span>IP</span>
            <span>延迟</span>
            <span className="hidden sm:block">加速效果</span>
            <span>操作</span>
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
              const isTesting = testingDomains.has(endpoint.domain)
              return (
                <ResultRow
                  key={endpoint.domain}
                  rank={index + 1}
                  endpoint={endpoint}
                  result={result}
                  isTesting={isTesting}
                  onApply={result ? () => onApply(result) : undefined}
                  onUnbind={() => onUnbindEndpoint(endpoint.domain)}
                  onTestSingle={() => onTestSingle(endpoint)}
                  onDelete={onEndpointsChange ? () => removeEndpointByDomain(endpoint.domain) : undefined}
                  bindingCount={bindingCount}
                />
              )
            })
          )}
        </div>
        </div>
      </div>

      {/* Endpoints Management */}
      <div className="flex-1 bg-white/70 backdrop-blur-sm rounded-2xl p-3 shadow-sm border border-gray-100 overflow-auto max-h-[200px]">
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
              <span className="text-apple-gray-400 font-mono">{endpoint.url}</span>
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
  isTesting,
  onApply,
  onUnbind,
  onTestSingle,
  onDelete,
  bindingCount: _bindingCount,
}: {
  rank: number
  endpoint: Endpoint
  result?: EndpointResult
  isTesting?: boolean
  onApply?: () => void
  onUnbind?: () => void
  onTestSingle?: () => void
  onDelete?: () => void
  bindingCount?: number
}) {
  const displayIp = result?.ip || '-'
  const displayLatency = result?.latency
  const showFailure = !!result && !result.success

  // 未测试状态
  if (!result) {
    return (
      <div
        className="grid grid-cols-[32px_minmax(60px,1fr)_minmax(80px,1fr)_90px_60px_80px_120px] lg:grid-cols-[40px_1fr_1fr_120px_80px_100px_150px] gap-1 lg:gap-2 px-3 lg:px-4 py-2.5 lg:py-3 items-center border-b border-apple-gray-100 last:border-0 bg-apple-gray-50/50"
      >
        <span className="text-xs lg:text-sm text-apple-gray-300">{rank}</span>
        <span className="text-xs lg:text-sm font-medium text-apple-gray-400 truncate">
          {endpoint.name}
        </span>
        <CopyableText
          text={endpoint.url}
          className="text-xs lg:text-sm text-apple-gray-300 font-mono"
        />
        <span className="text-xs lg:text-sm text-apple-gray-300">-</span>
        <span className="text-xs lg:text-sm text-apple-gray-300">{isTesting ? '测速中...' : '待测试'}</span>
        <span className="hidden sm:block text-xs lg:text-sm text-apple-gray-300">-</span>
        <div>
          <div className="flex items-center gap-1">
            {onTestSingle && (
              <button
                onClick={onTestSingle}
                disabled={isTesting}
                className="px-2 lg:px-3 py-1 text-xs font-medium rounded-lg btn-press transition-colors bg-apple-blue/10 text-apple-blue hover:bg-apple-blue/20 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-1"
                title="单独测速"
              >
                {isTesting ? (
                  <Loader2 className="w-3 h-3 animate-spin" />
                ) : (
                  <RefreshCw className="w-3 h-3" />
                )}
                <span className="hidden lg:inline">测速</span>
              </button>
            )}
            {onDelete && (
              <button
                onClick={onDelete}
                disabled={isTesting}
                className="px-2 py-1 text-xs font-medium rounded-lg btn-press transition-colors bg-apple-red/10 text-apple-red hover:bg-apple-red/20 disabled:opacity-50 disabled:cursor-not-allowed"
                title="删除站点"
                aria-label={`删除测速站点 ${endpoint.name}`}
              >
                <Trash2 className="w-3 h-3" />
              </button>
            )}
          </div>
        </div>
      </div>
    )
  }

  const latency = displayLatency || 0
  const latencyColor = showFailure
    ? 'text-apple-red'
    : latency > 0
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

    if (!result.original_latency || result.original_latency <= 0) {
      return <span className="text-apple-gray-400 text-xs">-</span>
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
    } else if (result.speedup_percent < 0) {
      return (
        <span
          className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-apple-orange/10 text-apple-orange text-xs"
          title={`原始延迟: ${result.original_latency.toFixed(0)}ms → 当前延迟: ${result.latency.toFixed(0)}ms`}
        >
          <TrendingDown className="w-3 h-3" />
          ↓ {Math.abs(result.speedup_percent).toFixed(0)}%
        </span>
      )
    } else {
      return (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-apple-gray-200 text-apple-gray-500 text-xs">
          <Minus className="w-3 h-3" />
          0%
        </span>
      )
    }
  }

  return (
    <div
      className={`grid grid-cols-[32px_minmax(60px,1fr)_minmax(80px,1fr)_90px_60px_80px_120px] lg:grid-cols-[40px_1fr_1fr_120px_80px_100px_150px] gap-1 lg:gap-2 px-3 lg:px-4 py-2.5 lg:py-3 items-center border-b border-apple-gray-100 last:border-0 hover:bg-apple-gray-50 transition-colors ${isTesting ? 'bg-apple-blue/5' : ''}`}
    >
      <span className="text-xs lg:text-sm text-apple-gray-400">{rank}</span>
      <span className="text-xs lg:text-sm font-medium text-apple-gray-600 truncate">
        {endpoint.name}
      </span>
      <CopyableText
        text={endpoint.url}
        className="text-xs lg:text-sm text-apple-gray-400 font-mono"
      />
      <span className="text-xs lg:text-sm font-mono text-apple-gray-400 truncate">
        {isTesting ? <span className="text-apple-gray-400">测速中...</span> : displayIp}
      </span>
      <span className={`text-xs lg:text-sm font-medium ${isTesting ? 'text-apple-gray-400' : latencyColor}`}>
        {isTesting
          ? <Loader2 className="w-3.5 h-3.5 animate-spin text-apple-blue" />
          : showFailure
            ? (result?.error || '失败')
            : (latency > 0 ? `${latency.toFixed(0)}ms` : '-')}
      </span>
      <div className="hidden sm:block">
        {isTesting ? <span className="text-apple-gray-300 text-xs">-</span> : renderSpeedupBadge()}
      </div>
      <div className="flex items-center gap-1">
        {onTestSingle && (
          <button
            onClick={onTestSingle}
            disabled={isTesting}
            className="px-1.5 lg:px-2 py-1 text-xs font-medium rounded-lg btn-press transition-colors bg-apple-gray-100 text-apple-gray-500 hover:bg-apple-blue/10 hover:text-apple-blue disabled:opacity-50 disabled:cursor-not-allowed"
            title="单独测速"
          >
            {isTesting ? (
              <Loader2 className="w-3 h-3 animate-spin" />
            ) : (
              <RefreshCw className="w-3 h-3" />
            )}
          </button>
        )}
        {result?.success && onApply && !isTesting && (
          <button
            onClick={onApply}
            className="px-1.5 lg:px-2 py-1 text-xs font-medium rounded-lg btn-press transition-colors bg-apple-green/10 text-apple-green hover:bg-apple-green/20"
            title="绑定到 hosts"
          >
            <Link className="w-3 h-3" />
          </button>
        )}
        {onUnbind && !isTesting && (
          <button
            onClick={onUnbind}
            className="px-1.5 lg:px-2 py-1 text-xs font-medium rounded-lg btn-press transition-colors bg-apple-orange/10 text-apple-orange hover:bg-apple-orange/20"
            title="解绑 hosts"
          >
            <Unlink className="w-3 h-3" />
          </button>
        )}
        {onDelete && (
          <button
            onClick={onDelete}
            disabled={isTesting}
            className="px-1.5 lg:px-2 py-1 text-xs font-medium rounded-lg btn-press transition-colors bg-apple-red/10 text-apple-red hover:bg-apple-red/20 disabled:opacity-50 disabled:cursor-not-allowed"
            title="删除站点"
            aria-label={`删除测速站点 ${endpoint.name}`}
          >
            <Trash2 className="w-3 h-3" />
          </button>
        )}
      </div>
    </div>
  )
}
