# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 始终使用中文进行交流

## Project Overview

anyFAST 是一个中转站端点优选工具（Relay Endpoint Optimizer），基于 Tauri 2.0 构建的跨平台桌面应用。核心功能：并发测速多个中转站端点 → 选出最快 IP → 写入 hosts 文件绑定 → 后台健康监控自动切换。

## Build & Development Commands

所有前端命令在 `rust/` 目录下执行：

```bash
npm install                  # 安装前端依赖
npm run tauri dev            # 开发模式（热重载）
npm run tauri build          # 生产构建
```

### Testing

```bash
# 前端测试（Vitest + React Testing Library）
npm test                     # 运行一次
npm run test:watch           # 监听模式
npm run test:coverage        # 覆盖率报告

# Rust 后端测试
cd rust/src-tauri && cargo test --verbose
```

### Linting

```bash
# TypeScript 类型检查
cd rust && npx tsc --noEmit

# Rust 格式检查和 lint
cd rust/src-tauri && cargo fmt --all -- --check
cd rust/src-tauri && cargo clippy --all-targets --all-features -- -D warnings
```

## Architecture

### 目录结构

- `rust/src/` — React + TypeScript 前端
- `rust/src-tauri/src/` — Rust 后端（Tauri commands）
- `rust/src/types/index.ts` — 前端共享类型定义（集中管理）
- `rust/src/components/index.ts` — 组件导出集中管理

### 前端 → 后端通信

前端通过 Tauri 的 `invoke()` 调用后端 `#[tauri::command]` 函数。所有 Tauri commands 注册在 `rust/src-tauri/src/lib.rs` 的 `invoke_handler` 中。

### 后端核心模块（rust/src-tauri/src/）

| 模块 | 职责 |
|------|------|
| `lib.rs` | Tauri command handlers、AppState 定义、应用入口 |
| `models.rs` | 数据结构：Endpoint, EndpointResult, AppConfig, HistoryRecord 等 |
| `endpoint_tester.rs` | 核心测速逻辑：并发 HTTP(S) 测速、Cloudflare IP 优选、DNS+TLS+延迟测量 |
| `health_checker.rs` | 后台健康监控：检测慢/失败端点，自动切换更优 IP |
| `hosts_manager.rs` | Hosts 文件读写逻辑 |
| `hosts_ops.rs` | OS 特定的 hosts 操作，含 service/helper 降级策略 |
| `config.rs` | JSON 配置持久化（存储在 OS app data 目录） |
| `history.rs` | 历史记录管理 |
| `service/` | Windows Service（通过 named pipe 与 GUI 通信） |
| `client/pipe_client.rs` | Named pipe 客户端（GUI 侧） |
| `bin/anyfast-service.rs` | Windows Service 可执行文件 |
| `bin/anyfast-helper-macos.rs` | macOS setuid 权限 helper |

### 权限提升架构

Hosts 文件写入需要特权，不同平台策略不同：
- **Windows**: 优先通过 named pipe 与 anyfast-service（系统服务）通信；降级为直接操作（需管理员权限）
- **macOS**: 通过 setuid helper（`/usr/local/bin/anyfast-helper-macos`）；首次运行通过 osascript 安装

### 状态管理

- 后端：`AppState` 使用 `Arc<Mutex<T>>` 管理共享状态（tokio::sync::Mutex）
- 取消操作：使用 `CancellationToken` 模式（tokio_util）
- 前端：React useState + 自定义 hooks

### 主要工作流

1. **start_workflow**: 测速 → 智能应用（稳定性优先：当前 IP 仍可用则保持） → 启动健康检查
2. **stop_workflow**: 停止健康检查 → 清除所有 anyFAST hosts 绑定 → 刷新 DNS

## Conventions

- Commit 遵循 Conventional Commits（`feat:`, `fix:`, `ci:`）
- 前端测试文件命名 `*.test.tsx`，Rust 测试内联在模块中
- 涉及 Tauri/installer/平台特定代码的 PR 需注明影响的平台（Windows/macOS）
- 版本号在 `rust/src-tauri/tauri.conf.json` 中管理，`build.rs` 会自动设置 `APP_VERSION` 环境变量

## Environment Requirements

- Node.js >= 18
- Rust >= 1.75
- Windows 10/11 or macOS 12+
