# anyFAST

> Relay Endpoint Optimizer — 中转站端点优选工具

![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue)
![Rust](https://img.shields.io/badge/rust-1.75+-orange)
![Tauri](https://img.shields.io/badge/tauri-2.0-purple)
![License](https://img.shields.io/badge/license-MIT-green)

同一个中转站域名背后可能有几十个 IP，延迟参差不齐。anyFAST 帮你自动找出最快的 IP，写入 hosts 绑定，后台持续守护，变慢自动切换。

## 核心特性

**一键启动** — 测速 → 绑定 → 守护，全自动完成

- **并发测速** — 同时测试多个端点的多个候选 IP，DNS + TLS + HTTP 延迟一体化测量，秒级出结果
- **Cloudflare IP 优选** — 自动扫描 CF CDN 优质 IP 段，支持在线拉取最新优选 IP 列表
- **智能绑定** — 自动将最快 IP 写入系统 hosts 文件；当前 IP 仍然够快时不切换，避免频繁抖动
- **持续守护** — 后台健康检查周期性探活已绑定端点，失败或严重变慢时自动触发全量优选并切换
- **反限流策略** — 分批测速 + 请求错开 + 自适应降级，有效规避运营商 QoS 和 CF Rate Limiting
- **批量导入** — 支持导入 [All API Hub](https://github.com/qixing-jk/all-api-hub) 备份文件，或直接粘贴多行 URL
- **单端点操作** — 对单个端点独立测速、绑定或解绑
- **历史统计** — 记录每次优化效果，查看累计加速数据
- **实时日志** — 测速进度、CF 风控检测、自动切换事件全程可视
- **系统集成** — 托盘常驻、开机自启、应用内更新

## 下载安装

前往 [GitHub Releases](https://github.com/wangwingzero/anyFAST/releases/latest) 下载：

| 平台 | 文件 |
|------|------|
| Windows x64 | `anyFAST_x.x.x_x64-setup.exe` |
| macOS Intel | `anyFAST_x.x.x_x64.dmg` |
| macOS Apple Silicon | `anyFAST_x.x.x_aarch64.dmg` |
| Linux x64 | `anyFAST_x.x.x_amd64.deb` / `.AppImage` |

### Windows

安装后直接运行。安装包已集成 Windows Service，用于以系统权限写入 hosts 文件。

### macOS

> **首次打开提示"已损坏"或"无法验证开发者"？**
>
> 打开终端执行：
>
> ```bash
> sudo xattr -rd com.apple.quarantine /Applications/anyFAST.app
> ```

首次启动会提示安装 Helper（setuid 权限辅助程序），点击安装并输入系统密码即可。只需一次，之后无感使用。

## 使用方法

1. 启动 anyFAST，在设置页添加你使用的中转站端点
2. 点击「启动」按钮
3. 等待测速完成，自动绑定最优 IP
4. 后台持续守护 — IP 变慢或失效时自动切换
5. 点击「停止」可停止守护并清除所有绑定

也可以在 Dashboard 中对单个端点进行独立测速、绑定或解绑。

### 批量导入端点

在设置页端点列表区域点击「导入」按钮，支持两种方式：

- **文件导入** — 选择 All API Hub 导出的备份 JSON 文件，自动识别站点列表
- **文本粘贴** — 直接粘贴多行 URL（支持换行、逗号、分号分隔）

导入前可预览、勾选需要的站点，已存在的端点会自动标记避免重复。

## 反限流机制

直连 CF IP 进行高并发测速容易触发运营商 QoS 限流和 CF Rate Limiting（429）。anyFAST v2.4.0 引入了完整的反限流策略：

| 维度 | 机制 |
|------|------|
| **分批测速** | 候选 IP 不再一次性全部发起，而是分批 spawn + 批间冷却 |
| **请求错开** | 批内每个请求错开 200ms + 随机抖动，模拟自然请求分布 |
| **自适应降级** | 检测到 CF 429 → 并发减半 + 间隔加倍，自动降级 |
| **可恢复冷却** | CF 限流不再永久跳过，改为 60 秒冷却期后恢复 |
| **提前结束** | 已找到延迟 < 原始 70% 的结果时立即停止，减少不必要请求 |
| **三档策略** | 保守 / 标准 / 激进三档可调，适应不同网络环境 |

## 权限说明

写入 hosts 文件需要系统特权，不同平台策略不同：

- **Windows** — 通过 named pipe 与 anyfast-service（Windows 系统服务）通信；若服务不可用则降级为直接操作（需管理员权限）
- **macOS** — 通过 setuid helper 程序操作；首次运行通过 osascript 提权安装

## 架构

```
┌──────────────────────────────────────┐
│            React + TypeScript        │  前端 UI
│     TailwindCSS · Recharts · Vite    │
├──────────────────────────────────────┤
│          Tauri IPC (invoke)          │  前后端桥接
├──────────────────────────────────────┤
│              Rust Backend            │  后端核心
│  ┌────────────┐  ┌────────────────┐  │
│  │ Endpoint   │  │ Health         │  │
│  │ Tester     │  │ Checker        │  │
│  │ (测速引擎) │  │ (持续守护)     │  │
│  └────────────┘  └────────────────┘  │
│  ┌────────────┐  ┌────────────────┐  │
│  │ Hosts      │  │ Config /       │  │
│  │ Manager    │  │ History        │  │
│  │ (hosts读写)│  │ (配置/历史)    │  │
│  └────────────┘  └────────────────┘  │
├──────────────────────────────────────┤
│  Windows Service / macOS Helper      │  权限提升
└──────────────────────────────────────┘
```

### 后端模块

| 模块 | 职责 |
|------|------|
| `endpoint_tester.rs` | 核心测速引擎：分批并发测速、CF IP 优选、DNS+TLS+延迟测量、反限流策略 |
| `health_checker.rs` | 后台健康监控：轻量探活 → 劣化检测 → 全量优选 → 自动切换 |
| `hosts_manager.rs` | Hosts 文件读写、绑定管理 |
| `hosts_ops.rs` | OS 特定的 hosts 操作，Windows Service / macOS Helper 降级策略 |
| `models.rs` | 数据结构：Endpoint, EndpointResult, AppConfig, TestStrategy 等 |
| `config.rs` | JSON 配置持久化（OS app data 目录） |
| `history.rs` | 历史记录管理 |
| `service/` | Windows Service（named pipe 通信） |
| `client/pipe_client.rs` | Named pipe 客户端（GUI 侧） |

### 测速引擎工作流

```
test_all(endpoints)
  │
  ├── 端点间冷却（500ms）
  │
  └── test_endpoint(endpoint)
        │
        ├── DNS 解析 → 获取原始 IP
        ├── 测试原始 IP → 基准延迟
        ├── 收集候选 IP（用户白名单 > CF优选 > DNS多解析）
        │
        └── 分批测试候选 IP
              ├── batch 1: [ip1, ip2, ip3] (批内错开 200ms+jitter)
              ├── cooldown 500ms
              ├── batch 2: [ip4, ip5, ip6]
              ├── ...
              ├── 限流检测 → 降级（并发减半+间隔加倍）
              └── 提前结束（找到足够好的结果）
```

## 开发

```bash
git clone https://github.com/wangwingzero/anyFAST.git
cd anyFAST/rust

npm install
npm run tauri dev    # 开发模式（热重载）
npm run tauri build  # 生产构建
```

### 测试

```bash
# 前端测试
cd rust && npm test

# Rust 后端测试
cd rust/src-tauri && cargo test --no-default-features --verbose

# TypeScript 类型检查
cd rust && npx tsc --noEmit

# Rust lint
cd rust/src-tauri && cargo clippy --all-targets --all-features -- -D warnings
```

### 环境要求

- Node.js >= 18
- Rust >= 1.75
- Windows 10/11 / macOS 12+ / Linux

## 技术栈

| 层 | 技术 |
|----|------|
| 框架 | Tauri 2.0 |
| 后端 | Rust, Tokio, Reqwest, Hickory DNS, Native-TLS |
| 前端 | React 18, TypeScript, TailwindCSS, Recharts, Vite |
| 测试 | Vitest + React Testing Library, Cargo Test |
| CI/CD | GitHub Actions（多平台构建、签名、公证、自动发版） |

安装包 ~10MB，运行内存 ~30MB。

## 许可证

MIT License

---

**GitHub**: https://github.com/wangwingzero/anyFAST
