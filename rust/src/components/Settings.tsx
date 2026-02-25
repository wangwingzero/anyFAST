import { useState, useEffect, useRef } from 'react'
import { RotateCcw, Power, FileText, ExternalLink, RefreshCw, Download, Info, PlayCircle, Loader2, CheckCircle2, Github, Star, AlertCircle, Repeat, Search, XCircle, Globe } from 'lucide-react'
import { Endpoint, AppConfig, UpdateInfo, DiagnosticStep } from '../types'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-shell'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'

// 默认端点（与后端 models.rs 保持一致）
const DEFAULT_ENDPOINTS: Endpoint[] = [
  {
    name: 'anyrouter',
    url: 'https://cf.betterclau.de/claude/anyrouter.top',
    domain: 'cf.betterclau.de',
    enabled: true,
  },
  {
    name: 'WONG公益站',
    url: 'https://wzw.pp.ua',
    domain: 'wzw.pp.ua',
    enabled: true,
  },
]

interface SettingsProps {
  config: AppConfig | null
  onEndpointsChange: (endpoints: Endpoint[]) => void
  onConfigChange: (config: AppConfig) => void
}

export function Settings({
  config,
  onEndpointsChange,
  onConfigChange,
}: SettingsProps) {
  // 更新检查状态
  const [checkingUpdate, setCheckingUpdate] = useState(false)
  const [updateError, setUpdateError] = useState<string | null>(null)
  const [currentVersion, setCurrentVersion] = useState<string>('')
  const [updateChecked, setUpdateChecked] = useState(false)

  // 更新下载状态
  const [updatePhase, setUpdatePhase] = useState<'idle' | 'downloading' | 'installing' | 'restarting'>('idle')
  const [downloadProgress, setDownloadProgress] = useState(0)
  const [downloadTotal, setDownloadTotal] = useState(0)
  const updateRef = useRef<Update | null>(null)
  // 降级方案：Rust 侧检测到的更新信息（当 Tauri updater 插件失败时使用）
  const [fallbackUpdateInfo, setFallbackUpdateInfo] = useState<UpdateInfo | null>(null)

  // 更新日志
  const [updateLogs, setUpdateLogs] = useState<string[]>([])
  const [showUpdateLog, setShowUpdateLog] = useState(false)
  const logRef = useRef<HTMLDivElement>(null)
  const addLog = (msg: string) => {
    const ts = new Date().toLocaleTimeString('zh-CN', { hour12: false })
    setUpdateLogs(prev => [...prev, `[${ts}] ${msg}`])
  }

  // 强制下载状态
  const [forceDownloading, setForceDownloading] = useState(false)

  // 更新排查诊断
  const [diagnosing, setDiagnosing] = useState(false)
  const [diagnosticSteps, setDiagnosticSteps] = useState<DiagnosticStep[] | null>(null)

  // 自启动状态
  const [autostart, setAutostart] = useState(config?.autostart ?? false)
  const [autostartLoading, setAutostartLoading] = useState(false)

  // 持续优化状态
  const [continuousMode, setContinuousMode] = useState(config?.continuous_mode ?? true)
  const [continuousModeLoading, setContinuousModeLoading] = useState(false)

  // 更新代理状态
  const [updateProxy, setUpdateProxy] = useState(config?.update_proxy ?? 'auto')
  const [customProxy, setCustomProxy] = useState('')
  const [proxyLoading, setProxyLoading] = useState(false)
  const [detectedProxy, setDetectedProxy] = useState<string | null>(null)

  const initializedRef = useRef(false)

  // 初始化
  useEffect(() => {
    if (!config) return
    if (!initializedRef.current) {
      initializedRef.current = true
      setContinuousMode(config.continuous_mode ?? true)
      const proxy = config.update_proxy ?? 'auto'
      setUpdateProxy(proxy)
      // 如果是自定义代理地址，填入输入框
      if (proxy !== 'auto' && proxy !== '') {
        setCustomProxy(proxy)
      }
      // 自动检测系统代理（仅展示）
      invoke<string | null>('detect_system_proxy').then(setDetectedProxy).catch(() => {})
    }
  }, [config])

  // 日志自动滚动到底部
  useEffect(() => {
    if (logRef.current && showUpdateLog) {
      logRef.current.scrollTop = logRef.current.scrollHeight
    }
  }, [updateLogs, showUpdateLog])

  // 获取当前版本号
  useEffect(() => {
    invoke<string>('get_current_version').then(setCurrentVersion).catch(console.error)
  }, [])

  // 获取自启动状态
  useEffect(() => {
    invoke<boolean>('get_autostart')
      .then(setAutostart)
      .catch(console.error)
  }, [])

  // 更新自启动设置
  const updateAutostart = async (enabled: boolean) => {
    setAutostartLoading(true)
    try {
      await invoke('set_autostart', { enabled })
      setAutostart(enabled)
    } catch (e) {
      console.error('Failed to set autostart:', e)
      // 恢复原状态
      setAutostart(!enabled)
    } finally {
      setAutostartLoading(false)
    }
  }

  // 更新持续优化模式
  const updateContinuousMode = async (enabled: boolean) => {
    if (!config) return
    setContinuousModeLoading(true)
    try {
      const newConfig: AppConfig = { ...config, continuous_mode: enabled }
      await invoke('save_config', { config: newConfig })
      onConfigChange(newConfig)
      setContinuousMode(enabled)
      if (!enabled) {
        await invoke('stop_continuous_optimization')
      } else {
        // 如果有活跃绑定则启动
        const hasBindings = await invoke<boolean>('has_any_bindings')
        if (hasBindings) {
          await invoke('start_continuous_optimization')
        }
      }
    } catch (e) {
      console.error('Failed to set continuous mode:', e)
      setContinuousMode(!enabled)
    } finally {
      setContinuousModeLoading(false)
    }
  }

  // 更新代理设置
  const saveProxySetting = async (value: string) => {
    if (!config) return
    setProxyLoading(true)
    try {
      const newConfig: AppConfig = { ...config, update_proxy: value }
      await invoke('save_config', { config: newConfig })
      onConfigChange(newConfig)
      setUpdateProxy(value)
    } catch (e) {
      console.error('Failed to save proxy setting:', e)
    } finally {
      setProxyLoading(false)
    }
  }

  // 检查更新 - 使用 Tauri updater 插件，失败时降级到 Rust 侧检测
  const checkForUpdate = async () => {
    setCheckingUpdate(true)
    setUpdateError(null)
    setUpdateChecked(false)
    updateRef.current = null
    setFallbackUpdateInfo(null)
    setDiagnosticSteps(null)
    setUpdateLogs([])
    addLog('开始检查更新...')

    // 解析代理地址
    let proxyUrl: string | undefined
    const proxySetting = config?.update_proxy ?? 'auto'
    if (proxySetting === 'auto') {
      addLog('自动检测系统代理...')
      try {
        const detected = await invoke<string | null>('detect_system_proxy')
        if (detected) {
          proxyUrl = detected
          addLog(`检测到系统代理: ${detected}`)
        } else {
          addLog('未检测到系统代理，直连')
        }
      } catch {
        addLog('代理检测失败，直连')
      }
    } else if (proxySetting) {
      proxyUrl = proxySetting
      addLog(`使用手动代理: ${proxySetting}`)
    } else {
      addLog('代理已禁用，直连')
    }

    const checkOpts: { timeout: number; proxy?: string } = { timeout: 30000 }
    if (proxyUrl) checkOpts.proxy = proxyUrl

    addLog(`尝试 Tauri updater 插件 (timeout=30s${proxyUrl ? ', proxy=' + proxyUrl : ''})`)
    try {
      const update = await check(checkOpts)
      if (update) {
        updateRef.current = update
        addLog(`插件检测到新版本: v${update.version}`)
        if (update.date) addLog(`发布日期: ${update.date}`)
      } else {
        addLog('插件返回: 已是最新版本')
      }
      setUpdateChecked(true)
    } catch (pluginErr) {
      // Tauri updater 插件失败，降级到 Rust 侧 check_for_update
      addLog(`插件失败: ${String(pluginErr)}`)
      addLog('降级到 Rust 侧 check_for_update...')
      console.warn('Tauri updater plugin failed, falling back to Rust check:', pluginErr)
      try {
        const info = await invoke<UpdateInfo>('check_for_update')
        addLog(`Rust 侧返回: 当前 v${info.currentVersion} → 最新 v${info.latestVersion}`)
        if (info.hasUpdate) {
          setFallbackUpdateInfo(info)
          setUpdateChecked(true)
          addLog('有新版本可用（降级方案，需手动下载）')
        } else {
          setUpdateChecked(true)
          addLog('Rust 侧确认: 已是最新版本')
        }
      } catch (fallbackErr) {
        // 两种方式都失败，显示原始错误
        addLog(`Rust 侧也失败: ${String(fallbackErr)}`)
        addLog('两种更新检查方式均失败')
        setUpdateError(String(pluginErr))
      }
    } finally {
      setCheckingUpdate(false)
      addLog('检查更新流程结束')
    }
  }

  // 执行应用内更新：下载 + 安装 + 重启
  const performUpdate = async () => {
    if (!updateRef.current) return

    setUpdatePhase('downloading')
    setDownloadProgress(0)
    setDownloadTotal(0)
    setUpdateError(null)
    addLog(`开始下载更新 v${updateRef.current.version}...`)

    try {
      let downloaded = 0

      await updateRef.current.downloadAndInstall((event) => {
        switch (event.event) {
          case 'Started':
            setDownloadTotal(event.data.contentLength ?? 0)
            addLog(`下载开始, 文件大小: ${event.data.contentLength ? formatBytes(event.data.contentLength) : '未知'}`)
            break
          case 'Progress':
            downloaded += event.data.chunkLength
            setDownloadProgress(downloaded)
            break
          case 'Finished':
            setUpdatePhase('installing')
            addLog(`下载完成, 共 ${formatBytes(downloaded)}`)
            addLog('开始安装...')
            break
        }
      })

      setUpdatePhase('restarting')
      addLog('安装完成，准备重启应用...')
      await relaunch()
    } catch (e) {
      const errMsg = String(e)
      addLog(`更新失败: ${errMsg}`)

      // Tauri updater 插件下载失败（常见于代理未透传），自动降级到 Rust 侧直接下载
      if (errMsg.includes('decoding response body') || errMsg.includes('error sending request') || errMsg.includes('connect error')) {
        addLog('检测到下载通道异常，自动切换到直接下载方式...')
        try {
          setUpdatePhase('downloading')
          const filePath = await invoke<string>('force_download_update')
          addLog(`安装包已下载: ${filePath}`)
          addLog('正在打开安装程序...')
          setUpdatePhase('idle')
          return
        } catch (fallbackErr) {
          addLog(`直接下载也失败: ${String(fallbackErr)}`)
        }
      }

      setUpdateError(errMsg)
      setUpdatePhase('idle')
    }
  }

  // 打开 GitHub Releases 页面（备用）
  const openReleasePage = async () => {
    try {
      await open('https://github.com/wangwingzero/anyFAST/releases/latest')
    } catch (e) {
      console.error('Failed to open release page:', e)
    }
  }

  // 格式化文件大小
  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i]
  }

  // 更新错误提示增强
  const getUpdateErrorHint = (error: string): string | null => {
    const normalized = error.toLowerCase()
    if (normalized.includes('signature verification failed') || normalized.includes('invalidsignature')) {
      return '当前版本内置更新公钥存在历史错误，请先到 GitHub 手动安装一次最新版；安装后应用内更新将恢复正常。'
    }
    if (normalized.includes('error sending request') || normalized.includes('timeout') || normalized.includes('connect error') || normalized.includes('network')) {
      return '网络连接异常，可能是代理设置或防火墙导致。请检查网络后重试，或直接到 GitHub 下载。'
    }
    return null
  }

  // 执行更新排查诊断
  const runDiagnostic = async () => {
    setDiagnosing(true)
    setDiagnosticSteps(null)
    addLog('开始更新排查诊断...')
    try {
      const steps = await invoke<DiagnosticStep[]>('diagnose_update')
      setDiagnosticSteps(steps)
      for (const step of steps) {
        addLog(`[诊断] ${step.name} (${step.status}): ${step.detail}`)
      }
    } catch (e) {
      addLog(`诊断失败: ${String(e)}`)
      setDiagnosticSteps([{
        name: '排查失败',
        status: 'error' as const,
        detail: String(e),
      }])
    } finally {
      setDiagnosing(false)
      addLog('排查诊断结束')
    }
  }

  // 强制下载安装包（绕过 Tauri updater 插件）
  const forceDownload = async () => {
    setForceDownloading(true)
    addLog('开始直接下载安装包...')
    try {
      const filePath = await invoke<string>('force_download_update')
      addLog(`安装包已下载: ${filePath}`)
      addLog('正在打开安装程序...')
    } catch (e) {
      const errMsg = String(e)
      addLog(`下载失败: ${errMsg}`)
      setUpdateError(errMsg)
    } finally {
      setForceDownloading(false)
    }
  }

  const restoreAllDefaults = async () => {
    onEndpointsChange(DEFAULT_ENDPOINTS)

    // 保存默认配置（保留后端专属字段的当前值，避免静默重置）
    const newConfig: AppConfig = {
      check_interval: config?.check_interval ?? 120,
      slow_threshold: config?.slow_threshold ?? 150,
      failure_threshold: config?.failure_threshold ?? 5,
      test_count: config?.test_count ?? 3,
      endpoints: DEFAULT_ENDPOINTS,
      autostart: config?.autostart ?? false,  // 保持当前自启动设置
      preferred_ips: [],
      continuous_mode: config?.continuous_mode ?? true,  // 保持当前持续优化设置
      test_aggressiveness: config?.test_aggressiveness ?? 2,
      update_proxy: 'auto',
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

        {/* System Settings */}
        <Section icon={<Power className="w-5 h-5" />} title="系统">
          <div className="space-y-3">
            <label className="flex items-center justify-between p-3 bg-apple-gray-50 rounded-xl cursor-pointer">
              <div className="flex-1 min-w-0 mr-3">
                <div className="flex items-center gap-2">
                  <PlayCircle className="w-4 h-4 text-apple-gray-400" />
                  <span className="text-sm text-apple-gray-600">开机自启动</span>
                </div>
                <p className="text-xs text-apple-gray-400 mt-0.5 ml-6">系统启动时自动运行 anyFAST</p>
              </div>
              <div
                className={`w-11 h-6 rounded-full p-0.5 transition-colors flex-shrink-0 ${
                  autostartLoading ? 'opacity-50 cursor-wait' : ''
                } ${autostart ? 'bg-apple-green' : 'bg-apple-gray-300'}`}
                onClick={() => !autostartLoading && updateAutostart(!autostart)}
              >
                <div
                  className={`w-5 h-5 bg-white rounded-full shadow transition-transform ${
                    autostart ? 'translate-x-5' : 'translate-x-0'
                  }`}
                />
              </div>
            </label>

            <label className="flex items-center justify-between p-3 bg-apple-gray-50 rounded-xl cursor-pointer">
              <div className="flex-1 min-w-0 mr-3">
                <div className="flex items-center gap-2">
                  <Repeat className="w-4 h-4 text-apple-gray-400" />
                  <span className="text-sm text-apple-gray-600">持续优化模式</span>
                </div>
                <p className="text-xs text-apple-gray-400 mt-0.5 ml-6">绑定后自动定期测速并切换至更快 IP</p>
              </div>
              <div
                className={`w-11 h-6 rounded-full p-0.5 transition-colors flex-shrink-0 ${
                  continuousModeLoading ? 'opacity-50 cursor-wait' : ''
                } ${continuousMode ? 'bg-apple-green' : 'bg-apple-gray-300'}`}
                onClick={() => !continuousModeLoading && updateContinuousMode(!continuousMode)}
              >
                <div
                  className={`w-5 h-5 bg-white rounded-full shadow transition-transform ${
                    continuousMode ? 'translate-x-5' : 'translate-x-0'
                  }`}
                />
              </div>
            </label>

          </div>
        </Section>

        {/* Advanced */}
        <Section icon={<FileText className="w-5 h-5" />} title="高级">
          <div className="space-y-3">
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
                disabled={checkingUpdate || updatePhase !== 'idle'}
                className="flex items-center gap-1.5 px-3 py-1.5 bg-apple-blue text-white text-sm font-medium rounded-xl hover:bg-apple-blue/90 transition-colors flex-shrink-0 disabled:opacity-50"
              >
                <RefreshCw className={`w-4 h-4 ${checkingUpdate ? 'animate-spin' : ''}`} />
                {checkingUpdate ? '检查中...' : '检查更新'}
              </button>
            </div>

            {/* 更新代理设置 */}
            <div className="p-3 bg-apple-gray-50 rounded-xl space-y-2.5">
              <div className="flex items-center gap-2">
                <Globe className="w-4 h-4 text-apple-gray-400" />
                <span className="text-sm text-apple-gray-600">更新代理</span>
                {proxyLoading && <Loader2 className="w-3 h-3 animate-spin text-apple-gray-400" />}
              </div>
              <p className="text-xs text-apple-gray-400 ml-6">检查更新时使用的网络代理</p>

              <div className="ml-6 space-y-2">
                {/* 自动检测 */}
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="radio"
                    name="update_proxy"
                    checked={updateProxy === 'auto'}
                    onChange={() => saveProxySetting('auto')}
                    disabled={proxyLoading}
                    className="w-3.5 h-3.5 accent-apple-blue"
                  />
                  <span className="text-sm text-apple-gray-600">自动检测</span>
                  {updateProxy === 'auto' && detectedProxy && (
                    <span className="text-xs text-apple-gray-400">({detectedProxy})</span>
                  )}
                  {updateProxy === 'auto' && !detectedProxy && (
                    <span className="text-xs text-apple-gray-400">(未检测到代理)</span>
                  )}
                </label>

                {/* 不使用代理 */}
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="radio"
                    name="update_proxy"
                    checked={updateProxy === ''}
                    onChange={() => saveProxySetting('')}
                    disabled={proxyLoading}
                    className="w-3.5 h-3.5 accent-apple-blue"
                  />
                  <span className="text-sm text-apple-gray-600">直连（不使用代理）</span>
                </label>

                {/* 手动指定 */}
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="radio"
                    name="update_proxy"
                    checked={updateProxy !== 'auto' && updateProxy !== ''}
                    onChange={() => {
                      const val = customProxy || 'http://127.0.0.1:7890'
                      setCustomProxy(val)
                      saveProxySetting(val)
                    }}
                    disabled={proxyLoading}
                    className="w-3.5 h-3.5 accent-apple-blue"
                  />
                  <span className="text-sm text-apple-gray-600">手动指定</span>
                </label>

                {/* 手动输入框 + 预设 */}
                {updateProxy !== 'auto' && updateProxy !== '' && (
                  <div className="ml-5.5 space-y-2">
                    <input
                      type="text"
                      value={customProxy}
                      onChange={(e) => setCustomProxy(e.target.value)}
                      onBlur={() => {
                        if (customProxy && customProxy !== updateProxy) {
                          saveProxySetting(customProxy)
                        }
                      }}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter' && customProxy) {
                          saveProxySetting(customProxy)
                        }
                      }}
                      placeholder="http://127.0.0.1:7890"
                      className="w-full px-3 py-1.5 text-sm bg-white border border-apple-gray-200 rounded-lg text-apple-gray-600 placeholder-apple-gray-300 focus:outline-none focus:border-apple-blue focus:ring-1 focus:ring-apple-blue/30"
                    />
                    <div className="flex flex-wrap gap-1.5">
                      {[
                        { label: 'Clash', value: 'http://127.0.0.1:7890' },
                        { label: 'V2Ray', value: 'http://127.0.0.1:10809' },
                        { label: 'SS', value: 'http://127.0.0.1:1080' },
                        { label: 'Surge', value: 'http://127.0.0.1:6152' },
                      ].map((preset) => (
                        <button
                          key={preset.label}
                          onClick={() => {
                            setCustomProxy(preset.value)
                            saveProxySetting(preset.value)
                          }}
                          disabled={proxyLoading}
                          className={`px-2 py-0.5 text-xs rounded-md transition-colors ${
                            customProxy === preset.value
                              ? 'bg-apple-blue text-white'
                              : 'bg-apple-gray-200 text-apple-gray-500 hover:bg-apple-gray-300'
                          }`}
                        >
                          {preset.label}
                        </button>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            </div>

            {/* GitHub 仓库 */}
            <div className="flex items-center justify-between p-3 bg-apple-gray-50 rounded-xl">
              <div className="flex items-center gap-2.5 flex-1 min-w-0 mr-3">
                <Github className="w-4 h-4 text-apple-gray-500 flex-shrink-0" />
                <div>
                  <span className="text-sm text-apple-gray-600">GitHub 仓库</span>
                  <p className="text-xs text-apple-gray-400 mt-0.5">wangwingzero/anyFAST</p>
                </div>
              </div>
              <div className="flex items-center gap-2 flex-shrink-0">
                <button
                  onClick={() => open('https://github.com/wangwingzero/anyFAST/issues').catch(console.error)}
                  className="flex items-center gap-1.5 px-2.5 py-1.5 text-apple-gray-500 text-xs font-medium rounded-lg hover:bg-apple-gray-200 transition-colors"
                >
                  <AlertCircle className="w-3.5 h-3.5" />
                  Issue
                </button>
                <button
                  onClick={() => open('https://github.com/wangwingzero/anyFAST').catch(console.error)}
                  className="flex items-center gap-1.5 px-3 py-1.5 bg-apple-gray-800 text-white text-xs font-medium rounded-lg hover:bg-apple-gray-700 transition-colors"
                >
                  <Star className="w-3.5 h-3.5" />
                  Star
                </button>
              </div>
            </div>

            {/* 更新日志按钮 + 查看器 */}
            {updateLogs.length > 0 && (
              <>
                <div className="flex justify-end">
                  <button
                    onClick={() => setShowUpdateLog(v => !v)}
                    className="flex items-center gap-1.5 px-2.5 py-1 text-xs text-apple-gray-400 hover:text-apple-gray-600 transition-colors"
                  >
                    <FileText className="w-3.5 h-3.5" />
                    {showUpdateLog ? '隐藏日志' : '查看更新日志'}
                  </button>
                </div>
                {showUpdateLog && (
                  <div
                    ref={logRef}
                    className="p-3 bg-apple-gray-800 rounded-xl max-h-48 overflow-y-auto font-mono text-xs text-apple-gray-300 space-y-0.5 select-text"
                  >
                    {updateLogs.map((line, i) => (
                      <div key={i} className={line.includes('失败') || line.includes('error') ? 'text-red-400' : ''}>{line}</div>
                    ))}
                  </div>
                )}
              </>
            )}

            {/* 更新错误 */}
            {updateError && (
              <div className="p-3 bg-red-50 border border-red-200 rounded-xl">
                <p className="text-sm text-red-600 mb-2">更新失败: {updateError}</p>
                {(() => {
                  const hint = getUpdateErrorHint(updateError)
                  return hint && <p className="text-xs text-red-500 mb-2">{hint}</p>
                })()}
                <div className="flex items-center gap-2">
                  <button
                    onClick={forceDownload}
                    disabled={forceDownloading}
                    className="flex items-center gap-1.5 px-3 py-1.5 bg-apple-green text-white text-sm font-medium rounded-xl hover:bg-apple-green/90 transition-colors disabled:opacity-50"
                  >
                    <Download className={`w-4 h-4 ${forceDownloading ? 'animate-bounce' : ''}`} />
                    {forceDownloading ? '下载中...' : '直接下载安装'}
                  </button>
                  <button
                    onClick={runDiagnostic}
                    disabled={diagnosing}
                    className="flex items-center gap-1.5 px-2.5 py-1 text-xs bg-red-100 text-red-700 rounded-lg hover:bg-red-200 transition-colors disabled:opacity-50"
                  >
                    <Search className={`w-3 h-3 ${diagnosing ? 'animate-spin' : ''}`} />
                    {diagnosing ? '排查中...' : '排查原因'}
                  </button>
                  <button
                    onClick={openReleasePage}
                    className="flex items-center gap-1.5 px-2.5 py-1 text-xs text-red-600 hover:text-red-700 hover:underline transition-colors"
                  >
                    <ExternalLink className="w-3 h-3" />
                    前往 GitHub 手动下载
                  </button>
                </div>
              </div>
            )}

            {/* 排查诊断结果 */}
            {diagnosticSteps && (
              <div className="p-3 bg-apple-gray-50 border border-apple-gray-200 rounded-xl space-y-2">
                <div className="flex items-center gap-2 mb-2">
                  <Search className="w-4 h-4 text-apple-blue" />
                  <span className="text-sm font-medium text-apple-gray-600">更新排查结果</span>
                </div>
                {diagnosticSteps.map((step, i) => (
                  <div key={i} className="flex items-start gap-2 text-xs">
                    {step.status === 'ok' && <CheckCircle2 className="w-3.5 h-3.5 text-apple-green mt-0.5 flex-shrink-0" />}
                    {step.status === 'warn' && <AlertCircle className="w-3.5 h-3.5 text-apple-orange mt-0.5 flex-shrink-0" />}
                    {step.status === 'error' && <XCircle className="w-3.5 h-3.5 text-red-500 mt-0.5 flex-shrink-0" />}
                    <div>
                      <span className="font-medium text-apple-gray-600">{step.name}: </span>
                      <span className="text-apple-gray-500">{step.detail}</span>
                    </div>
                  </div>
                ))}
              </div>
            )}

            {/* 有更新可用 */}
            {updateChecked && updateRef.current && !updateError && (
              <div className="p-3 rounded-xl bg-apple-green/10 border border-apple-green/30">
                <div className="space-y-2">
                  <div className="flex items-center gap-2">
                    <Download className="w-4 h-4 text-apple-green" />
                    <span className="text-sm font-medium text-apple-green">发现新版本!</span>
                  </div>
                  <p className="text-sm text-apple-gray-600">
                    最新版本: <span className="font-medium">v{updateRef.current.version}</span>
                    {updateRef.current.date && (
                      <span className="text-apple-gray-400 ml-2">
                        ({new Date(updateRef.current.date).toLocaleDateString('zh-CN')})
                      </span>
                    )}
                  </p>
                  {updateRef.current.body && (
                    <p className="text-xs text-apple-gray-400 line-clamp-3 whitespace-pre-line">{updateRef.current.body}</p>
                  )}

                  {/* 下载进度 */}
                  {updatePhase !== 'idle' && (
                    <div className="mt-2 space-y-1.5">
                      {/* 进度条 */}
                      <div className="w-full bg-apple-gray-200 rounded-full h-2 overflow-hidden">
                        <div
                          className="h-full bg-apple-green rounded-full transition-all duration-300 ease-out"
                          style={{
                            width: updatePhase === 'downloading' && downloadTotal > 0
                              ? `${Math.min((downloadProgress / downloadTotal) * 100, 100)}%`
                              : updatePhase === 'installing' || updatePhase === 'restarting'
                              ? '100%'
                              : '0%'
                          }}
                        />
                      </div>
                      {/* 状态文字 */}
                      <div className="flex items-center justify-between text-xs text-apple-gray-400">
                        <div className="flex items-center gap-1.5">
                          {updatePhase === 'downloading' && (
                            <>
                              <Loader2 className="w-3 h-3 animate-spin" />
                              <span>正在下载...</span>
                            </>
                          )}
                          {updatePhase === 'installing' && (
                            <>
                              <Loader2 className="w-3 h-3 animate-spin" />
                              <span>正在安装...</span>
                            </>
                          )}
                          {updatePhase === 'restarting' && (
                            <>
                              <CheckCircle2 className="w-3 h-3 text-apple-green" />
                              <span className="text-apple-green">更新完成，正在重启...</span>
                            </>
                          )}
                        </div>
                        {updatePhase === 'downloading' && downloadTotal > 0 && (
                          <span>{formatBytes(downloadProgress)} / {formatBytes(downloadTotal)}</span>
                        )}
                      </div>
                    </div>
                  )}

                  {/* 操作按钮 */}
                  {updatePhase === 'idle' && (
                    <div className="flex gap-2 mt-2">
                      <button
                        onClick={performUpdate}
                        className="flex items-center gap-1.5 px-3 py-1.5 bg-apple-green text-white text-sm font-medium rounded-xl hover:bg-apple-green/90 transition-colors"
                      >
                        <Download className="w-4 h-4" />
                        立即更新
                      </button>
                      <button
                        onClick={forceDownload}
                        disabled={forceDownloading}
                        className="flex items-center gap-1.5 px-3 py-1.5 text-apple-gray-500 text-sm font-medium rounded-xl hover:bg-apple-gray-100 transition-colors disabled:opacity-50"
                      >
                        <Download className={`w-3.5 h-3.5 ${forceDownloading ? 'animate-bounce' : ''}`} />
                        {forceDownloading ? '下载中...' : '下载安装包'}
                      </button>
                    </div>
                  )}
                </div>
              </div>
            )}

            {/* 降级方案：Rust 侧检测到有更新（无法应用内更新，提供手动下载） */}
            {updateChecked && fallbackUpdateInfo && !updateRef.current && !updateError && (
              <div className="p-3 rounded-xl bg-apple-green/10 border border-apple-green/30">
                <div className="space-y-2">
                  <div className="flex items-center gap-2">
                    <Download className="w-4 h-4 text-apple-green" />
                    <span className="text-sm font-medium text-apple-green">发现新版本!</span>
                  </div>
                  <p className="text-sm text-apple-gray-600">
                    最新版本: <span className="font-medium">v{fallbackUpdateInfo.latestVersion}</span>
                    {fallbackUpdateInfo.publishedAt && (
                      <span className="text-apple-gray-400 ml-2">
                        ({new Date(fallbackUpdateInfo.publishedAt).toLocaleDateString('zh-CN')})
                      </span>
                    )}
                  </p>
                  {fallbackUpdateInfo.releaseNotes && (
                    <p className="text-xs text-apple-gray-400 line-clamp-3 whitespace-pre-line">{fallbackUpdateInfo.releaseNotes}</p>
                  )}
                  <p className="text-xs text-amber-600">应用内静默更新暂不可用，可直接下载安装包更新。</p>
                  <div className="flex gap-2 mt-2">
                    <button
                      onClick={forceDownload}
                      disabled={forceDownloading}
                      className="flex items-center gap-1.5 px-3 py-1.5 bg-apple-green text-white text-sm font-medium rounded-xl hover:bg-apple-green/90 transition-colors disabled:opacity-50"
                    >
                      <Download className={`w-4 h-4 ${forceDownloading ? 'animate-bounce' : ''}`} />
                      {forceDownloading ? '下载中...' : '直接下载安装'}
                    </button>
                    <button
                      onClick={() => open(fallbackUpdateInfo.releaseUrl).catch(console.error)}
                      className="flex items-center gap-1.5 px-3 py-1.5 text-apple-gray-500 text-sm font-medium rounded-xl hover:bg-apple-gray-100 transition-colors"
                    >
                      <ExternalLink className="w-4 h-4" />
                      前往 GitHub
                    </button>
                    <button
                      onClick={runDiagnostic}
                      disabled={diagnosing}
                      className="flex items-center gap-1.5 px-3 py-1.5 text-apple-gray-500 text-sm font-medium rounded-xl hover:bg-apple-gray-100 transition-colors disabled:opacity-50"
                    >
                      <Search className={`w-4 h-4 ${diagnosing ? 'animate-spin' : ''}`} />
                      {diagnosing ? '排查中...' : '排查原因'}
                    </button>
                  </div>
                </div>
              </div>
            )}

            {/* 已是最新版本 */}
            {updateChecked && !updateRef.current && !fallbackUpdateInfo && !updateError && (
              <div className="p-3 rounded-xl bg-apple-gray-50">
                <div className="flex items-center gap-2">
                  <CheckCircle2 className="w-4 h-4 text-apple-green" />
                  <span className="text-sm text-apple-gray-600">已是最新版本</span>
                </div>
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
