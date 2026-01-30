import { useState, useEffect, useRef } from 'react'
import { RotateCcw, Power, FileText, ExternalLink, RefreshCw, Download, Info, PlayCircle } from 'lucide-react'
import { Endpoint, AppConfig, UpdateInfo } from '../types'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-shell'

// 默认端点（与后端 models.rs 保持一致）
const DEFAULT_ENDPOINTS: Endpoint[] = [
  {
    name: 'anyrouter大善人',
    url: 'https://betterclau.de/claude/anyrouter.top',
    domain: 'betterclau.de',
    enabled: true,
  },
  {
    name: 'L站WONG大佬',
    url: 'https://wzw.pp.ua/v1',
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
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null)
  const [checkingUpdate, setCheckingUpdate] = useState(false)
  const [updateError, setUpdateError] = useState<string | null>(null)
  const [currentVersion, setCurrentVersion] = useState<string>('')

  // 自启动状态
  const [autostart, setAutostart] = useState(config?.autostart ?? false)
  const [autostartLoading, setAutostartLoading] = useState(false)

  const initializedRef = useRef(false)

  // 初始化
  useEffect(() => {
    if (config && !initializedRef.current) {
      initializedRef.current = true
    }
  }, [config])

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

  const restoreAllDefaults = async () => {
    onEndpointsChange(DEFAULT_ENDPOINTS)

    // 保存默认配置
    const newConfig: AppConfig = {
      endpoints: DEFAULT_ENDPOINTS,
      autostart: config?.autostart ?? false,  // 保持当前自启动设置
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

          </div>
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
                    
                    <div className="flex gap-2 mt-2">
                      <button
                        onClick={openReleasePage}
                        className="flex items-center gap-1.5 px-3 py-1.5 bg-apple-green text-white text-sm font-medium rounded-xl hover:bg-apple-green/90 transition-colors"
                      >
                        <Download className="w-4 h-4" />
                        前往下载
                      </button>
                    </div>
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
