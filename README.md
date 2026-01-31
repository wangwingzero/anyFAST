# anyFAST v1.2.3

> 中转站端点优选工具 - 自动测速、智能切换、hosts 绑定

![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS-blue)
![Rust](https://img.shields.io/badge/rust-1.75+-orange)
![Tauri](https://img.shields.io/badge/tauri-2.0-purple)
![License](https://img.shields.io/badge/license-MIT-green)

各位佬友好！用中转站的都懂那种痛——同一个域名背后几十个 IP，延迟从 100ms 到 800ms 不等，手动一个个测？累死个人。

所以肝了这个工具，**打开即用、自动测速、自动绑定、自动起飞**。

## 功能特性

- **打开即用** - 启动后自动开始测速和绑定，无需手动操作
- **秒级测速** - 并发测所有 IP，几秒出结果，告别手动 ping
- **智能优选** - 自动找最快的 IP，延迟降低 30%-70% 不是梦
- **一键绑定** - 自动写入 hosts，小白也能用
- **后台守护** - IP 变慢了自动切换，全程无感
- **托盘常驻** - 最小化到托盘，安静守护你的网络
- **历史统计** - 记录每次优化效果，看看省了多少延迟
- **开机自启** - 设置里勾一下，开机就自动工作
- **退出清理** - 关闭程序自动清除 hosts 绑定，不留痕迹

## 内置端点

已内置两位大佬的中转站（**再次感谢！**）：

| 名称 | 域名 |
|------|------|
| **anyrouter大善人** | betterclau.de |
| **L站WONG大佬** | wzw.pp.ua |

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

## 设置说明

v1.1.0 简化了设置界面，只保留必要选项：

| 设置项 | 说明 |
|--------|------|
| **开机自启动** | 系统启动时自动运行 anyFAST |
| **Hosts 文件** | 手动打开系统 hosts 文件 |
| **检查更新** | 检查是否有新版本 |
| **恢复默认值** | 重置端点配置 |

以下行为已改为默认强制：
- ✅ 自动模式运行（无需手动触发）
- ✅ 关闭/最小化时隐藏到托盘
- ✅ 退出时自动清除 hosts 绑定

## 开源地址

**GitHub**: https://github.com/wangwingzero/anyFAST

欢迎 Star 欢迎 PR 欢迎提 Issue！

---

## 开发指南

### 环境要求

- Node.js >= 18
- Rust >= 1.75
- Windows 10/11 或 macOS 12+

### macOS 权限设置

macOS 版本首次启动时会弹出对话框，点击"安装 Helper"按钮，系统会弹出密码输入框。输入密码后 Helper 即安装完成，重启应用即可正常使用。

整个过程只需要输入一次密码，之后永久无感使用。

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

### 运行测试

```bash
cd rust

# Rust 后端测试
cd src-tauri && cargo test

# 前端测试
npm test
```

## 许可证

MIT License

## 致谢

感谢 WONG 大佬和 anyrouter 大佬提供的中转服务！
