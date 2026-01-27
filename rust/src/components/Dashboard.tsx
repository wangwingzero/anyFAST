import { Play, Square, CheckCircle2, Zap, Globe, Link2, Trash2, TrendingUp, TrendingDown, Minus } from 'lucide-react'
import { Endpoint, EndpointResult, Progress } from '../types'

interface DashboardProps {
  endpoints: Endpoint[]
  results: EndpointResult[]
  isRunning: boolean
  progress: Progress
  bindingCount: number
  onStart: () => void
  onStop: () => void
  onApply: (result: EndpointResult) => void
  onApplyAll: () => void
  onClearBindings: () => void
}

export function Dashboard({
  endpoints,
  results,
  isRunning,
  progress,
  bindingCount,
  onStart,
  onStop,
  onApply,
  onApplyAll,
  onClearBindings,
}: DashboardProps) {
  const testedCount = results.length
  const availableCount = results.filter((r) => r.success).length

  return (
    <div className="h-full flex flex-col p-6 overflow-hidden">
      {/* Header */}
      <div className="mb-6">
        <h1 className="text-2xl font-semibold text-apple-gray-600">仪表盘</h1>
        <p className="text-sm text-apple-gray-400 mt-1">测试中转站端点延迟</p>
      </div>

      {/* Status Cards */}
      <div className="grid grid-cols-3 gap-4 mb-6">
        <StatusCard
          icon={<Globe className="w-5 h-5" />}
          label="已测端点"
          value={testedCount}
          color="blue"
        />
        <StatusCard
          icon={<CheckCircle2 className="w-5 h-5" />}
          label="可用端点"
          value={availableCount}
          color="green"
        />
        <StatusCard
          icon={<Link2 className="w-5 h-5" />}
          label="当前绑定"
          value={bindingCount}
          color="orange"
        />
      </div>

      {/* Control Panel */}
      <div className="glass rounded-apple-lg p-4 mb-6 shadow-apple">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm text-apple-gray-500">{progress.message}</p>
            {isRunning && progress.total > 0 && (
              <div className="mt-2 w-64 h-1.5 bg-apple-gray-200 rounded-full overflow-hidden">
                <div
                  className="h-full bg-apple-blue rounded-full transition-all duration-300"
                  style={{ width: `${(progress.current / progress.total) * 100}%` }}
                />
              </div>
            )}
          </div>
          <div className="flex gap-2">
            {!isRunning ? (
              <button
                onClick={onStart}
                disabled={endpoints.length === 0}
                className="flex items-center gap-2 px-4 py-2 bg-apple-blue text-white text-sm font-medium rounded-apple shadow-apple btn-press hover:bg-apple-blue-hover transition-colors disabled:opacity-50"
              >
                <Play className="w-4 h-4" />
                开始测速
              </button>
            ) : (
              <button
                onClick={onStop}
                className="flex items-center gap-2 px-4 py-2 bg-apple-red text-white text-sm font-medium rounded-apple shadow-apple btn-press hover:opacity-90 transition-opacity"
              >
                <Square className="w-4 h-4" />
                停止
              </button>
            )}
          </div>
        </div>
      </div>

      {/* Results Table */}
      <div className="flex-1 glass rounded-apple-lg shadow-apple overflow-hidden flex flex-col">
        <div className="px-4 py-3 border-b border-apple-gray-200 flex items-center justify-between">
          <h2 className="text-sm font-medium text-apple-gray-600">测速结果</h2>
          <span className="text-xs text-apple-gray-400">
            {availableCount}/{testedCount} 可用
          </span>
        </div>

        {/* Table Header */}
        <div className="grid grid-cols-[40px_1fr_1fr_120px_80px_100px_80px] gap-2 px-4 py-2 text-xs text-apple-gray-400 border-b border-apple-gray-100">
          <span>#</span>
          <span>名称</span>
          <span>域名</span>
          <span>IP</span>
          <span>延迟</span>
          <span>加速效果</span>
          <span></span>
        </div>

        {/* Table Body */}
        <div className="flex-1 overflow-y-auto">
          {results.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full text-apple-gray-400">
              <Zap className="w-12 h-12 mb-3 opacity-30" />
              <p className="text-sm">点击"开始测速"测试端点</p>
            </div>
          ) : (
            results.map((result, index) => (
              <ResultRow
                key={result.endpoint.domain}
                rank={index + 1}
                result={result}
                onApply={() => onApply(result)}
              />
            ))
          )}
        </div>
      </div>

      {/* Action Buttons */}
      <div className="flex gap-3 mt-4">
        <button
          onClick={onApplyAll}
          disabled={availableCount === 0}
          className="flex items-center gap-2 px-4 py-2 bg-apple-green text-white text-sm font-medium rounded-apple shadow-apple btn-press hover:opacity-90 transition-opacity disabled:opacity-50"
        >
          <CheckCircle2 className="w-4 h-4" />
          一键全部应用
        </button>
        <button
          onClick={onClearBindings}
          className="flex items-center gap-2 px-4 py-2 bg-apple-gray-200 text-apple-gray-600 text-sm font-medium rounded-apple btn-press hover:bg-apple-gray-300 transition-colors"
        >
          <Trash2 className="w-4 h-4" />
          清除绑定
        </button>
      </div>
    </div>
  )
}

function StatusCard({
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
    <div className="glass rounded-apple-lg p-4 shadow-apple card-hover">
      <div className={`w-10 h-10 rounded-apple flex items-center justify-center ${colorMap[color]} mb-3`}>
        {icon}
      </div>
      <p className="text-xs text-apple-gray-400 mb-1">{label}</p>
      <p className="text-2xl font-semibold text-apple-gray-600">{value}</p>
    </div>
  )
}

function ResultRow({
  rank,
  result,
  onApply,
}: {
  rank: number
  result: EndpointResult
  onApply: () => void
}) {
  const latencyColor = result.success
    ? result.latency < 200
      ? 'text-apple-green'
      : result.latency < 500
        ? 'text-apple-gray-600'
        : result.latency < 1000
          ? 'text-apple-orange'
          : 'text-apple-red'
    : 'text-apple-red'

  // 加速效果显示
  const renderSpeedupBadge = () => {
    if (!result.success) return null

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
      className={`
        grid grid-cols-[40px_1fr_1fr_120px_80px_100px_80px] gap-2 px-4 py-3 items-center
        border-b border-apple-gray-100 last:border-0
        hover:bg-apple-gray-50
        transition-colors
      `}
    >
      <span className="text-sm text-apple-gray-400">{rank}</span>
      <span className="text-sm font-medium text-apple-gray-600 truncate">
        {result.endpoint.name}
      </span>
      <span className="text-sm text-apple-gray-400 font-mono truncate">
        {result.endpoint.domain}
      </span>
      <span className="text-sm text-apple-gray-400 font-mono truncate">
        {result.ip || '-'}
      </span>
      <span className={`text-sm font-medium ${latencyColor}`}>
        {result.success ? `${result.latency.toFixed(0)}ms` : result.error || '失败'}
      </span>
      <div>
        {renderSpeedupBadge()}
      </div>
      <div>
        {result.success && !result.use_original && (
          <button
            onClick={onApply}
            className="px-3 py-1 text-xs font-medium rounded-md btn-press transition-colors bg-apple-blue text-white hover:bg-apple-blue-hover"
          >
            应用
          </button>
        )}
      </div>
    </div>
  )
}
