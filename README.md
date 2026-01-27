# anyFAST

> 中转站端点优选工具 - 自动测速、智能切换、hosts 绑定

![Platform](https://img.shields.io/badge/platform-Windows-blue)
![Rust](https://img.shields.io/badge/rust-1.75+-orange)
![Tauri](https://img.shields.io/badge/tauri-2.0-purple)
![License](https://img.shields.io/badge/license-MIT-green)

anyFAST 是一款面向需要访问中转站服务的用户设计的桌面工具。它通过并发测试多个中转站端点的 HTTPS 延迟，自动选择最优 IP 并写入系统 hosts 文件，实现稳定低延迟的访问体验。

## 功能特性

- **端点测速** - 并发测试多个中转站端点的 HTTPS 响应延迟
- **Cloudflare 优选** - 自动识别 CF 站点，使用优选 IP 列表测试，选择最快 IP
- **加速效果对比** - 显示与原始 IP 的延迟对比和加速百分比
- **Hosts 绑定** - 一键应用到 Windows hosts 文件
- **自动模式** - 后台定时检测，自动切换到更快端点
- **系统托盘** - 最小化到托盘，后台运行
- **苹果风格 UI** - 现代简洁的毛玻璃界面，深色/浅色主题

## 截图

*苹果风格毛玻璃 UI*

| 仪表盘 | 设置 |
|--------|------|
| 测速结果展示、加速百分比 | 端点管理、自动模式配置 |

## 技术栈

| 组件 | 技术 |
|------|------|
| 后端 | Rust + Tauri 2.0 |
| 前端 | React 18 + TypeScript + Tailwind CSS |
| 网络 | tokio + tokio-rustls (异步 TLS) |
| DNS | hickory-resolver (异步 DNS) |
| 打包 | ~10MB 单文件 |

## 快速开始

### 环境要求

- Node.js >= 18
- Rust >= 1.75
- Windows 10/11

### 安装依赖

```bash
cd rust

# 安装前端依赖
npm install
```

### 开发模式

```bash
npm run tauri dev
```

### 构建发布版

```bash
npm run tauri build
```

### 输出文件

```
rust/src-tauri/target/release/anyFAST.exe      # 可执行文件
rust/src-tauri/target/release/bundle/msi/      # MSI 安装包
rust/src-tauri/target/release/bundle/nsis/     # NSIS 安装包
```

## 项目结构

```
anyFAST/
├── README.md                     # 本文件
├── CLAUDE.md                     # AI 开发指引
├── .github/
│   └── workflows/
│       ├── ci.yml                # CI 自动测试
│       └── release.yml           # 发布工作流
│
└── rust/                         # 主要实现
    ├── src/                      # React 前端
    │   ├── App.tsx               # 主应用
    │   ├── index.css             # 苹果风格样式
    │   ├── components/
    │   │   ├── Sidebar.tsx       # 侧边栏导航
    │   │   ├── Dashboard.tsx     # 仪表盘
    │   │   ├── Settings.tsx      # 设置页面
    │   │   ├── Logs.tsx          # 日志查看
    │   │   └── HistoryView.tsx   # 历史统计
    │   └── types/
    │       └── index.ts          # TypeScript 类型
    │
    ├── src-tauri/                # Rust 后端
    │   ├── Cargo.toml
    │   ├── tauri.conf.json       # Tauri 配置
    │   └── src/
    │       ├── main.rs           # 入口
    │       ├── lib.rs            # Tauri 命令
    │       ├── models.rs         # 数据模型
    │       ├── config.rs         # 配置管理
    │       ├── endpoint_tester.rs # 端点测速引擎
    │       ├── hosts_manager.rs  # Hosts 文件操作
    │       ├── health_checker.rs # 健康检查
    │       └── history.rs        # 历史记录
    │
    ├── package.json
    ├── tailwind.config.js
    └── vite.config.ts
```

## 核心功能

### Cloudflare IP 优选

自动识别 Cloudflare IP 段并使用优选列表测试：

```rust
// 自动检测 CF IP 范围
const CF_RANGES: &[&str] = &[
    "104.16.", "104.17.", "104.18.", "104.19.",
    "104.20.", "104.21.", "104.22.", "104.23.",
    "172.67.", "162.159.",
];

// 测试多个 CF 优选 IP，选择最快的
```

### 并发测试

使用 tokio 异步运行时并发测试所有 IP：

- 最多 8 个端点同时测试（Semaphore 控制）
- 每个端点测试多个 IP（原始 IP + CF 优选 IP）
- 5 秒连接超时，15 秒总超时
- 自动选择延迟最低的 IP

### 加速效果

显示与原始 DNS 解析 IP 的对比：

```
端点名称    最优 IP          延迟      加速
example    104.21.x.x       120ms    +35.2%
```

## 配置文件

位置: `%APPDATA%\com.anyrouter\fast\config.json`

```json
{
  "mode": "manual",
  "check_interval": 30,
  "slow_threshold": 50,
  "failure_threshold": 3,
  "minimize_to_tray": true,
  "close_to_tray": false,
  "clear_on_exit": false,
  "cloudflare_ips": [],
  "endpoints": [
    {
      "name": "示例端点",
      "url": "https://example.com/v1",
      "domain": "example.com",
      "enabled": true
    }
  ]
}
```

### 配置说明

| 字段 | 说明 | 默认值 |
|------|------|--------|
| `mode` | 运行模式 (manual/auto) | manual |
| `check_interval` | 自动模式检查间隔（秒） | 30 |
| `slow_threshold` | 慢速阈值（%） | 50 |
| `failure_threshold` | 连续失败次数阈值 | 3 |
| `minimize_to_tray` | 最小化到托盘 | true |
| `close_to_tray` | 关闭按钮最小化到托盘 | false |
| `clear_on_exit` | 退出时清除 hosts 绑定 | false |
| `cloudflare_ips` | 自定义 CF 优选 IP 列表 | [] |

## 使用说明

### 手动模式

1. 点击「开始测速」按钮
2. 等待所有端点测试完成
3. 查看结果表格，确认延迟和加速效果
4. 点击「一键应用」绑定所有可用端点
5. 或点击单个端点的「应用」按钮

### 自动模式

1. 在设置页面切换到「自动」模式
2. 配置检查间隔和阈值
3. 程序将在后台自动检测和切换

### 管理员权限

修改 hosts 文件需要管理员权限。请右键以管理员身份运行程序。

## 开发

### 运行测试

```bash
cd rust

# Rust 后端测试
cd src-tauri && cargo test

# 前端测试
npm test
```

### 代码检查

```bash
cd rust/src-tauri

# 格式化
cargo fmt

# Lint
cargo clippy
```

## 许可证

MIT License

## 作者

**hudawang**

## 贡献

欢迎提交 Issue 和 Pull Request！
