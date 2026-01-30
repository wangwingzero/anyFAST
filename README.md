# anyFAST v1.2.1

> 中转站端点优选工具 - 自动测速、智能切换、hosts 绑定

![Platform](https://img.shields.io/badge/platform-Windows-blue)
![Rust](https://img.shields.io/badge/rust-1.75+-orange)
![Tauri](https://img.shields.io/badge/tauri-2.0-purple)
![License](https://img.shields.io/badge/license-MIT-green)

各位佬友好！用中转站的都懂那种痛——同一个域名背后几十个 IP，延迟从 100ms 到 800ms 不等，手动一个个测？累死个人。

所以肝了这个工具，**打开即用、自动测速、自动绑定、自动起飞**。

## 这个工具是干什么的？

简单说：**只加速 URL，不改你的配置**。

比如你的 MCP 配置是这样的：

```json
{
  "ANTHROPIC_AUTH_TOKEN": "sk-XXXXXXXXXXX",
  "ANTHROPIC_BASE_URL": "https://betterclau.de/claude/anyrouter.top"
}
```

anyFAST 做的事情是：
1. 测试 `betterclau.de` 这个域名背后所有 IP 的延迟
2. 找出最快的那个 IP
3. 把 `betterclau.de -> 最快IP` 写入系统 hosts 文件

**你的配置完全不用改**，该怎么设置还是怎么设置。anyFAST 只是在 DNS 层面帮你绑定最快的 IP，让你的请求自动走最优线路。

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
- **自定义端点** - 支持添加/删除/启用/禁用端点

## 内置端点

已内置两位大佬的中转站（**再次感谢！**）：

| 名称 | 域名 |
|------|------|
| **BetterClaude (anyrouter大善人)** | betterclau.de |
| **WZW 代理 (L站WONG大佬)** | wzw.pp.ua |

## 效果展示

实测加速效果：
- 原始延迟 400ms → 优化后 150ms，**提升 62%**
- 原始延迟 600ms → 优化后 200ms，**提升 67%**

## 技术栈

- **Rust + Tauri 2.0** - 性能拉满，安全可靠
- **React + TypeScript** - 现代前端技术栈
- **安装包 ~10MB** - 小巧精悍
- **内存占用 30-50MB** - 轻量运行
- **Windows Service 模式** - 无需每次管理员确认

## 下载使用

1. 去 [GitHub Releases](https://github.com/wangwingzero/anyFAST/releases) 下载最新版
2. 安装时会申请管理员权限（写 hosts 需要）
3. 打开即自动测速绑定，坐等起飞！

## 界面说明

### 仪表盘

主界面显示：
- **状态栏** - 已测/可用/绑定数量，工作状态指示器
- **端点列表** - 管理端点（添加/删除/启用/禁用）
- **控制面板** - 启动/停止按钮，进度显示
- **测速结果** - 实时显示各端点延迟和加速效果

### 设置

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
- Windows 10/11

### 开发模式

```bash
cd rust
npm install
npm run tauri dev
```

### 构建发布版

```bash
cd rust
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
