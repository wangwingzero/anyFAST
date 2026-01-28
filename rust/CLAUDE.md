[根目录](../CLAUDE.md) > **rust**

# Rust/Tauri 模块

> 基于 Tauri v2 + React 18 的跨平台中转站端点优选桌面应用

## 变更记录 (Changelog)

### 2026-01-28 13:16:12
- **文档同步更新**
  - 确认 Tauri Commands 实际数量为 24 个
  - 补充更新检查命令: `check_for_update`, `get_current_version`
  - 补充权限提升命令: `restart_as_admin`
  - 补充 Service RPC 方法说明
  - 新增 UpdateInfo/PermissionStatus 数据模型说明

### 2026-01-28 08:00:00
- **新增 Windows Service 架构**
  - 添加 `anyfast-service` Windows 服务可执行文件
  - 新增 `service/` 模块：JSON-RPC over Named Pipe 服务端
  - 新增 `client/` 模块：Named Pipe 客户端
  - 新增 `hosts_ops.rs`：自动切换 Service/直接操作
  - 更新 app.manifest：从 requireAdministrator 改为 asInvoker
  - 新增 NSIS 安装钩子：自动注册/卸载 Service
  - 更新 Tauri Commands 至 24 个
  - Sidebar 显示 Service 模式状态

### 2026-01-28 06:48:54
- 更新自动模式（HealthChecker）完整说明
- 补充历史记录（HistoryManager）模块说明
- 更新 Tauri Commands 至 18 个
- 补充前端组件测试文件说明
- 更新数据模型字段（新增 close_to_tray, clear_on_exit 等）

### 2026-01-28 05:06:36
- 初始化模块文档

---

## 模块职责

提供高性能、低资源占用的桌面应用:
- **Rust 后端**: 端点测试、hosts 管理、配置持久化、自动模式健康检查、历史记录
- **React 前端**: Apple 风格 UI，响应式设计，日志系统，历史统计图表
- **Tauri v2 框架**: 跨平台打包，系统托盘集成

## 入口与启动

### 开发模式
```bash
npm install
npm run tauri dev
```

### 生产构建
```bash
npm run tauri build
# 输出: src-tauri/target/release/bundle/msi/ 和 nsis/
```

**启动流程**:
1. `src-tauri/src/main.rs` 调用 `anyfast_lib::run()`
2. `lib.rs` 初始化 Tauri Builder，注册 shell 插件
3. 设置 `AppState`（ConfigManager, HistoryManager, HealthChecker, 测试结果缓存）
4. 创建系统托盘菜单（显示窗口/退出）
5. 注册所有 `#[tauri::command]` 命令（24 个）
6. 如果配置为自动模式，延迟 2 秒启动健康检查
7. 创建 960x640 窗口，加载前端

## 对外接口

### Tauri Commands (IPC)

从 React 前端通过 `invoke` 调用:

#### 配置管理
| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `get_config` | - | `AppConfig` | 加载配置文件 |
| `save_config` | `config: AppConfig` | - | 保存配置 |

#### 测速与绑定
| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `start_speed_test` | - | `EndpointResult[]` | 开始测速（并发，60s 超时） |
| `stop_speed_test` | - | - | 取消测速 |
| `apply_endpoint` | `domain, ip` | - | 绑定单个端点 |
| `apply_all_endpoints` | - | `u32` | 批量绑定所有成功端点 |
| `clear_all_bindings` | - | `u32` | 清除所有绑定 |
| `get_bindings` | - | `(domain, ip?)[]` | 获取当前绑定状态 |
| `get_binding_count` | - | `u32` | 获取已绑定数量 |

#### 系统信息
| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `check_admin` | - | `bool` | 检查是否有写入权限 |
| `is_service_running` | - | `bool` | 检查 Service 是否运行 |
| `get_permission_status` | - | `(bool, bool)` | 获取权限状态 (has_permission, is_using_service) |
| `refresh_service_status` | - | `bool` | 刷新 Service 状态检测 |
| `get_hosts_path` | - | `String` | 获取 hosts 文件路径 |
| `open_hosts_file` | - | - | 用系统编辑器打开 hosts |

#### 历史记录
| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `get_history_stats` | `hours: u32` | `HistoryStats` | 获取历史统计（0=全部） |
| `clear_history` | - | - | 清空历史记录 |

#### 自动模式
| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `start_auto_mode` | - | - | 启动自动模式 |
| `stop_auto_mode` | - | - | 停止自动模式 |
| `get_auto_mode_status` | - | `HealthStatus` | 获取自动模式状态 |
| `is_auto_mode_running` | - | `bool` | 检查自动模式是否运行中 |

#### 更新检查
| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `check_for_update` | - | `UpdateInfo` | 检查 GitHub 最新版本 |
| `get_current_version` | - | `String` | 获取当前版本号 |

#### 权限管理
| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `restart_as_admin` | - | - | 以管理员权限重启（Windows） |

### 使用示例
```typescript
import { invoke } from '@tauri-apps/api/core'

// 获取配置
const config = await invoke<AppConfig>('get_config')

// 开始测速
const results = await invoke<EndpointResult[]>('start_speed_test')

// 一键应用所有可用端点
const count = await invoke<number>('apply_all_endpoints')

// 获取 24 小时历史统计
const stats = await invoke<HistoryStats>('get_history_stats', { hours: 24 })

// 启动自动模式
await invoke('start_auto_mode')
```

## 关键依赖与配置

### Rust 依赖 (Cargo.toml)
```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"
tokio = { version = "1", features = ["full"] }
tokio-util = "0.7"
tokio-rustls = "0.26"
rustls = { version = "0.23", features = ["std", "tls12"] }
webpki-roots = "0.26"
hickory-resolver = { version = "0.24", features = ["tokio-runtime"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
directories = "5"
thiserror = "2"
chrono = "0.4"
fs2 = "0.4"

[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = ["Win32_Foundation", "Win32_System_Console", "Win32_UI_Shell", "Win32_Security"] }

[dev-dependencies]
tempfile = "3"
tokio-test = "0.4"
```

### 前端依赖 (package.json)
```json
{
  "dependencies": {
    "react": "^18.3.1",
    "react-dom": "^18.3.1",
    "@tauri-apps/api": "^2.0.0",
    "@tauri-apps/plugin-shell": "^2.0.0",
    "lucide-react": "^0.400.0",
    "clsx": "^2.1.0",
    "tailwind-merge": "^2.3.0",
    "recharts": "^2.12.0"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.0.0",
    "typescript": "^5.4.5",
    "vite": "^5.3.0",
    "tailwindcss": "^3.4.4",
    "vitest": "^2.0.0",
    "@testing-library/react": "^16.0.0",
    "@testing-library/jest-dom": "^6.4.6",
    "jsdom": "^24.1.0"
  }
}
```

### 用户配置位置
- Windows: `%APPDATA%/com.anyrouter/fast/config.json`
- macOS: `~/Library/Application Support/com.anyrouter.fast/config.json`
- Linux: `~/.config/com.anyrouter.fast/config.json`

### 历史记录位置
- 同配置目录下的 `history.json`
- 自动保留 7 天，超过自动清理

## 数据模型

### Endpoint
```rust
pub struct Endpoint {
    pub name: String,       // 显示名称
    pub url: String,        // 完整 URL (https://example.com/v1)
    pub domain: String,     // 域名 (example.com)
    pub enabled: bool,      // 是否启用
}
```

### EndpointResult
```rust
pub struct EndpointResult {
    pub endpoint: Endpoint,
    pub ip: String,              // 测试使用的 IP（最优）
    pub latency: f64,            // 最终延迟 (ms)
    pub ttfb: f64,               // TTFB (ms)
    pub success: bool,
    pub error: Option<String>,
    // 加速效果相关
    pub original_ip: String,     // DNS 解析的原始 IP
    pub original_latency: f64,   // 原始 IP 延迟 (ms)
    pub speedup_percent: f64,    // 加速百分比
    pub use_original: bool,      // 是否使用原始 IP（优化无效时）
}
```

### AppConfig
```rust
pub struct AppConfig {
    pub mode: String,              // "manual" | "auto"
    pub check_interval: u64,       // 健康检查间隔（秒），默认 30
    pub slow_threshold: u32,       // 慢速阈值百分比，默认 50
    pub failure_threshold: u32,    // 连续失败次数，默认 3
    pub test_count: u32,           // 测试次数，默认 3
    pub minimize_to_tray: bool,    // 最小化时隐藏到托盘
    pub close_to_tray: bool,       // 关闭按钮最小化到托盘，默认 true
    pub clear_on_exit: bool,       // 退出时清除 hosts 绑定，默认 false
    pub cloudflare_ips: Vec<String>,
    pub endpoints: Vec<Endpoint>,
}
```

### UpdateInfo
```rust
pub struct UpdateInfo {
    pub current_version: String,   // 当前版本
    pub latest_version: String,    // 最新版本
    pub has_update: bool,          // 是否有更新
    pub release_url: String,       // Release 页面 URL
    pub release_notes: String,     // 更新说明
    pub published_at: String,      // 发布时间
}
```

### PermissionStatus
```rust
pub struct PermissionStatus {
    pub has_permission: bool,      // 是否有写入权限
    pub is_using_service: bool,    // 是否通过 Service
}
```

### HistoryRecord / HistoryStats
```rust
pub struct HistoryRecord {
    pub timestamp: i64,            // Unix 时间戳（秒）
    pub domain: String,
    pub original_latency: f64,     // 原始延迟 (ms)
    pub optimized_latency: f64,    // 优化后延迟 (ms)
    pub speedup_percent: f64,      // 加速百分比
    pub applied: bool,             // 是否应用了优化
}

pub struct HistoryStats {
    pub total_tests: u32,          // 总测试次数
    pub total_speedup_ms: f64,     // 累计节省时间 (ms)
    pub avg_speedup_percent: f64,  // 平均加速百分比
    pub records: Vec<HistoryRecord>, // 最近记录（最多 100 条）
}
```

### HealthStatus / EndpointHealth
```rust
pub struct HealthStatus {
    pub is_running: bool,
    pub last_check: Option<i64>,   // 上次检查时间戳
    pub check_count: u32,          // 总检查次数
    pub switch_count: u32,         // 总切换次数
    pub endpoints_status: Vec<EndpointHealth>,
}

pub struct EndpointHealth {
    pub domain: String,
    pub current_ip: Option<String>,
    pub latency: f64,
    pub baseline_latency: f64,     // 基准延迟
    pub consecutive_failures: u32, // 连续失败次数
    pub is_healthy: bool,
}
```

### TypeScript 类型
```typescript
// src/types/index.ts - 完整映射 Rust 模型
interface AppConfig {
  mode: 'manual' | 'auto'
  check_interval: number
  slow_threshold: number
  failure_threshold: number
  test_count: number
  minimize_to_tray: boolean
  close_to_tray: boolean
  clear_on_exit: boolean
  cloudflare_ips: string[]
  endpoints: Endpoint[]
}
```

## 核心模块说明

### endpoint_tester.rs
- **DNS 解析**: 使用 hickory-resolver（带缓存，128 条）
- **Cloudflare 检测**: 根据 IP 前缀判断（104.16-27.x, 172.67.x, 162.159.x）
- **IP 优选**: 检测到 CF 时，测试 11 个默认 CF IP + 自定义 IP
- **并发控制**: 最多 8 个端点同时测试（Semaphore）
- **超时保护**: 单 IP 测试 5s，单端点 15s，总测试 60s
- **智能回退**: 如果优化 IP 比原始 IP 慢，自动使用原始 IP
- **TLS 测试**: 使用 tokio-rustls + webpki-roots，不依赖系统证书
- **测试方法**: HEAD 请求，测量 TCP+TLS+HTTP 全链路延迟

### hosts_manager.rs
- **路径**: Windows `C:\Windows\System32\drivers\etc\hosts`，Unix `/etc/hosts`
- **文件锁**: 使用 fs2 的 exclusive lock 保证原子操作
- **BOM 处理**: 自动处理 UTF-8 BOM
- **输入验证**: 验证 IP 和域名格式，防止注入
- **批量操作**: `write_bindings_batch` 和 `clear_bindings_batch` 单次文件操作
- **DNS 刷新**: Windows `ipconfig /flushdns`，macOS `dscacheutil -flushcache`

### health_checker.rs
- **触发条件**:
  - 延迟比基准慢超过 `slow_threshold%` 时标记为不健康
  - 连续 `failure_threshold` 次不健康后触发切换
- **后台任务**: 使用 tokio::spawn，每 `check_interval` 秒检查一次
- **取消支持**: CancellationToken 支持优雅停止
- **事件通知**: 通过 `app_handle.emit()` 向前端发送状态变化

### history.rs
- **存储**: JSON 文件，使用 `directories` crate 获取跨平台路径
- **自动清理**: 保留 7 天（HISTORY_RETENTION_DAYS）
- **统计功能**: 支持按时间段（1h/24h/7d）获取统计数据
- **记录上限**: 返回最多 100 条最近记录

### hosts_ops.rs
- **统一接口**: 自动检测 Service 可用性，fallback 到直接操作
- **缓存状态**: 使用 `OnceLock<AtomicBool>` 缓存 Service 运行状态
- **自动降级**: Service 调用失败时自动标记不可用，切换到直接操作
- **权限检测**: `get_permission_status()` 返回 `(has_permission, is_using_service)`

### service/ 模块 (Windows only)
- **rpc.rs**: JSON-RPC 2.0 协议定义
  - 请求/响应类型、错误码、方法名常量
  - 支持方法: ping, write_binding, write_bindings_batch, clear_binding, clear_bindings_batch, read_binding, get_all_bindings, flush_dns
- **pipe_server.rs**: Named Pipe 服务端
  - Pipe 名称: `\\.\pipe\anyfast-hosts-service`
  - 以 SYSTEM 权限运行，处理来自 GUI 的请求

### client/ 模块 (Windows only)
- **pipe_client.rs**: Named Pipe 客户端
  - 连接超时: 5000ms
  - 自动生成请求 ID
  - 错误类型: ServiceNotRunning, ConnectionTimeout, Rpc, Io

### config.rs
- **存储**: 使用 `directories` crate 获取跨平台配置目录
- **格式**: JSON (pretty print)
- **默认值**: 提供完整默认配置，包含 2 个默认端点

## 前端架构

### 组件结构
```
src/
├── main.tsx          # React 入口
├── App.tsx           # 主应用（状态管理、IPC 调用）
├── App.test.tsx      # 主应用测试
├── index.css         # TailwindCSS + Apple 风格样式
├── types/index.ts    # TypeScript 类型定义
├── test/
│   ├── setup.ts      # Vitest 配置
│   └── mocks/tauri.ts # Tauri API 模拟
└── components/
    ├── index.ts      # 统一导出
    ├── Sidebar.tsx   # 侧边栏导航 + 权限状态
    ├── Dashboard.tsx # 仪表盘（测速 + 结果表格 + 加速效果）
    ├── Settings.tsx  # 设置页（端点管理 + 参数配置 + 自动模式）
    ├── Logs.tsx      # 运行日志（带统计 + 复制功能）
    ├── HistoryView.tsx # 历史统计（图表 + 记录表格）
    └── Toast.tsx     # Toast 通知组件
```

### UI 设计
- **设计语言**: Apple Human Interface Guidelines 风格
- **颜色系统**: 自定义 apple-gray, apple-blue, apple-green, apple-red, apple-orange
- **组件**: 玻璃态背景（backdrop-filter: blur）、圆角 12/16/20px、微交互动画
- **图标**: lucide-react
- **图表**: Recharts (LineChart)

### 前端状态管理
- 使用 React useState/useCallback 管理本地状态
- 通过 `invoke` 与后端同步数据
- Toast 通知系统用于操作反馈

## 测试与质量

### 前端测试
```bash
npm test              # 运行一次
npm run test:watch    # 监听模式
npm run test:coverage # 覆盖率报告
```

已有测试文件:
- `App.test.tsx`
- `Dashboard.test.tsx`
- `Settings.test.tsx`
- `Sidebar.test.tsx`
- `Logs.test.tsx`

### 后端测试
```bash
cd src-tauri
cargo test
```

各模块内置单元测试:
- `models.rs` - 数据模型创建、序列化、加速百分比计算
- `endpoint_tester.rs` - Cloudflare IP 检测、CF IP 列表生成
- `hosts_manager.rs` - 文件读写、批量操作、BOM 处理（使用 tempfile）
- `config.rs` - 配置加载/保存、默认值

## 常见问题 (FAQ)

**Q: 需要管理员权限吗？**
A: 是的，修改 hosts 文件需要管理员权限。程序会检测权限并在侧边栏显示状态。

**Q: Cloudflare IP 优选是怎么工作的？**
A: 检测到端点 DNS 解析到 CF IP 时，会额外测试 11 个内置 CF IP + 用户自定义 IP，选择延迟最低的。如果优化 IP 比原始 IP 慢，会自动使用原始 IP。

**Q: 自动模式如何工作？**
A: 每隔 `check_interval` 秒检查所有端点延迟，如果比基准慢超过 `slow_threshold%`，或连续失败 `failure_threshold` 次，自动切换到更优 IP。

**Q: 如何添加自定义端点？**
A: 在设置页面输入 URL，程序会自动提取域名。

**Q: 配置文件在哪里？**
A: Windows: `%APPDATA%/com.anyrouter/fast/config.json`

**Q: 历史记录保留多久？**
A: 7 天，超过自动清理。

**Q: Tauri v2 和 v1 的区别？**
A: v2 使用 `@tauri-apps/api/core` 替代 `@tauri-apps/api/tauri`，插件系统重构，托盘图标 API 改变。

## 相关文件清单

```
rust/
├── package.json               # 前端依赖与脚本
├── tsconfig.json              # TypeScript 配置
├── tsconfig.node.json         # Vite 配置的 TS
├── vite.config.ts             # Vite 构建配置
├── tailwind.config.js         # TailwindCSS 配置（Apple 风格）
├── postcss.config.js          # PostCSS 配置
├── index.html                 # HTML 模板
├── src/
│   ├── main.tsx               # React 入口
│   ├── App.tsx                # 主应用组件（状态管理）
│   ├── App.test.tsx           # 主应用测试
│   ├── index.css              # 全局样式
│   ├── types/
│   │   └── index.ts           # TypeScript 类型定义
│   ├── test/
│   │   ├── setup.ts           # Vitest 配置
│   │   └── mocks/tauri.ts     # Tauri API 模拟
│   └── components/
│       ├── index.ts           # 组件导出
│       ├── Dashboard.tsx      # 仪表盘组件
│       ├── Dashboard.test.tsx
│       ├── Settings.tsx       # 设置组件
│       ├── Settings.test.tsx
│       ├── Sidebar.tsx        # 侧边栏组件
│       ├── Sidebar.test.tsx
│       ├── Logs.tsx           # 日志组件
│       ├── Logs.test.tsx
│       ├── HistoryView.tsx    # 历史统计组件
│       └── Toast.tsx          # Toast 通知组件
└── src-tauri/
    ├── Cargo.toml             # Rust 依赖
    ├── tauri.conf.json        # Tauri 配置
    ├── build.rs               # 构建脚本
    ├── icons/                 # 应用图标 (ico, png)
    ├── capabilities/          # Tauri v2 能力配置
    └── src/
        ├── main.rs            # Rust 入口
        ├── lib.rs             # Tauri 命令注册（24 个命令）
        ├── models.rs          # 数据模型 + 单元测试
        ├── config.rs          # 配置管理器 + 测试
        ├── endpoint_tester.rs # 端点测试器（CF 优选）+ 测试
        ├── hosts_manager.rs   # hosts 文件管理 + 测试
        ├── health_checker.rs  # 自动模式健康检查
        ├── history.rs         # 历史记录管理
        ├── hosts_ops.rs       # hosts 操作包装器 (Service/直接 自动切换)
        ├── service/           # Windows Service 模块
        │   ├── mod.rs
        │   ├── rpc.rs         # JSON-RPC 2.0 协议定义
        │   └── pipe_server.rs # Named Pipe 服务端
        ├── client/            # Pipe 客户端模块
        │   ├── mod.rs
        │   └── pipe_client.rs # Named Pipe 客户端
        └── bin/
            └── anyfast-service.rs # Service 可执行文件入口
```
