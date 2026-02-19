# anyFAST

> 中转站端点优选工具 — 一键测速、智能绑定、自动守护

![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue)
![Rust](https://img.shields.io/badge/rust-1.75+-orange)
![Tauri](https://img.shields.io/badge/tauri-2.0-purple)
![License](https://img.shields.io/badge/license-MIT-green)

同一个中转站域名背后可能有几十个 IP，延迟参差不齐。anyFAST 帮你自动找到最快的那个，写入 hosts 绑定，后台持续守护，慢了自动切换。

## 功能

- **一键启动** — 测速 → 绑定 → 守护，全自动完成
- **并发测速** — 同时测试所有端点的所有 IP（DNS + TLS + HTTP 延迟），秒级出结果
- **Cloudflare IP 优选** — 对 Cloudflare CDN 端点自动扫描优质 IP 段
- **智能绑定** — 自动将最快 IP 写入系统 hosts 文件
- **稳定性优先** — 当前绑定的 IP 仍然够快时不会频繁切换
- **后台健康监控** — 持续检测已绑定端点，失败或变慢时自动切换更优 IP
- **批量导入端点** — 支持导入 [All API Hub](https://github.com/qixing-jk/all-api-hub) 备份文件，或直接粘贴多行 URL 批量添加
- **单端点测速** — 支持对单个端点独立测速并绑定
- **单端点解绑** — 支持对单个端点独立解除绑定
- **历史统计** — 记录每次优化效果，查看累计加速数据
- **日志面板** — 实时查看运行日志，方便排查问题
- **托盘常驻** — 最小化到系统托盘，安静运行
- **开机自启** — 可选开机自动运行
- **应用内更新** — 检测新版本后一键下载安装
- **退出清理** — 关闭时自动清除所有 hosts 绑定并刷新 DNS

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

1. 启动 anyFAST，在主界面添加你使用的中转站端点
2. 点击「启动」按钮
3. 等待测速完成，自动绑定最优 IP
4. 后台持续守护 — IP 变慢时自动切换
5. 点击「停止」可停止守护并清除所有绑定

也可以在 Dashboard 中对单个端点进行独立测速、绑定或解绑。

### 批量导入端点

在端点列表区域点击「导入」按钮，支持两种方式：

- **文件导入** — 选择 [All API Hub](https://github.com/qixing-jk/all-api-hub) 导出的备份 JSON 文件，自动识别站点列表
- **文本粘贴** — 直接粘贴多行 URL（支持换行、逗号、分号分隔）

导入前可预览、勾选需要的站点，已存在的端点会自动标记避免重复。

## 权限说明

写入 hosts 文件需要系统特权，不同平台策略不同：

- **Windows** — 通过 named pipe 与 anyfast-service（Windows 系统服务）通信；若服务不可用则降级为直接操作（需管理员权限）
- **macOS** — 通过 setuid helper 程序操作；首次运行通过 osascript 提权安装

## 技术栈

- **Rust + Tauri 2.0** — 原生性能，跨平台
- **React + TypeScript** — 现代化前端
- **安装包 ~10MB** — 小巧精悍
- **内存占用 ~30MB** — 轻量运行

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
# 前端
npm test

# Rust 后端
cd src-tauri && cargo test --verbose
```

### 环境要求

- Node.js >= 18
- Rust >= 1.75
- Windows 10/11 / macOS 12+ / Linux

## 许可证

MIT License

---

**GitHub**: https://github.com/wangwingzero/anyFAST
