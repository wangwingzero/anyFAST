# anyFAST v1.0.3

> 中转站端点优选工具 - 自动测速、智能切换、hosts 绑定

![Platform](https://img.shields.io/badge/platform-Windows-blue)
![Rust](https://img.shields.io/badge/rust-1.75+-orange)
![Tauri](https://img.shields.io/badge/tauri-2.0-purple)
![License](https://img.shields.io/badge/license-MIT-green)

各位佬友好！用中转站的都懂那种痛——同一个域名背后几十个 IP，延迟从 100ms 到 800ms 不等，手动一个个测？累死个人。

所以肝了这个工具，**打开即用、自动测速、自动绑定、自动起飞**。

## 功能特性

- **秒级测速** - 并发测所有 IP，几秒出结果，告别手动 ping
- **智能优选** - 自动找最快的 IP，延迟降低 30%-70% 不是梦
- **一键绑定** - 自动写入 hosts，小白也能用
- **后台守护** - 开启自动模式，IP 变慢了自动切换，全程无感
- **托盘常驻** - 最小化到托盘，安静守护你的网络
- **历史统计** - 记录每次优化效果，看看省了多少延迟
- **开机自启** - 设置里勾一下，开机就自动工作
- **打开即用** - 启动后自动开始测速和绑定，无需手动操作

## 内置端点

已内置两位大佬的中转站（**再次感谢！**）：

| 名称 | 域名 |
|------|------|
| **anyrouter大善人** | betterclau.de |
| **L站WONG大佬** | wzw.pp.ua |

其他中转站可以在设置里自己添加，填个 URL 就行。

## 效果展示

实测加速效果：
- 原始延迟 400ms → 优化后 150ms，**提升 62%**
- 原始延迟 600ms → 优化后 200ms，**提升 67%**

## 技术栈

- **Rust + Tauri 2.0** - 性能拉满，安全可靠
- **安装包 ~10MB** - 小巧精悍
- **内存占用 30-50MB** - 轻量运行
- **Windows Service 模式** - 无需每次管理员确认

## 下载使用

1. 去 [GitHub Releases](https://github.com/wangwingzero/anyFAST/releases) 下载最新版
2. 安装时会申请管理员权限（写 hosts 需要）
3. 打开即自动测速绑定，坐等起飞！

## 开源地址

**GitHub**: https://github.com/wangwingzero/anyFAST

欢迎 Star 欢迎 PR 欢迎提 Issue！

---

## 开发指南

### 环境要求

- Node.js >= 18
- Rust >= 1.75
- Windows 10/11

### 开发模式

```bash
cd rust
npm install
npm run tauri dev
```

### 构建发布版

```bash
npm run tauri build
```

## 配置文件

位置: `%APPDATA%\AnyRouter\fast\config\config.json`

| 字段 | 说明 | 默认值 |
|------|------|--------|
| `mode` | 运行模式 (manual/auto) | auto |
| `check_interval` | 自动模式检查间隔（秒） | 30 |
| `minimize_to_tray` | 最小化到托盘 | true |
| `close_to_tray` | 关闭按钮最小化到托盘 | true |
| `clear_on_exit` | 退出时清除 hosts 绑定 | false |
| `autostart` | 开机自启动 | false |

## 许可证

MIT License

## 致谢

感谢 WONG 大佬和 anyrouter 大佬提供的中转服务！
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
