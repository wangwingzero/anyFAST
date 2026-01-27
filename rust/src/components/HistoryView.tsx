import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import {
  BarChart3,
  Clock,
  TrendingUp,
  Zap,
  RefreshCw,
  Trash2,
  Calendar,
  CheckCircle2,
  XCircle,
} from 'lucide-react'
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from 'recharts'
import { HistoryStats, HistoryRecord } from '../types'

type TimeRange = 1 | 24 | 168 // 1 小时, 24 小时, 7 天 (168 小时)

export function HistoryView() {
  const [stats, setStats] = useState<HistoryStats | null>(null)
  const [loading, setLoading] = useState(false)
  const [timeRange, setTimeRange] = useState<TimeRange>(24)

  const loadStats = useCallback(async () => {
    setLoading(true)
    try {
      const data = await invoke<HistoryStats>('get_history_stats', { hours: timeRange })
      setStats(data)
    } catch (e) {
      console.error('Failed to load history stats:', e)
    } finally {
      setLoading(false)
    }
  }, [timeRange])

  useEffect(() => {
    loadStats()
  }, [loadStats])

  const handleClearHistory = async () => {
    if (!confirm('确定要清除所有历史记录吗？')) return
    try {
      await invoke('clear_history')
      await loadStats()
    } catch (e) {
      console.error('Failed to clear history:', e)
    }
  }

  // 准备图表数据
  const chartData = stats?.records
    ? [...stats.records]
        .reverse()
        .slice(-20)
        .map((r) => ({
          time: new Date(r.timestamp * 1000).toLocaleTimeString('zh-CN', {
            hour: '2-digit',
            minute: '2-digit',
          }),
          original: Math.round(r.original_latency),
          optimized: Math.round(r.optimized_latency),
          speedup: Math.round(r.speedup_percent),
        }))
    : []

  const timeRangeOptions: { value: TimeRange; label: string }[] = [
    { value: 1, label: '最近 1 小时' },
    { value: 24, label: '最近 24 小时' },
    { value: 168, label: '最近 7 天' },
  ]

  return (
    <div className="h-full flex flex-col p-6 overflow-hidden">
      {/* Header */}
      <div className="mb-6 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold text-apple-gray-600">历史统计</h1>
          <p className="text-sm text-apple-gray-400 mt-1">查看测速历史和加速效果统计</p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={loadStats}
            disabled={loading}
            className="flex items-center gap-2 px-3 py-1.5 bg-apple-gray-200 text-apple-gray-600 text-sm font-medium rounded-apple btn-press hover:bg-apple-gray-300 transition-colors disabled:opacity-50"
          >
            <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
            刷新
          </button>
          <button
            onClick={handleClearHistory}
            className="flex items-center gap-2 px-3 py-1.5 bg-apple-red/10 text-apple-red text-sm font-medium rounded-apple btn-press hover:bg-apple-red/20 transition-colors"
          >
            <Trash2 className="w-4 h-4" />
            清除
          </button>
        </div>
      </div>

      {/* Time Range Selector */}
      <div className="mb-6">
        <div className="inline-flex bg-apple-gray-200 rounded-apple p-1">
          {timeRangeOptions.map((option) => (
            <button
              key={option.value}
              onClick={() => setTimeRange(option.value)}
              className={`px-4 py-1.5 text-sm font-medium rounded-apple transition-all ${
                timeRange === option.value
                  ? 'bg-white text-apple-gray-600 shadow-sm'
                  : 'text-apple-gray-500 hover:text-apple-gray-600'
              }`}
            >
              {option.label}
            </button>
          ))}
        </div>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-3 gap-4 mb-6">
        <StatCard
          icon={<BarChart3 className="w-5 h-5" />}
          label="总测试次数"
          value={stats?.total_tests ?? 0}
          color="blue"
        />
        <StatCard
          icon={<Clock className="w-5 h-5" />}
          label="累计节省时间"
          value={stats?.total_speedup_ms ? `${(stats.total_speedup_ms / 1000).toFixed(1)}s` : '0s'}
          color="green"
        />
        <StatCard
          icon={<TrendingUp className="w-5 h-5" />}
          label="平均加速"
          value={stats?.avg_speedup_percent ? `${stats.avg_speedup_percent.toFixed(1)}%` : '0%'}
          color="orange"
        />
      </div>

      {/* Chart */}
      <div className="glass rounded-apple-lg p-4 mb-6 shadow-apple">
        <h2 className="text-sm font-medium text-apple-gray-600 mb-4">延迟趋势</h2>
        {chartData.length > 0 ? (
          <div className="h-48">
            <ResponsiveContainer width="100%" height="100%">
              <LineChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#e5e5e5" />
                <XAxis
                  dataKey="time"
                  tick={{ fontSize: 11, fill: '#8e8e93' }}
                  axisLine={{ stroke: '#e5e5e5' }}
                />
                <YAxis
                  tick={{ fontSize: 11, fill: '#8e8e93' }}
                  axisLine={{ stroke: '#e5e5e5' }}
                  unit="ms"
                />
                <Tooltip
                  contentStyle={{
                    backgroundColor: 'rgba(255, 255, 255, 0.95)',
                    border: 'none',
                    borderRadius: '8px',
                    boxShadow: '0 2px 10px rgba(0, 0, 0, 0.1)',
                  }}
                  labelStyle={{ color: '#1d1d1f', fontWeight: 500 }}
                />
                <Legend wrapperStyle={{ fontSize: 12 }} />
                <Line
                  type="monotone"
                  dataKey="original"
                  name="原始延迟"
                  stroke="#ff9500"
                  strokeWidth={2}
                  dot={{ r: 3 }}
                  activeDot={{ r: 5 }}
                />
                <Line
                  type="monotone"
                  dataKey="optimized"
                  name="优化延迟"
                  stroke="#34c759"
                  strokeWidth={2}
                  dot={{ r: 3 }}
                  activeDot={{ r: 5 }}
                />
              </LineChart>
            </ResponsiveContainer>
          </div>
        ) : (
          <div className="h-48 flex items-center justify-center text-apple-gray-400">
            <div className="text-center">
              <Zap className="w-10 h-10 mx-auto mb-2 opacity-30" />
              <p className="text-sm">暂无数据</p>
            </div>
          </div>
        )}
      </div>

      {/* Recent Records Table */}
      <div className="flex-1 glass rounded-apple-lg shadow-apple overflow-hidden flex flex-col">
        <div className="px-4 py-3 border-b border-apple-gray-200 flex items-center justify-between">
          <h2 className="text-sm font-medium text-apple-gray-600">最近记录</h2>
          <span className="text-xs text-apple-gray-400">
            共 {stats?.records.length ?? 0} 条
          </span>
        </div>

        {/* Table Header */}
        <div className="grid grid-cols-[150px_1fr_100px_100px_100px_80px] gap-2 px-4 py-2 text-xs text-apple-gray-400 border-b border-apple-gray-100">
          <span>时间</span>
          <span>域名</span>
          <span>原始延迟</span>
          <span>优化延迟</span>
          <span>加速效果</span>
          <span>状态</span>
        </div>

        {/* Table Body */}
        <div className="flex-1 overflow-y-auto">
          {!stats?.records.length ? (
            <div className="flex flex-col items-center justify-center h-full text-apple-gray-400">
              <Calendar className="w-12 h-12 mb-3 opacity-30" />
              <p className="text-sm">暂无历史记录</p>
            </div>
          ) : (
            stats.records.map((record, index) => (
              <RecordRow key={`${record.timestamp}-${index}`} record={record} />
            ))
          )}
        </div>
      </div>
    </div>
  )
}

function StatCard({
  icon,
  label,
  value,
  color,
}: {
  icon: React.ReactNode
  label: string
  value: string | number
  color: 'blue' | 'green' | 'orange'
}) {
  const colorMap = {
    blue: 'text-apple-blue bg-apple-blue/10',
    green: 'text-apple-green bg-apple-green/10',
    orange: 'text-apple-orange bg-apple-orange/10',
  }

  return (
    <div className="glass rounded-apple-lg p-4 shadow-apple card-hover">
      <div
        className={`w-10 h-10 rounded-apple flex items-center justify-center ${colorMap[color]} mb-3`}
      >
        {icon}
      </div>
      <p className="text-xs text-apple-gray-400 mb-1">{label}</p>
      <p className="text-2xl font-semibold text-apple-gray-600">{value}</p>
    </div>
  )
}

function RecordRow({ record }: { record: HistoryRecord }) {
  const time = new Date(record.timestamp * 1000).toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })

  const speedupColor =
    record.speedup_percent > 0
      ? 'text-apple-green'
      : record.speedup_percent < 0
        ? 'text-apple-red'
        : 'text-apple-gray-400'

  return (
    <div className="grid grid-cols-[150px_1fr_100px_100px_100px_80px] gap-2 px-4 py-2.5 items-center border-b border-apple-gray-100 last:border-0 hover:bg-apple-gray-50 transition-colors">
      <span className="text-sm text-apple-gray-400">{time}</span>
      <span className="text-sm text-apple-gray-600 font-mono truncate">{record.domain}</span>
      <span className="text-sm text-apple-orange">{record.original_latency.toFixed(0)}ms</span>
      <span className="text-sm text-apple-green">{record.optimized_latency.toFixed(0)}ms</span>
      <span className={`text-sm font-medium ${speedupColor}`}>
        {record.speedup_percent > 0 ? '+' : ''}
        {record.speedup_percent.toFixed(1)}%
      </span>
      <div>
        {record.applied ? (
          <span className="inline-flex items-center gap-1 text-xs text-apple-green">
            <CheckCircle2 className="w-3 h-3" />
            已应用
          </span>
        ) : (
          <span className="inline-flex items-center gap-1 text-xs text-apple-gray-400">
            <XCircle className="w-3 h-3" />
            未应用
          </span>
        )}
      </div>
    </div>
  )
}
