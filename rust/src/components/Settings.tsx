import { useState, useEffect, useRef, useCallback } from 'react'
import { Globe, Clock, Gauge, Monitor, RotateCcw, AlertTriangle, Zap, Power, FileText, ExternalLink, RefreshCw, Download, Info } from 'lucide-react'
import { Endpoint, AppConfig, UpdateInfo } from '../types'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-shell'

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
  const [cfIps, setCfIps] = useState(config?.cloudflare_ips.join('\n') || '')
  const [mode, setMode] = useState(config?.mode || 'auto')
  const [checkInterval, setCheckInterval] = useState(config?.check_interval || 30)
  const [slowThreshold, setSlowThreshold] = useState(config?.slow_threshold || 50)
  const [failureThreshold, setFailureThreshold] = useState(config?.failure_threshold || 3)
  const [minimizeToTray, setMinimizeToTray] = useState(config?.minimize_to_tray ?? true)
  const [closeToTray, setCloseToTray] = useState(config?.close_to_tray ?? true)
  const [clearOnExit, setClearOnExit] = useState(config?.clear_on_exit ?? false)

  // 更新检查状态
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null)
  const [checkingUpdate, setCheckingUpdate] = useState(false)
  const [updateError, setUpdateError] = useState<string | null>(null)
  const [currentVersion, setCurrentVersion] = useState<string>('')

  const initializedRef = useRef(false)

  // 自动保存函数
  const autoSave = useCallback(async (updates: Partial<{
    mode: string
    check_interval: number
    slow_threshold: number
    failure_threshold: number
    minimize_to_tray: boolean
    close_to_tray: boolean
    clear_on_exit: boolean
    cloudflare_ips: string
  }>) => {
    const newConfig: AppConfig = {
      mode: (updates.mode ?? mode) as 'manual' | 'auto',
      check_interval: updates.check_interval ?? checkInterval,
      slow_threshold: updates.slow_threshold ?? slowThreshold,
      failure_threshold: updates.failure_threshold ?? failureThreshold,
      test_count: config?.test_count || 3,
      minimize_to_tray: updates.minimize_to_tray ?? minimizeToTray,
      close_to_tray: updates.close_to_tray ?? closeToTray,
      clear_on_exit: updates.clear_on_exit ?? clearOnExit,
      cloudflare_ips: (updates.cloudflare_ips ?? cfIps).split('\n').filter((ip) => ip.trim() && !ip.startsWith('#')),
      endpoints,
    }

    try {
      await invoke('save_config', { config: newConfig })
      onConfigChange(newConfig)
    } catch (e) {
      console.error('Auto save failed:', e)
    }
  }, [mode, checkInterval, slowThreshold, failureThreshold, minimizeToTray, closeToTray, clearOnExit, cfIps, endpoints, config, onConfigChange])

  // 初始化
  useEffect(() => {
    if (config && !initializedRef.current) {
      initializedRef.current = true
      setCfIps(config.cloudflare_ips.join('\n'))
      setMode(config.mode)
      setCheckInterval(config.check_interval)
      setSlowThreshold(config.slow_threshold)
      setFailureThreshold(config.failure_threshold)
      setMinimizeToTray(config.minimize_to_tray)
      setCloseToTray(config.close_to_tray)
      setClearOnExit(config.clear_on_exit)
    }
  }, [config])

  // 获取当前版本号
  useEffect(() => {
    invoke<string>('get_current_version').then(setCurrentVersion).catch(console.error)
  }, [])

  // 检查更新
  const checkForUpdate = async () => {
    setCheckingUpdate(true)
    setUpdateError(null)
    try {
      const info = await invoke<UpdateInfo>('check_for_update')
      setUpdateInfo(info)
    } catch (e) {
      setUpdateError(e as string)
    } finally {
      setCheckingUpdate(false)
    }
  }

  // 打开下载页面
  const openReleasePage = async () => {
    if (updateInfo?.releaseUrl) {
      try {
        await open(updateInfo.releaseUrl)
      } catch (e) {
        console.error('Failed to open release page:', e)
      }
    }
  }

  // 带自动保存的 setter
  const updateMode = (v: 'manual' | 'auto') => { setMode(v); autoSave({ mode: v }) }
  const updateCheckInterval = (v: number) => { setCheckInterval(v); autoSave({ check_interval: v }) }
  const updateSlowThreshold = (v: number) => { setSlowThreshold(v); autoSave({ slow_threshold: v }) }
  const updateFailureThreshold = (v: number) => { setFailureThreshold(v); autoSave({ failure_threshold: v }) }
  const updateMinimizeToTray = (v: boolean) => { setMinimizeToTray(v); autoSave({ minimize_to_tray: v }) }
  const updateCloseToTray = (v: boolean) => { setCloseToTray(v); autoSave({ close_to_tray: v }) }
  const updateClearOnExit = (v: boolean) => { setClearOnExit(v); autoSave({ clear_on_exit: v }) }

  // cfIps 用 debounce 保存（输入时不立即保存）
  const cfIpsTimeoutRef = useRef<ReturnType<typeof setTimeout>>()
  const updateCfIps = (v: string) => {
    setCfIps(v)
    if (cfIpsTimeoutRef.current) clearTimeout(cfIpsTimeoutRef.current)
    cfIpsTimeoutRef.current = setTimeout(() => autoSave({ cloudflare_ips: v }), 500)
  }

  const restoreAllDefaults = async () => {
    setMode(DEFAULT_CONFIG.mode)
    setCheckInterval(DEFAULT_CONFIG.check_interval)
    setSlowThreshold(DEFAULT_CONFIG.slow_threshold)
    setFailureThreshold(DEFAULT_CONFIG.failure_threshold)
    setMinimizeToTray(DEFAULT_CONFIG.minimize_to_tray)
    setCloseToTray(DEFAULT_CONFIG.close_to_tray)
    setClearOnExit(DEFAULT_CONFIG.clear_on_exit)
    setCfIps(DEFAULT_CONFIG.cloudflare_ips)
    onEndpointsChange(DEFAULT_ENDPOINTS)

    // 保存默认配置
    const newConfig: AppConfig = {
      mode: DEFAULT_CONFIG.mode,
      check_interval: DEFAULT_CONFIG.check_interval,
      slow_threshold: DEFAULT_CONFIG.slow_threshold,
      failure_threshold: DEFAULT_CONFIG.failure_threshold,
      test_count: config?.test_count || 3,
      minimize_to_tray: DEFAULT_CONFIG.minimize_to_tray,
      close_to_tray: DEFAULT_CONFIG.close_to_tray,
      clear_on_exit: DEFAULT_CONFIG.clear_on_exit,
      cloudflare_ips: [],
      endpoints: DEFAULT_ENDPOINTS,
    }
    try {
      await invoke('save_config', { config: newConfig })
      onConfigChange(newConfig)
    } catch (e) {
      console.error('Restore defaults failed:', e)
    }
  }

  return (
    <div className="h-full overflow-y-auto p-4 lg:p-6">
      <div className="max-w-2xl w-full">
        {/* Header */}
        <div className="mb-6 lg:mb-8">
          <h1 className="text-xl lg:text-2xl font-semibold text-apple-gray-600">设置</h1>
          <p className="text-sm text-apple-gray-400 mt-1">配置运行参数</p>
        </div>

        {/* Mode Section */}
        <Section icon={<Gauge className="w-5 h-5" />} title="运行模式">
          <div className="space-y-3">
            <label className="flex items-center gap-3 p-3 bg-apple-gray-50 rounded-xl cursor-pointer">
              <input
                type="radio"
                name="mode"
                value="manual"
                checked={mode === 'manual'}
                onChange={() => updateMode('manual')}
                className="w-4 h-4 accent-apple-blue"
              />
              <div>
                <p className="text-sm font-medium text-apple-gray-600">手动模式</p>
                <p className="text-xs text-apple-gray-400">手动测速并应用绑定</p>
              </div>
            </label>
            <label className="flex items-center gap-3 p-3 bg-apple-gray-50 rounded-xl cursor-pointer">
              <input
                type="radio"
                name="mode"
                value="auto"
                checked={mode === 'auto'}
                onChange={() => updateMode('auto')}
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
              <div className="flex flex-col sm:flex-row sm:items-center gap-2 sm:gap-4">
                <div className="flex items-center gap-2">
                  <Clock className="w-4 h-4 text-apple-gray-400 flex-shrink-0" />
                  <span className="text-sm text-apple-gray-500 w-20 lg:w-24">检查间隔</span>
                </div>
                <select
                  value={checkInterval}
                  onChange={(e) => updateCheckInterval(Number(e.target.value))}
                  className="flex-1 sm:flex-none px-3 py-1.5 text-sm bg-apple-gray-50 border border-apple-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-apple-blue/30"
                >
                  <option value={10}>10 秒（高可用）</option>
                  <option value={30}>30 秒（推荐）</option>
                  <option value={60}>60 秒</option>
                </select>
              </div>

              {/* 慢速阈值 */}
              <div className="flex flex-col sm:flex-row sm:items-center gap-2 sm:gap-4">
                <div className="flex items-center gap-2">
                  <Zap className="w-4 h-4 text-apple-gray-400 flex-shrink-0" />
                  <span className="text-sm text-apple-gray-500 w-20 lg:w-24">慢速阈值</span>
                </div>
                <div className="flex items-center gap-2 flex-1">
                  <select
                    value={slowThreshold}
                    onChange={(e) => updateSlowThreshold(Number(e.target.value))}
                    className="flex-1 sm:flex-none px-3 py-1.5 text-sm bg-apple-gray-50 border border-apple-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-apple-blue/30"
                  >
                    <option value={30}>比基准慢 30%（严格）</option>
                    <option value={50}>比基准慢 50%（推荐）</option>
                    <option value={100}>比基准慢 100%（宽松）</option>
                  </select>
                  <span className="hidden lg:inline text-xs text-apple-gray-400">与基准对比</span>
                </div>
              </div>

              {/* 失败阈值 */}
              <div className="flex flex-col sm:flex-row sm:items-center gap-2 sm:gap-4">
                <div className="flex items-center gap-2">
                  <AlertTriangle className="w-4 h-4 text-apple-gray-400 flex-shrink-0" />
                  <span className="text-sm text-apple-gray-500 w-20 lg:w-24">失败阈值</span>
                </div>
                <div className="flex items-center gap-2 flex-1">
                  <select
                    value={failureThreshold}
                    onChange={(e) => updateFailureThreshold(Number(e.target.value))}
                    className="flex-1 sm:flex-none px-3 py-1.5 text-sm bg-apple-gray-50 border border-apple-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-apple-blue/30"
                  >
                    <option value={2}>2 次</option>
                    <option value={3}>3 次（推荐）</option>
                    <option value={5}>5 次</option>
                  </select>
                  <span className="hidden lg:inline text-xs text-apple-gray-400">连续失败次数</span>
                </div>
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
            onChange={(e) => updateCfIps(e.target.value)}
            placeholder="# 示例：&#10;103.21.244.176&#10;104.16.0.1"
            className="w-full h-24 px-3 py-2 text-sm bg-apple-gray-50 border border-apple-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-apple-blue/30 font-mono resize-none selectable"
          />
        </Section>

        {/* UI Settings */}
        <Section icon={<Monitor className="w-5 h-5" />} title="界面">
          <div className="space-y-3">
            <label className="flex items-center justify-between p-3 bg-apple-gray-50 rounded-xl cursor-pointer">
              <div className="flex-1 min-w-0 mr-3">
                <span className="text-sm text-apple-gray-600">点击关闭按钮时最小化到托盘</span>
                <p className="text-xs text-apple-gray-400 mt-0.5">关闭后点击关闭按钮将直接退出程序</p>
              </div>
              <div
                className={`w-11 h-6 rounded-full p-0.5 transition-colors flex-shrink-0 ${
                  closeToTray ? 'bg-apple-green' : 'bg-apple-gray-300'
                }`}
                onClick={() => updateCloseToTray(!closeToTray)}
              >
                <div
                  className={`w-5 h-5 bg-white rounded-full shadow transition-transform ${
                    closeToTray ? 'translate-x-5' : 'translate-x-0'
                  }`}
                />
              </div>
            </label>
            <label className="flex items-center justify-between p-3 bg-apple-gray-50 rounded-xl cursor-pointer">
              <span className="text-sm text-apple-gray-600">最小化时隐藏到托盘</span>
              <div
                className={`w-11 h-6 rounded-full p-0.5 transition-colors flex-shrink-0 ${
                  minimizeToTray ? 'bg-apple-green' : 'bg-apple-gray-300'
                }`}
                onClick={() => updateMinimizeToTray(!minimizeToTray)}
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
          <label className="flex items-center justify-between p-3 bg-apple-gray-50 rounded-xl cursor-pointer">
            <div className="flex-1 min-w-0 mr-3">
              <span className="text-sm text-apple-gray-600">退出时清除 hosts 绑定</span>
              <p className="text-xs text-apple-gray-400 mt-0.5">退出程序时自动移除所有 hosts 优化，恢复原始状态</p>
            </div>
            <div
              className={`w-11 h-6 rounded-full p-0.5 transition-colors flex-shrink-0 ${
                clearOnExit ? 'bg-apple-green' : 'bg-apple-gray-300'
              }`}
              onClick={() => updateClearOnExit(!clearOnExit)}
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
          <div className="flex items-center justify-between p-3 bg-apple-gray-50 rounded-xl">
            <div className="flex-1 min-w-0 mr-3">
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
              className="flex items-center gap-1.5 px-3 py-1.5 bg-apple-gray-200 text-apple-gray-600 text-sm font-medium rounded-xl hover:bg-apple-gray-300 transition-colors flex-shrink-0"
            >
              <ExternalLink className="w-4 h-4" />
              打开
            </button>
          </div>
        </Section>

        {/* About & Update */}
        <Section icon={<Info className="w-5 h-5" />} title="关于">
          <div className="space-y-3">
            {/* 当前版本 */}
            <div className="flex items-center justify-between p-3 bg-apple-gray-50 rounded-xl">
              <div className="flex-1 min-w-0 mr-3">
                <span className="text-sm text-apple-gray-600">当前版本</span>
                <p className="text-xs text-apple-gray-400 mt-0.5">anyFAST v{currentVersion || '...'}</p>
              </div>
              <button
                onClick={checkForUpdate}
                disabled={checkingUpdate}
                className="flex items-center gap-1.5 px-3 py-1.5 bg-apple-blue text-white text-sm font-medium rounded-xl hover:bg-apple-blue/90 transition-colors flex-shrink-0 disabled:opacity-50"
              >
                <RefreshCw className={`w-4 h-4 ${checkingUpdate ? 'animate-spin' : ''}`} />
                {checkingUpdate ? '检查中...' : '检查更新'}
              </button>
            </div>

            {/* 更新结果 */}
            {updateError && (
              <div className="p-3 bg-red-50 border border-red-200 rounded-xl">
                <p className="text-sm text-red-600">检查更新失败: {updateError}</p>
              </div>
            )}

            {updateInfo && !updateError && (
              <div className={`p-3 rounded-xl ${updateInfo.hasUpdate ? 'bg-apple-green/10 border border-apple-green/30' : 'bg-apple-gray-50'}`}>
                {updateInfo.hasUpdate ? (
                  <div className="space-y-2">
                    <div className="flex items-center gap-2">
                      <Download className="w-4 h-4 text-apple-green" />
                      <span className="text-sm font-medium text-apple-green">发现新版本!</span>
                    </div>
                    <p className="text-sm text-apple-gray-600">
                      最新版本: <span className="font-medium">v{updateInfo.latestVersion}</span>
                      {updateInfo.publishedAt && (
                        <span className="text-apple-gray-400 ml-2">
                          ({new Date(updateInfo.publishedAt).toLocaleDateString('zh-CN')})
                        </span>
                      )}
                    </p>
                    {updateInfo.releaseNotes && (
                      <p className="text-xs text-apple-gray-400 line-clamp-2">{updateInfo.releaseNotes}</p>
                    )}
                    <button
                      onClick={openReleasePage}
                      className="flex items-center gap-1.5 px-3 py-1.5 bg-apple-green text-white text-sm font-medium rounded-xl hover:bg-apple-green/90 transition-colors mt-2"
                    >
                      <ExternalLink className="w-4 h-4" />
                      前往下载
                    </button>
                  </div>
                ) : (
                  <div className="flex items-center gap-2">
                    <span className="text-sm text-apple-gray-600">✓ 已是最新版本</span>
                  </div>
                )}
              </div>
            )}
          </div>
        </Section>

        {/* Action Button */}
        <div className="mt-6">
          <button
            onClick={restoreAllDefaults}
            className="flex items-center justify-center gap-2 px-5 py-2.5 bg-apple-gray-50 border border-apple-gray-200 text-apple-gray-600 text-sm font-medium rounded-xl hover:bg-apple-gray-100 transition-colors"
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
    <div className="bg-white/70 backdrop-blur-sm rounded-2xl p-4 lg:p-5 shadow-sm border border-gray-100 mb-4 lg:mb-6">
      <div className="flex items-center gap-2 mb-4">
        <span className="text-apple-blue">{icon}</span>
        <h2 className="text-sm font-semibold text-apple-gray-600">{title}</h2>
      </div>
      {children}
    </div>
  )
}
