# anyFAST

> 中转站端点优选工具 - 一键测速、智能绑定、自动守护

![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS-blue)
![Rust](https://img.shields.io/badge/rust-1.75+-orange)
![Tauri](https://img.shields.io/badge/tauri-2.0-purple)
![License](https://img.shields.io/badge/license-MIT-green)

用中转站的都懂——同一个域名背后几十个 IP，延迟从 100ms 到 800ms 不等。手动测？累死。

这个工具帮你搞定：**打开即用、自动测速、自动绑定、自动起飞**。

## ✨ 功能

- **一键启动** - 点击启动按钮，自动测速 + 绑定 + 后台守护
- **秒级测速** - 并发测试所有端点，几秒出结果
- **智能优选** - 自动选最快 IP，延迟降低 30%-70%
- **自动绑定** - 写入 hosts 文件，无需手动操作
- **后台守护** - 持续监控，IP 变慢自动切换
- **托盘常驻** - 最小化到托盘，安静运行
- **历史统计** - 记录优化效果，查看累计节省时间
- **开机自启** - 可选开机自动运行
- **退出清理** - 关闭时自动清除 hosts 绑定

## 📦 内置 15 个公益中转站

感谢各位大佬提供的公益服务！

| 名称 | 站点 |
|------|------|
| WONG公益站 | wzw.pp.ua |
| anyrouter大善人 | anyrouter.top |
| henryxiaoyang | runanytime.hxi.me |
| Cyrus (鸭佬) | free.duckcoding.com |
| ByteBender | elysiver.h-e.top |
| beizhi (Wind Hub) | api.224442.xyz |
| kkkyyx (不过减速带) | kfc-api.sxxe.net |
| 钟阮 | gyapi.zxiaoruan.cn |
| sc0152 (DEV88) | api.dev88.tech |
| ZeroLiya (小呆) | new.184772.xyz |
| freenessfish | welfare.apikey.cc |
| Mitchll | api.mitchll.com |
| mazhichen等四位大佬 | api.hotaruapi.top |
| TechnologyStar | aidrouter.qzz.io |
| Simonzhu | ai.zzhdsgsss.xyz |

## 📥 下载安装

前往 [GitHub Releases](https://github.com/wangwingzero/anyFAST/releases) 下载：

- **Windows**: `anyFAST_x.x.x_x64-setup.exe`（安装包已集成 Service）
- **macOS Intel**: `anyFAST_x.x.x_x64.dmg`
- **macOS Apple Silicon**: `anyFAST_x.x.x_aarch64.dmg`

### Windows 使用

安装后直接运行，Service 已集成在安装包中，无需额外下载。

### macOS 使用

首次启动会提示安装 Helper，点击安装并输入系统密码即可。只需一次，之后永久无感使用。

## 🚀 使用方法

1. 启动 anyFAST
2. 点击「启动」按钮
3. 等待测速完成，自动绑定最优 IP
4. 后台持续守护，IP 变慢自动切换
5. 点击「停止」按钮可停止守护并清除绑定

## 🛠 技术栈

- **Rust + Tauri 2.0** - 高性能、低内存
- **React + TypeScript** - 现代化前端
- **安装包 ~10MB** - 小巧精悍
- **内存占用 30-50MB** - 轻量运行

## 📊 效果展示

实测加速效果：
- 原始 400ms → 优化 150ms，**提升 62%**
- 原始 600ms → 优化 200ms，**提升 67%**

## 🔧 开发

```bash
# 克隆项目
git clone https://github.com/wangwingzero/anyFAST.git
cd anyFAST/rust

# 安装依赖
npm install

# 开发模式
npm run tauri dev

# 构建发布
npm run tauri build

# 运行测试
cd src-tauri && cargo test
npm test
```

### 环境要求

- Node.js >= 18
- Rust >= 1.75
- Windows 10/11 或 macOS 12+

## 📄 许可证

MIT License

## 🙏 致谢

感谢所有提供公益中转服务的大佬们！

---

**GitHub**: https://github.com/wangwingzero/anyFAST

欢迎 Star ⭐ | 欢迎 PR | 欢迎提 Issue
