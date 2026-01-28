import { useRef, useEffect, useState } from 'react'
import { Trash2, Info, AlertTriangle, XCircle, CheckCircle2, Zap, Clock, Copy, Check } from 'lucide-react'
import { LogEntry } from '../types'

interface LogsProps {
  logs: LogEntry[]
  onClear: () => void
}

export function Logs({ logs, onClear }: LogsProps) {
  const scrollRef = useRef<HTMLDivElement>(null)
  const [copied, setCopied] = useState(false)

  // 自动滚动到底部
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight
    }
  }, [logs])

  const getLevelIcon = (level: LogEntry['level']) => {
    switch (level) {
      case 'success':
        return <CheckCircle2 className="w-4 h-4 text-apple-green" />
      case 'info':
        return <Info className="w-4 h-4 text-apple-blue" />
      case 'warning':
        return <AlertTriangle className="w-4 h-4 text-apple-orange" />
      case 'error':
        return <XCircle className="w-4 h-4 text-apple-red" />
    }
  }

  const getLevelBg = (level: LogEntry['level']) => {
    switch (level) {
      case 'success':
        return 'bg-apple-green/5 border-apple-green/20'
      case 'info':
        return 'bg-apple-blue/5 border-apple-blue/20'
      case 'warning':
        return 'bg-apple-orange/5 border-apple-orange/20'
      case 'error':
        return 'bg-apple-red/5 border-apple-red/20'
    }
  }

  // 统计各级别日志数量
  const stats = {
    total: logs.length,
    success: logs.filter((l) => l.level === 'success').length,
    info: logs.filter((l) => l.level === 'info').length,
    warning: logs.filter((l) => l.level === 'warning').length,
    error: logs.filter((l) => l.level === 'error').length,
  }

  // 一键复制所有日志
  const copyAllLogs = async () => {
    const logText = logs
      .map((log) => `[${log.timestamp}] [${log.level.toUpperCase()}] ${log.message}`)
      .join('\n')

    // 添加系统信息头
    const header = `=== AnyFAST 日志导出 ===
时间: ${new Date().toLocaleString('zh-CN')}
总计: ${stats.total} 条 (成功: ${stats.success}, 信息: ${stats.info}, 警告: ${stats.warning}, 错误: ${stats.error})
${'='.repeat(30)}

`
    try {
      await navigator.clipboard.writeText(header + logText)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch (err) {
      console.error('复制失败:', err)
    }
  }

  return (
    <div className="h-full flex flex-col p-4 lg:p-6 overflow-y-auto">
      {/* Header */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 mb-4 lg:mb-6">
        <div>
          <h1 className="text-xl lg:text-2xl font-semibold text-apple-gray-600">运行日志</h1>
          <p className="text-sm text-apple-gray-400 mt-1">查看操作记录和测试结果</p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={copyAllLogs}
            disabled={logs.length === 0}
            className={`flex items-center gap-2 px-3 py-2 text-sm rounded-xl transition-colors disabled:opacity-50 ${
              copied
                ? 'text-apple-green bg-apple-green/10'
                : 'text-apple-gray-500 bg-apple-gray-100 hover:bg-apple-gray-200'
            }`}
          >
            {copied ? (
              <>
                <Check className="w-4 h-4" />
                已复制
              </>
            ) : (
              <>
                <Copy className="w-4 h-4" />
                <span className="hidden sm:inline">复制日志</span>
              </>
            )}
          </button>
          <button
            onClick={onClear}
            disabled={logs.length === 0}
            className="flex items-center gap-2 px-3 py-2 text-sm text-apple-gray-500 bg-apple-gray-100 rounded-xl hover:bg-apple-gray-200 transition-colors disabled:opacity-50"
          >
            <Trash2 className="w-4 h-4" />
            <span className="hidden sm:inline">清空日志</span>
          </button>
        </div>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-2 lg:gap-3 mb-4 lg:mb-6">
        <StatCard icon={<Clock className="w-4 h-4" />} label="总计" value={stats.total} color="gray" />
        <StatCard icon={<CheckCircle2 className="w-4 h-4" />} label="成功" value={stats.success} color="green" />
        <StatCard icon={<Info className="w-4 h-4" />} label="信息" value={stats.info} color="blue" />
        <StatCard icon={<AlertTriangle className="w-4 h-4" />} label="警告" value={stats.warning} color="orange" />
        <StatCard icon={<XCircle className="w-4 h-4" />} label="错误" value={stats.error} color="red" />
      </div>

      {/* Log List */}
      <div className="flex-1 bg-white/70 backdrop-blur-sm rounded-2xl shadow-sm border border-gray-100 overflow-hidden min-h-0">
        <div
          ref={scrollRef}
          className="h-full overflow-y-auto p-3 lg:p-4 space-y-2"
        >
          {logs.length === 0 ? (
            <div className="h-full flex flex-col items-center justify-center text-apple-gray-400 min-h-[200px]">
              <Zap className="w-12 lg:w-16 h-12 lg:h-16 mb-4 opacity-20" />
              <p className="text-sm">暂无日志记录</p>
              <p className="text-xs mt-1 opacity-60">开始测速后将显示操作记录</p>
            </div>
          ) : (
            logs.map((log, index) => (
              <div
                key={index}
                className={`
                  flex items-start gap-2 lg:gap-3 p-2.5 lg:p-3 rounded-xl border
                  ${getLevelBg(log.level)}
                  animate-fade-in
                `}
              >
                <div className="mt-0.5 flex-shrink-0">{getLevelIcon(log.level)}</div>
                <div className="flex-1 min-w-0">
                  <p className="text-xs lg:text-sm text-apple-gray-600 break-words">{log.message}</p>
                </div>
                <span className="text-xs text-apple-gray-400 font-mono whitespace-nowrap flex-shrink-0">
                  {log.timestamp}
                </span>
              </div>
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
  value: number
  color: 'gray' | 'green' | 'blue' | 'orange' | 'red'
}) {
  const colorMap = {
    gray: 'text-apple-gray-500 bg-apple-gray-100',
    green: 'text-apple-green bg-apple-green/10',
    blue: 'text-apple-blue bg-apple-blue/10',
    orange: 'text-apple-orange bg-apple-orange/10',
    red: 'text-apple-red bg-apple-red/10',
  }

  return (
    <div className="bg-white/70 backdrop-blur-sm rounded-xl p-2.5 lg:p-3 shadow-sm border border-gray-100">
      <div className={`w-7 lg:w-8 h-7 lg:h-8 rounded-lg flex items-center justify-center ${colorMap[color]} mb-1.5 lg:mb-2`}>
        {icon}
      </div>
      <p className="text-xs text-apple-gray-400">{label}</p>
      <p className="text-base lg:text-lg font-semibold text-apple-gray-600">{value}</p>
    </div>
  )
}
