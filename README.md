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
