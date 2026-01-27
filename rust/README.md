# anyFAST

中转站端点优选工具 - 测试 HTTPS 延迟，自动绑定最快 IP 到 hosts。

![Platform](https://img.shields.io/badge/platform-Windows-blue)
![Rust](https://img.shields.io/badge/rust-1.75+-orange)
![License](https://img.shields.io/badge/license-MIT-green)

## 功能特性

- **端点测速** - 并发测试多个中转站端点的 HTTPS 响应延迟
- **Cloudflare 优选** - 自动识别 CF 站点，使用优选 IP 列表测试
- **Hosts 绑定** - 一键应用到 Windows hosts 文件
- **自动模式** - 后台定时检测，自动切换到更快端点
- **系统托盘** - 最小化到托盘，后台运行
- **苹果风格 UI** - 现代简洁的毛玻璃界面

## 技术栈

- **后端**: Rust + Tauri 2.0
- **前端**: React + TypeScript + Tailwind CSS
- **网络**: tokio + rustls (异步 TLS)
- **打包**: ~10MB 单文件

## 快速开始

### 环境要求

- Node.js >= 18
- Rust >= 1.75
- Windows 10/11

### 开发

```bash
cd rust

# 安装前端依赖
npm install

# 开发模式 (热重载)
npm run tauri dev

# 构建发布版
npm run tauri build
```

### 输出

```
rust/src-tauri/target/release/anyFAST.exe     # 可执行文件
rust/src-tauri/target/release/bundle/         # 安装包 (MSI/NSIS)
```

## 项目结构

```
rust/
├── src/                      # React 前端
│   ├── App.tsx               # 主应用
│   ├── index.css             # 苹果风格样式
│   ├── components/
│   │   ├── Sidebar.tsx       # 侧边栏导航
│   │   ├── Dashboard.tsx     # 仪表盘 (测速)
│   │   └── Settings.tsx      # 设置页面
│   └── types/
│       └── index.ts          # TypeScript 类型
│
├── src-tauri/                # Rust 后端
│   ├── Cargo.toml
│   ├── tauri.conf.json       # Tauri 配置
│   ├── icons/                # 应用图标
│   └── src/
│       ├── main.rs           # 入口
│       ├── lib.rs            # Tauri 命令
│       ├── models.rs         # 数据模型
│       ├── config.rs         # 配置管理
│       ├── endpoint_tester.rs # 端点测速 (CF优选)
│       └── hosts_manager.rs  # Hosts 文件操作
│
├── package.json
├── tailwind.config.js        # Apple 风格主题
└── vite.config.ts
```

## 核心逻辑

### Cloudflare 检测

自动识别 Cloudflare IP 段并使用优选列表：

```rust
const CF_RANGES: &[&str] = &[
    "104.16.", "104.17.", "104.18.", "104.19.",
    "104.20.", "104.21.", "104.22.", "104.23.",
    "172.67.", "162.159.",
];
```

### 并发测试

使用 tokio 异步并发测试所有 IP，选择最快的：

```rust
// 并发测试所有 IP
let handles: Vec<_> = test_ips.iter()
    .map(|ip| tokio::spawn(test_single_ip(endpoint, ip)))
    .collect();

// 选择延迟最低的
let best = results.iter().min_by_key(|r| r.latency);
```

### Hosts 操作

精确匹配域名，UTF-8 BOM 处理：

```rust
// 写入格式: IP<TAB>域名<TAB># anyFAST
format!("{}\t{}\t# anyFAST", ip, domain)
```

## 配置文件

位置: `%APPDATA%\com.anyfast.app\config.json`

```json
{
  "mode": "manual",
  "auto_interval": 1800,
  "auto_threshold": 0.5,
  "test_count": 3,
  "minimize_to_tray": true,
  "cloudflare_ips": [],
  "endpoints": [
    {
      "name": "Codex",
      "url": "https://example.com/v1",
      "domain": "example.com",
      "enabled": true
    }
  ]
}
```

## 截图

*苹果风格毛玻璃 UI，深色/浅色主题*

## 许可证

MIT License

## 作者

**hudawang**
