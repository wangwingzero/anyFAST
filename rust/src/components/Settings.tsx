import { useState } from 'react'
import { Plus, Trash2, Save, Globe, Clock, Gauge, Monitor, RotateCcw, AlertTriangle, Zap, Power, FileText, ExternalLink } from 'lucide-react'
import { Endpoint, AppConfig } from '../types'
import { invoke } from '@tauri-apps/api/core'

// 默认端点
const DEFAULT_ENDPOINTS: Endpoint[] = [
  {
    name: 'WZW 代理',
    url: 'https://wzw.pp.ua/v1',
    domain: 'wzw.pp.ua',
    enabled: true,
  },
  {
    name: 'BetterClaude',
    url: 'https://betterclau.de/claude/anyrouter.top',
    domain: 'betterclau.de',
    enabled: true,
  },
]

// 默认配置
const DEFAULT_CONFIG = {
  mode: 'auto' as const,
  check_interval: 30,
  slow_threshold: 50,
  failure_threshold: 3,
  test_count: 3,
  minimize_to_tray: true,
  close_to_tray: true,
  clear_on_exit: false,
  cloudflare_ips: '',
}

interface SettingsProps {
  endpoints: Endpoint[]
  config: AppConfig | null
  onEndpointsChange: (endpoints: Endpoint[]) => void
  onConfigChange: (config: AppConfig) => void
}

export function Settings({
  endpoints,
  config,
  onEndpointsChange,
  onConfigChange,
}: SettingsProps) {
  const [newName, setNewName] = useState('')
  const [newUrl, setNewUrl] = useState('')
  const [cfIps, setCfIps] = useState(config?.cloudflare_ips.join('\n') || '')
  const [mode, setMode] = useState(config?.mode || 'auto')
  const [checkInterval, setCheckInterval] = useState(config?.check_interval || 30)
  const [slowThreshold, setSlowThreshold] = useState(config?.slow_threshold || 50)
  const [failureThreshold, setFailureThreshold] = useState(config?.failure_threshold || 3)
  const [minimizeToTray, setMinimizeToTray] = useState(config?.minimize_to_tray ?? true)
  const [closeToTray, setCloseToTray] = useState(config?.close_to_tray ?? true)
  const [clearOnExit, setClearOnExit] = useState(config?.clear_on_exit ?? false)

  const addEndpoint = () => {
    if (!newUrl.trim()) return
    const domain = newUrl.replace(/^https?:\/\//, '').split('/')[0]
    const name = newName.trim() || domain
    const newEndpoint: Endpoint = { name, url: newUrl, domain, enabled: true }
    onEndpointsChange([...endpoints, newEndpoint])
    setNewName('')
    setNewUrl('')
  }

  const removeEndpoint = (index: number) => {
    onEndpointsChange(endpoints.filter((_, i) => i !== index))
  }

  const toggleEndpoint = (index: number) => {
    const updated = [...endpoints]
    updated[index].enabled = !updated[index].enabled
    onEndpointsChange(updated)
  }

  const restoreDefaultEndpoints = () => {
    const currentUrls = new Set(endpoints.map((e) => e.url))
    const missingDefaults = DEFAULT_ENDPOINTS.filter((e) => !currentUrls.has(e.url))
    if (missingDefaults.length > 0) {
      onEndpointsChange([...endpoints, ...missingDefaults])
    }
  }

  const restoreAllDefaults = () => {
    setMode(DEFAULT_CONFIG.mode)
    setCheckInterval(DEFAULT_CONFIG.check_interval)
    setSlowThreshold(DEFAULT_CONFIG.slow_threshold)
    setFailureThreshold(DEFAULT_CONFIG.failure_threshold)
    setMinimizeToTray(DEFAULT_CONFIG.minimize_to_tray)
    setCloseToTray(DEFAULT_CONFIG.close_to_tray)
    setClearOnExit(DEFAULT_CONFIG.clear_on_exit)
    setCfIps(DEFAULT_CONFIG.cloudflare_ips)
    onEndpointsChange(DEFAULT_ENDPOINTS)
  }

  const saveSettings = async () => {
    const newConfig: AppConfig = {
      mode: mode as 'manual' | 'auto',
      check_interval: checkInterval,
      slow_threshold: slowThreshold,
      failure_threshold: failureThreshold,
      test_count: config?.test_count || 3,
      minimize_to_tray: minimizeToTray,
      close_to_tray: closeToTray,
      clear_on_exit: clearOnExit,
      cloudflare_ips: cfIps.split('\n').filter((ip) => ip.trim() && !ip.startsWith('#')),
      endpoints,
    }

    try {
      await invoke('save_config', { config: newConfig })
      onConfigChange(newConfig)
    } catch (e) {
      console.error('Save failed:', e)
    }
  }

  return (
    <div className="h-full overflow-y-auto p-6">
      <div className="max-w-2xl">
        {/* Header */}
        <div className="mb-8">
          <h1 className="text-2xl font-semibold text-apple-gray-600">设置</h1>
          <p className="text-sm text-apple-gray-400 mt-1">配置端点和运行参数</p>
        </div>

        {/* Endpoints Section */}
        <Section icon={<Globe className="w-5 h-5" />} title="中转站端点">
          <div className="space-y-2">
            {endpoints.map((endpoint, index) => (
              <div
                key={index}
                className="flex items-center gap-3 p-3 bg-apple-gray-50 rounded-apple"
              >
                <input
                  type="checkbox"
                  checked={endpoint.enabled}
                  onChange={() => toggleEndpoint(index)}
                  className="w-4 h-4 rounded accent-apple-blue"
                />
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-apple-gray-600 truncate">
                    {endpoint.name}
                  </p>
                  <p className="text-xs text-apple-gray-400 font-mono truncate">
                    {endpoint.url}
                  </p>
                </div>
                <button
                  onClick={() => removeEndpoint(index)}
                  className="p-1.5 text-apple-gray-400 hover:text-apple-red hover:bg-apple-red/10 rounded-md transition-colors"
                >
                  <Trash2 className="w-4 h-4" />
                </button>
              </div>
            ))}
          </div>

          {/* Add New */}
          <div className="mt-4 pt-4 border-t border-apple-gray-200">
            <p className="text-xs text-apple-gray-400 mb-2">添加新端点</p>
            <div className="flex gap-2">
              <input
                type="text"
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                placeholder="名称"
                className="w-24 px-3 py-2 text-sm bg-apple-gray-50 border border-apple-gray-200 rounded-apple focus:outline-none focus:ring-2 focus:ring-apple-blue/30"
              />
              <input
                type="text"
                value={newUrl}
                onChange={(e) => setNewUrl(e.target.value)}
                placeholder="URL (https://example.com/v1)"
                className="flex-1 px-3 py-2 text-sm bg-apple-gray-50 border border-apple-gray-200 rounded-apple focus:outline-none focus:ring-2 focus:ring-apple-blue/30 font-mono"
              />
              <button
                onClick={addEndpoint}
                disabled={!newUrl.trim()}
                className="flex items-center gap-1 px-3 py-2 bg-apple-green text-white text-sm font-medium rounded-apple btn-press hover:opacity-90 transition-opacity disabled:opacity-50"
              >
                <Plus className="w-4 h-4" />
                添加
              </button>
            </div>

            {/* Restore Defaults */}
            <div className="mt-3 flex items-center gap-3">
              <button
                onClick={restoreDefaultEndpoints}
                className="flex items-center gap-1.5 px-3 py-2 bg-apple-gray-50 border border-apple-gray-200 text-apple-gray-600 text-sm font-medium rounded-apple hover:bg-apple-gray-100 transition-colors"
              >
                <RotateCcw className="w-4 h-4" />
                恢复默认端点
              </button>
              <span className="text-xs text-apple-gray-400">
                恢复 BetterClaude 和 WZW 代理
              </span>
            </div>
          </div>
        </Section>

        {/* Mode Section */}
        <Section icon={<Gauge className="w-5 h-5" />} title="运行模式">
          <div className="space-y-3">
            <label className="flex items-center gap-3 p-3 bg-apple-gray-50 rounded-apple cursor-pointer">
              <input
                type="radio"
                name="mode"
                value="manual"
                checked={mode === 'manual'}
                onChange={() => setMode('manual')}
                className="w-4 h-4 accent-apple-blue"
              />
              <div>
                <p className="text-sm font-medium text-apple-gray-600">手动模式</p>
                <p className="text-xs text-apple-gray-400">手动测速并应用绑定</p>
              </div>
            </label>
            <label className="flex items-center gap-3 p-3 bg-apple-gray-50 rounded-apple cursor-pointer">
              <input
                type="radio"
                name="mode"
                value="auto"
                checked={mode === 'auto'}
                onChange={() => setMode('auto')}
                className="w-4 h-4 accent-apple-blue"
              />
              <div>
                <p className="text-sm font-medium text-apple-gray-600">自动模式</p>
                <p className="text-xs text-apple-gray-400">检测到延迟变高或连接失败时自动切换 IP</p>
              </div>
            </label>
          </div>

          {mode === 'auto' && (
            <div className="mt-4 pt-4 border-t border-apple-gray-200 space-y-4">
              {/* 健康检查间隔 */}
              <div className="flex items-center gap-4">
                <Clock className="w-4 h-4 text-apple-gray-400" />
                <span className="text-sm text-apple-gray-500 w-24">检查间隔</span>
                <select
                  value={checkInterval}
                  onChange={(e) => setCheckInterval(Number(e.target.value))}
                  className="px-3 py-1.5 text-sm bg-apple-gray-50 border border-apple-gray-200 rounded-apple focus:outline-none focus:ring-2 focus:ring-apple-blue/30"
                >
                  <option value={10}>10 秒（高可用）</option>
                  <option value={30}>30 秒（推荐）</option>
                  <option value={60}>60 秒</option>
                </select>
              </div>

              {/* 慢速阈值 */}
              <div className="flex items-center gap-4">
                <Zap className="w-4 h-4 text-apple-gray-400" />
                <span className="text-sm text-apple-gray-500 w-24">慢速阈值</span>
                <select
                  value={slowThreshold}
                  onChange={(e) => setSlowThreshold(Number(e.target.value))}
                  className="px-3 py-1.5 text-sm bg-apple-gray-50 border border-apple-gray-200 rounded-apple focus:outline-none focus:ring-2 focus:ring-apple-blue/30"
                >
                  <option value={30}>比基准慢 30%（严格）</option>
                  <option value={50}>比基准慢 50%（推荐）</option>
                  <option value={100}>比基准慢 100%（宽松）</option>
                </select>
                <span className="text-xs text-apple-gray-400">与基准对比</span>
              </div>

              {/* 失败阈值 */}
              <div className="flex items-center gap-4">
                <AlertTriangle className="w-4 h-4 text-apple-gray-400" />
                <span className="text-sm text-apple-gray-500 w-24">失败阈值</span>
                <select
                  value={failureThreshold}
                  onChange={(e) => setFailureThreshold(Number(e.target.value))}
                  className="px-3 py-1.5 text-sm bg-apple-gray-50 border border-apple-gray-200 rounded-apple focus:outline-none focus:ring-2 focus:ring-apple-blue/30"
                >
                  <option value={2}>2 次</option>
                  <option value={3}>3 次（推荐）</option>
                  <option value={5}>5 次</option>
                </select>
                <span className="text-xs text-apple-gray-400">连续失败次数</span>
              </div>
            </div>
          )}
        </Section>

        {/* Cloudflare IPs */}
        <Section icon={<Globe className="w-5 h-5" />} title="Cloudflare 优选 IP">
          <p className="text-xs text-apple-gray-400 mb-2">
            自定义优选 IP（每行一个，# 开头为注释）
          </p>
          <textarea
            value={cfIps}
            onChange={(e) => setCfIps(e.target.value)}
            placeholder="# 示例：&#10;103.21.244.176&#10;104.16.0.1"
            className="w-full h-24 px-3 py-2 text-sm bg-apple-gray-50 border border-apple-gray-200 rounded-apple focus:outline-none focus:ring-2 focus:ring-apple-blue/30 font-mono resize-none selectable"
          />
        </Section>

        {/* UI Settings */}
        <Section icon={<Monitor className="w-5 h-5" />} title="界面">
          <div className="space-y-3">
            <label className="flex items-center justify-between p-3 bg-apple-gray-50 rounded-apple cursor-pointer">
              <div>
                <span className="text-sm text-apple-gray-600">点击关闭按钮时最小化到托盘</span>
                <p className="text-xs text-apple-gray-400 mt-0.5">关闭后点击关闭按钮将直接退出程序</p>
              </div>
              <div
                className={`w-11 h-6 rounded-full p-0.5 transition-colors ${
                  closeToTray ? 'bg-apple-green' : 'bg-apple-gray-300'
                }`}
                onClick={() => setCloseToTray(!closeToTray)}
              >
                <div
                  className={`w-5 h-5 bg-white rounded-full shadow transition-transform ${
                    closeToTray ? 'translate-x-5' : 'translate-x-0'
                  }`}
                />
              </div>
            </label>
            <label className="flex items-center justify-between p-3 bg-apple-gray-50 rounded-apple cursor-pointer">
              <span className="text-sm text-apple-gray-600">最小化时隐藏到托盘</span>
              <div
                className={`w-11 h-6 rounded-full p-0.5 transition-colors ${
                  minimizeToTray ? 'bg-apple-green' : 'bg-apple-gray-300'
                }`}
                onClick={() => setMinimizeToTray(!minimizeToTray)}
              >
                <div
                  className={`w-5 h-5 bg-white rounded-full shadow transition-transform ${
                    minimizeToTray ? 'translate-x-5' : 'translate-x-0'
                  }`}
                />
              </div>
            </label>
          </div>
        </Section>

        {/* Exit Settings */}
        <Section icon={<Power className="w-5 h-5" />} title="退出行为">
          <label className="flex items-center justify-between p-3 bg-apple-gray-50 rounded-apple cursor-pointer">
            <div>
              <span className="text-sm text-apple-gray-600">退出时清除 hosts 绑定</span>
              <p className="text-xs text-apple-gray-400 mt-0.5">退出程序时自动移除所有 hosts 优化，恢复原始状态</p>
            </div>
            <div
              className={`w-11 h-6 rounded-full p-0.5 transition-colors ${
                clearOnExit ? 'bg-apple-green' : 'bg-apple-gray-300'
              }`}
              onClick={() => setClearOnExit(!clearOnExit)}
            >
              <div
                className={`w-5 h-5 bg-white rounded-full shadow transition-transform ${
                  clearOnExit ? 'translate-x-5' : 'translate-x-0'
                }`}
              />
            </div>
          </label>
        </Section>

        {/* Advanced */}
        <Section icon={<FileText className="w-5 h-5" />} title="高级">
          <div className="flex items-center justify-between p-3 bg-apple-gray-50 rounded-apple">
            <div>
              <span className="text-sm text-apple-gray-600">Hosts 文件</span>
              <p className="text-xs text-apple-gray-400 mt-0.5">手动编辑系统 hosts 文件</p>
            </div>
            <button
              onClick={async () => {
                try {
                  await invoke('open_hosts_file')
                } catch (e) {
                  console.error('Failed to open hosts file:', e)
                }
              }}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-apple-gray-200 text-apple-gray-600 text-sm font-medium rounded-apple hover:bg-apple-gray-300 transition-colors"
            >
              <ExternalLink className="w-4 h-4" />
              打开
            </button>
          </div>
        </Section>

        {/* Action Buttons */}
        <div className="flex items-center gap-3 mt-6">
          <button
            onClick={saveSettings}
            className="flex items-center gap-2 px-5 py-2.5 bg-apple-blue text-white text-sm font-medium rounded-apple shadow-apple btn-press hover:bg-apple-blue-hover transition-colors"
          >
            <Save className="w-4 h-4" />
            保存设置
          </button>
          <button
            onClick={restoreAllDefaults}
            className="flex items-center gap-2 px-5 py-2.5 bg-apple-gray-50 border border-apple-gray-200 text-apple-gray-600 text-sm font-medium rounded-apple hover:bg-apple-gray-100 transition-colors"
          >
            <RotateCcw className="w-4 h-4" />
            恢复默认值
          </button>
        </div>
      </div>
    </div>
  )
}

function Section({
  icon,
  title,
  children,
}: {
  icon: React.ReactNode
  title: string
  children: React.ReactNode
}) {
  return (
    <div className="glass rounded-apple-lg p-5 shadow-apple mb-6">
      <div className="flex items-center gap-2 mb-4">
        <span className="text-apple-blue">{icon}</span>
        <h2 className="text-sm font-semibold text-apple-gray-600">{title}</h2>
      </div>
      {children}
    </div>
  )
}
