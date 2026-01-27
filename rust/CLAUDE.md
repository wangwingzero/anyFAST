[根目录](../CLAUDE.md) > **rust**

# Rust/Tauri 模块

> 基于 Tauri v2 + React 18 的跨平台中转站端点优选桌面应用

## 模块职责

提供高性能、低资源占用的桌面应用:
- **Rust 后端**: 端点测试、hosts 管理、配置持久化、管理员权限检测
- **React 前端**: Apple 风格 UI，响应式设计，日志系统
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
3. 设置 `AppState`（ConfigManager + 测试结果缓存）
4. 注册所有 `#[tauri::command]` 命令（10 个）
5. 创建 960x640 窗口，加载前端

## 对外接口

### Tauri Commands (IPC)

从 React 前端通过 `invoke` 调用:

| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `get_config` | - | `AppConfig` | 加载配置文件 |
| `save_config` | `config: AppConfig` | - | 保存配置 |
| `start_speed_test` | - | `EndpointResult[]` | 开始测速（并发） |
| `stop_speed_test` | - | - | 取消测速 |
| `apply_endpoint` | `domain, ip` | - | 绑定单个端点 |
| `apply_all_endpoints` | - | `u32` | 批量绑定所有成功端点 |
| `clear_all_bindings` | - | `u32` | 清除所有绑定 |
| `get_bindings` | - | `(domain, ip?)[]` | 获取当前绑定状态 |
| `get_binding_count` | - | `u32` | 获取已绑定数量 |
| `check_admin` | - | `bool` | 检查管理员权限 |

### 使用示例
```typescript
import { invoke } from '@tauri-apps/api/core'

// 获取配置
const config = await invoke<AppConfig>('get_config')

// 开始测速
const results = await invoke<EndpointResult[]>('start_speed_test')

// 一键应用所有可用端点
const count = await invoke<number>('apply_all_endpoints')
```

## 关键依赖与配置

### Rust 依赖 (Cargo.toml)
```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"
tokio = { version = "1", features = ["full"] }
tokio-rustls = "0.26"
rustls = { version = "0.23", features = ["std", "tls12"] }
webpki-roots = "0.26"
hickory-resolver = { version = "0.24", features = ["tokio-runtime"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
directories = "5"
thiserror = "2"

[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = ["Win32_Foundation", "Win32_System_Console", "Win32_UI_Shell", "Win32_Security"] }
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
    "tailwind-merge": "^2.3.0"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.0.0",
    "typescript": "^5.4.5",
    "vite": "^5.3.0",
    "tailwindcss": "^3.4.4"
  }
}
```

### 用户配置位置
- Windows: `%APPDATA%/com.anyrouter/fast/config.json`
- macOS: `~/Library/Application Support/com.anyrouter.fast/config.json`
- Linux: `~/.config/com.anyrouter.fast/config.json`

## 数据模型

### Endpoint
```rust
// Rust (models.rs)
pub struct Endpoint {
    pub name: String,       // 显示名称
    pub url: String,        // 完整 URL (https://example.com/v1)
    pub domain: String,     // 域名 (example.com)
    pub enabled: bool,      // 是否启用
}
```

### EndpointResult
```rust
// Rust (models.rs)
pub struct EndpointResult {
    pub endpoint: Endpoint,
    pub ip: String,         // 测试使用的 IP
    pub latency: f64,       // 延迟 (ms)
    pub ttfb: f64,          // TTFB (ms)
    pub success: bool,
    pub error: Option<String>,
}
```

### AppConfig
```rust
// Rust (models.rs)
pub struct AppConfig {
    pub mode: String,              // "manual" | "auto"
    pub check_interval: u64,       // 健康检查间隔（秒），默认 30
    pub slow_threshold: u32,       // 慢速阈值百分比，默认 50
    pub failure_threshold: u32,    // 连续失败次数，默认 3
    pub test_count: u32,           // 测试次数，默认 3
    pub minimize_to_tray: bool,    // 最小化到托盘，默认 true
    pub cloudflare_ips: Vec<String>,
    pub endpoints: Vec<Endpoint>,
}
```

### TypeScript 类型
```typescript
// src/types/index.ts
interface AppConfig {
  mode: 'manual' | 'auto'
  check_interval: number      // 健康检查间隔（秒）
  slow_threshold: number      // 慢速阈值（百分比）
  failure_threshold: number   // 连续失败次数阈值
  test_count: number
  minimize_to_tray: boolean
  cloudflare_ips: string[]
  endpoints: Endpoint[]
}

interface LogEntry {
  level: 'success' | 'info' | 'warning' | 'error'
  message: string
  timestamp: string
}
```

## 核心模块说明

### endpoint_tester.rs
- **DNS 解析**: 使用 hickory-resolver（带缓存，128 条）
- **Cloudflare 检测**: 根据 IP 前缀判断（104.16-27.x, 172.67.x, 162.159.x）
- **IP 优选**: 检测到 CF 时，测试 11 个默认 CF IP + 自定义 IP
- **并发控制**: 最多 8 个端点同时测试（Semaphore）
- **TLS 测试**: 使用 tokio-rustls + webpki-roots，5 秒超时
- **测试方法**: HEAD 请求，测量 TCP+TLS+HTTP 全链路延迟

### hosts_manager.rs
- **路径**: Windows `C:\Windows\System32\drivers\etc\hosts`，Unix `/etc/hosts`
- **标记**: 使用 `# AnyRouter` 标记自动绑定的行
- **BOM 处理**: 自动处理 UTF-8 BOM
- **批量操作**: `write_bindings_batch` 和 `clear_bindings_batch` 支持批量操作
- **DNS 刷新**: Windows 调用 `ipconfig /flushdns`，macOS 调用 `dscacheutil -flushcache`

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
├── index.css         # TailwindCSS + Apple 风格样式
├── types/index.ts    # TypeScript 类型定义
└── components/
    ├── index.ts      # 统一导出
    ├── Sidebar.tsx   # 侧边栏导航 + 权限状态
    ├── Dashboard.tsx # 仪表盘（测速 + 结果表格）
    ├── Settings.tsx  # 设置页（端点管理 + 参数配置）
    └── Logs.tsx      # 运行日志（带统计）
```

### UI 设计
- **设计语言**: Apple Human Interface Guidelines 风格
- **颜色系统**: 自定义 apple-gray, apple-blue, apple-green, apple-red, apple-orange
- **组件**: 玻璃态背景（backdrop-filter: blur）、圆角 12/16/20px、微交互动画
- **图标**: lucide-react

## 测试与质量

**当前状态**: 无自动化测试

**建议补充**:

Rust 后端:
```bash
cd src-tauri
cargo test
```

关键测试点:
- `endpoint_tester.rs`: 模拟 DNS 和 TLS 测试
- `hosts_manager.rs`: 文件操作测试（需要 mock 文件系统）
- `config.rs`: 配置序列化/反序列化测试

前端:
```bash
npm install vitest @testing-library/react jsdom -D
npm test
```

## 常见问题 (FAQ)

**Q: 需要管理员权限吗？**
A: 是的，修改 hosts 文件需要管理员权限。程序会检测权限并在侧边栏显示状态。

**Q: Cloudflare IP 优选是怎么工作的？**
A: 检测到端点 DNS 解析到 CF IP 时，会额外测试 11 个内置 CF IP + 用户自定义 IP，选择延迟最低的。

**Q: 如何添加自定义端点？**
A: 在设置页面输入 URL，程序会自动提取域名。

**Q: 配置文件在哪里？**
A: Windows: `%APPDATA%/com.anyrouter/fast/config.json`

**Q: Tauri v2 和 v1 的区别？**
A: v2 使用 `@tauri-apps/api/core` 替代 `@tauri-apps/api/tauri`，插件系统重构。

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
│   ├── index.css              # 全局样式
│   ├── types/
│   │   └── index.ts           # TypeScript 类型定义
│   └── components/
│       ├── index.ts           # 组件导出
│       ├── Dashboard.tsx      # 仪表盘组件
│       ├── Settings.tsx       # 设置组件
│       ├── Sidebar.tsx        # 侧边栏组件
│       └── Logs.tsx           # 日志组件
└── src-tauri/
    ├── Cargo.toml             # Rust 依赖
    ├── tauri.conf.json        # Tauri 配置
    ├── build.rs               # 构建脚本
    ├── icons/                 # 应用图标 (ico, png)
    └── src/
        ├── main.rs            # Rust 入口
        ├── lib.rs             # Tauri 命令注册（10 个命令）
        ├── models.rs          # 数据模型
        ├── config.rs          # 配置管理器
        ├── endpoint_tester.rs # 端点测试器（CF 优选）
        └── hosts_manager.rs   # hosts 文件管理
```

## 变更记录 (Changelog)

### 2026-01-28 05:06:36
- 更新文档：补充完整的 Tauri Commands 列表（10 个）
- 更新数据模型：修正 AppConfig 字段（check_interval, slow_threshold, failure_threshold）
- 补充前端依赖：添加 clsx, tailwind-merge, @tauri-apps/plugin-shell
- 补充 Logs.tsx 组件说明
- 补充核心模块详细说明（endpoint_tester, hosts_manager, config）
- 补充 FAQ 和配置文件路径

### 2026-01-28
- 初始化模块文档
