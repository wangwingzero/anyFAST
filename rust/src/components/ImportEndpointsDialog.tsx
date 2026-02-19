import { useState, useRef, useCallback } from 'react'
import { X, FileUp, ClipboardPaste, Upload, CheckSquare, Square, AlertCircle } from 'lucide-react'
import { Endpoint } from '../types'

interface ParsedEndpoint {
  name: string
  url: string
  domain: string
  checked: boolean
  exists: boolean
}

interface ImportEndpointsDialogProps {
  open: boolean
  onClose: () => void
  existingEndpoints: Endpoint[]
  onImport: (endpoints: Endpoint[]) => void
}

const extractDomain = (url: string) => url.replace(/^https?:\/\//, '').split('/')[0]

/** 解析 All API Hub 备份 JSON，提取站点列表 */
function parseAllApiHubJson(json: unknown): { name: string; url: string; disabled?: boolean }[] {
  if (!json || typeof json !== 'object') return []

  const obj = json as Record<string, unknown>

  // v2 全量备份: { version: "2.0", accounts: { accounts: [...] } }
  // v2 仅账号: { version: "2.0", type: "accounts", accounts: { accounts: [...] } }
  if (obj.accounts && typeof obj.accounts === 'object') {
    const accountsWrapper = obj.accounts as Record<string, unknown>
    if (Array.isArray(accountsWrapper.accounts)) {
      return accountsWrapper.accounts
        .filter((a: unknown) => a && typeof a === 'object' && (a as Record<string, unknown>).site_url)
        .map((a: unknown) => {
          const acc = a as Record<string, unknown>
          return {
            name: (acc.site_name as string) || '',
            url: (acc.site_url as string) || '',
            disabled: acc.disabled as boolean | undefined,
          }
        })
    }
  }

  // v1 兼容: 直接是 accounts 数组
  if (Array.isArray(obj)) {
    return (obj as unknown[])
      .filter((a: unknown) => a && typeof a === 'object' && (a as Record<string, unknown>).site_url)
      .map((a: unknown) => {
        const acc = a as Record<string, unknown>
        return {
          name: (acc.site_name as string) || '',
          url: (acc.site_url as string) || '',
          disabled: acc.disabled as boolean | undefined,
        }
      })
  }

  return []
}

/** 解析多行文本为 URL 列表 */
function parseTextUrls(text: string): string[] {
  return text
    .split(/[\n,;]+/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0 && /^https?:\/\/.+/.test(line))
}

export function ImportEndpointsDialog({
  open,
  onClose,
  existingEndpoints,
  onImport,
}: ImportEndpointsDialogProps) {
  const [activeTab, setActiveTab] = useState<'file' | 'text'>('file')
  const [parsed, setParsed] = useState<ParsedEndpoint[]>([])
  const [error, setError] = useState('')
  const [fileName, setFileName] = useState('')
  const [pasteText, setPasteText] = useState('')
  const fileInputRef = useRef<HTMLInputElement>(null)

  const existingDomains = new Set(existingEndpoints.map((e) => e.domain))

  const toParsedList = useCallback(
    (items: { name: string; url: string }[]): ParsedEndpoint[] => {
      // 按 domain 去重（导入数据内部去重）
      const seen = new Set<string>()
      return items
        .map((item) => {
          const domain = extractDomain(item.url)
          if (seen.has(domain)) return null
          seen.add(domain)
          const exists = existingDomains.has(domain)
          return {
            name: item.name || domain,
            url: item.url,
            domain,
            checked: !exists,
            exists,
          }
        })
        .filter((x): x is ParsedEndpoint => x !== null)
    },
    [existingDomains],
  )

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file) return

    setFileName(file.name)
    setError('')

    const reader = new FileReader()
    reader.onload = (ev) => {
      try {
        const json = JSON.parse(ev.target?.result as string)
        const sites = parseAllApiHubJson(json).filter((s) => !s.disabled)
        if (sites.length === 0) {
          setError('未在文件中发现可导入的站点')
          setParsed([])
          return
        }
        setParsed(toParsedList(sites))
      } catch {
        setError('JSON 解析失败，请检查文件格式')
        setParsed([])
      }
    }
    reader.readAsText(file)
  }

  const handleParseText = () => {
    setError('')
    const urls = parseTextUrls(pasteText)
    if (urls.length === 0) {
      setError('未发现有效的 URL（需以 http:// 或 https:// 开头）')
      setParsed([])
      return
    }
    const items = urls.map((url) => ({ name: extractDomain(url), url }))
    setParsed(toParsedList(items))
  }

  const toggleItem = (index: number) => {
    setParsed((prev) => prev.map((item, i) => (i === index ? { ...item, checked: !item.checked } : item)))
  }

  const checkedCount = parsed.filter((p) => p.checked).length
  const allChecked = parsed.length > 0 && checkedCount === parsed.length
  const noneChecked = checkedCount === 0

  const toggleAll = () => {
    const newChecked = !allChecked
    setParsed((prev) => prev.map((item) => ({ ...item, checked: newChecked })))
  }

  const handleImport = () => {
    const selected = parsed.filter((p) => p.checked)
    if (selected.length === 0) return
    onImport(
      selected.map((p) => ({
        name: p.name,
        url: p.url,
        domain: p.domain,
        enabled: true,
      })),
    )
  }

  const resetState = () => {
    setParsed([])
    setError('')
    setFileName('')
    setPasteText('')
    if (fileInputRef.current) fileInputRef.current.value = ''
  }

  const handleClose = () => {
    resetState()
    onClose()
  }

  const switchTab = (tab: 'file' | 'text') => {
    setActiveTab(tab)
    setParsed([])
    setError('')
  }

  if (!open) return null

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
      <div className="bg-white rounded-2xl shadow-2xl p-5 max-w-lg w-full mx-4 max-h-[85vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <Upload className="w-5 h-5 text-apple-blue" />
            <h3 className="text-base font-semibold text-apple-gray-600">批量导入端点</h3>
          </div>
          <button
            onClick={handleClose}
            className="p-1 hover:bg-apple-gray-100 rounded-lg transition-colors"
          >
            <X className="w-4 h-4 text-apple-gray-400" />
          </button>
        </div>

        {/* Tabs */}
        <div className="flex border-b border-apple-gray-200 mb-4">
          <button
            onClick={() => switchTab('file')}
            className={`flex items-center gap-1.5 px-4 py-2 text-sm font-medium transition-colors border-b-2 -mb-px ${
              activeTab === 'file'
                ? 'text-apple-blue border-apple-blue'
                : 'text-apple-gray-400 border-transparent hover:text-apple-gray-600'
            }`}
          >
            <FileUp className="w-4 h-4" />
            文件导入
          </button>
          <button
            onClick={() => switchTab('text')}
            className={`flex items-center gap-1.5 px-4 py-2 text-sm font-medium transition-colors border-b-2 -mb-px ${
              activeTab === 'text'
                ? 'text-apple-blue border-apple-blue'
                : 'text-apple-gray-400 border-transparent hover:text-apple-gray-600'
            }`}
          >
            <ClipboardPaste className="w-4 h-4" />
            文本粘贴
          </button>
        </div>

        {/* Tab Content */}
        <div className="flex-1 overflow-hidden flex flex-col min-h-0">
          {activeTab === 'file' && (
            <div className="mb-3">
              <p className="text-xs text-apple-gray-400 mb-2">
                选择 All API Hub 导出的备份 JSON 文件
              </p>
              <div className="flex items-center gap-2">
                <button
                  onClick={() => fileInputRef.current?.click()}
                  className="flex items-center gap-1.5 px-3 py-2 text-sm font-medium rounded-xl bg-apple-blue/10 text-apple-blue hover:bg-apple-blue/20 transition-colors"
                >
                  <FileUp className="w-4 h-4" />
                  选择文件
                </button>
                {fileName && (
                  <span className="text-xs text-apple-gray-400 truncate">{fileName}</span>
                )}
                <input
                  ref={fileInputRef}
                  type="file"
                  accept=".json"
                  onChange={handleFileChange}
                  className="hidden"
                />
              </div>
            </div>
          )}

          {activeTab === 'text' && (
            <div className="mb-3">
              <p className="text-xs text-apple-gray-400 mb-2">
                粘贴 URL 列表，每行一个（支持逗号、分号分隔）
              </p>
              <textarea
                value={pasteText}
                onChange={(e) => setPasteText(e.target.value)}
                placeholder={'https://api.example1.com\nhttps://api.example2.com'}
                rows={4}
                className="w-full px-3 py-2 text-sm font-mono bg-apple-gray-50 border border-apple-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-apple-blue/30 resize-y mb-2"
              />
              <button
                onClick={handleParseText}
                disabled={!pasteText.trim()}
                className="px-3 py-1.5 text-xs font-medium rounded-lg bg-apple-blue/10 text-apple-blue hover:bg-apple-blue/20 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                解析
              </button>
            </div>
          )}

          {/* Error */}
          {error && (
            <div className="flex items-center gap-2 px-3 py-2 mb-3 bg-apple-red/5 border border-apple-red/20 rounded-xl">
              <AlertCircle className="w-4 h-4 text-apple-red flex-shrink-0" />
              <span className="text-xs text-apple-red">{error}</span>
            </div>
          )}

          {/* Preview List */}
          {parsed.length > 0 && (
            <div className="flex-1 flex flex-col min-h-0">
              <div className="flex items-center justify-between mb-2">
                <span className="text-xs text-apple-gray-400">
                  发现 {parsed.length} 个站点，已选 {checkedCount} 个
                </span>
                <button
                  onClick={toggleAll}
                  className="flex items-center gap-1 text-xs text-apple-blue hover:text-apple-blue/80 transition-colors"
                >
                  {allChecked ? (
                    <CheckSquare className="w-3.5 h-3.5" />
                  ) : (
                    <Square className="w-3.5 h-3.5" />
                  )}
                  {allChecked ? '取消全选' : '全选'}
                </button>
              </div>

              <div className="flex-1 overflow-y-auto max-h-[300px] border border-apple-gray-200 rounded-xl">
                {parsed.map((item, index) => (
                  <label
                    key={item.domain}
                    className={`flex items-center gap-3 px-3 py-2.5 cursor-pointer transition-colors border-b border-apple-gray-100 last:border-0 ${
                      item.checked ? 'bg-apple-blue/5' : 'hover:bg-apple-gray-50'
                    }`}
                  >
                    <input
                      type="checkbox"
                      checked={item.checked}
                      onChange={() => toggleItem(index)}
                      className="w-4 h-4 rounded text-apple-blue accent-apple-blue"
                    />
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-medium text-apple-gray-600 truncate">
                          {item.name}
                        </span>
                        {item.exists && (
                          <span className="flex-shrink-0 px-1.5 py-0.5 text-[10px] font-medium rounded bg-apple-orange/10 text-apple-orange">
                            已存在
                          </span>
                        )}
                      </div>
                      <span className="text-xs text-apple-gray-400 font-mono truncate block">
                        {item.url}
                      </span>
                    </div>
                  </label>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-2 mt-4 pt-3 border-t border-apple-gray-100">
          <button
            onClick={handleClose}
            className="px-4 py-2 text-sm font-medium rounded-xl bg-apple-gray-100 text-apple-gray-600 hover:bg-apple-gray-200 transition-colors"
          >
            取消
          </button>
          <button
            onClick={handleImport}
            disabled={noneChecked || parsed.length === 0}
            className="px-4 py-2 text-sm font-medium rounded-xl bg-apple-blue text-white hover:bg-apple-blue/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {checkedCount > 0 ? `导入 ${checkedCount} 个端点` : '导入'}
          </button>
        </div>
      </div>
    </div>
  )
}
