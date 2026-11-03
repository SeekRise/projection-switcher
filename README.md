# ProjectionSwitcher · 投影一键切换

Windows 桌面常驻小工具：一个全局快捷键在 **仅电脑屏幕 ⇄ 仅第二屏幕** 之间来回切换（等价于 Win+P 的两种模式），另一个快捷键直接切换**外接显示器的输入源**（DDC/CI）。基于 Tauri 2 构建，体积小、无外部进程依赖、关窗即驻留托盘。

## 功能特性

- **一键切换投影**（默认 `Ctrl+Alt+D`）：在"仅电脑屏幕"与"仅第二屏幕"之间取反切换；也支持显式切换到 复制 / 扩展 模式
- **切换显示器输入源**（默认 `Ctrl+Alt+I`）：通过 DDC/CI（VCP 0x60）在配置的输入源列表中循环切换，例如 HDMI ⇄ DisplayPort，无需触碰显示器按键
- **系统托盘常驻**：关闭窗口 = 最小化到托盘；左键托盘打开主窗口，右键菜单可直达两个切换动作
- **开机自启**：可选，以 `--hidden` 静默启动，仅驻留托盘
- **快捷键可自定义**：注册失败（被其他程序占用）自动回滚并提示
- **目标显示器可选**：按 EDID 型号 ID 精确匹配（与 ControlMyMonitor 的 Monitor Name 同体系）

## 工作原理

**投影切换** —— 直接调用 Win32 Display Configuration API 的 `SetDisplayConfig`，与系统 Win+P 面板相同的底层调用，不启动 `DisplaySwitch.exe` 等外部进程。当前处于哪一侧的判定采用"分辨率签名 + 状态记忆"：

1. 枚举第一个活动显示输出，取当前分辨率作为模式签名；
2. 与已学习的内置/外接签名比对，判定"仅电脑屏幕"或"仅第二屏幕"；
3. 签名无法判定时（首次运行 / 复制 / 扩展 / 改过分辨率），按上次切换记录取反；
4. 每次切换后轮询签名变化并重新学习，越用越准。

**输入源切换** —— 基于 Windows 物理显示器 API（`dxva2.dll`，DDC/CI 协议）读写 VCP code `0x60`（Input Select）：读取当前值，在配置的值列表中循环切到下一个；读不到或不在列表中时切到列表第一个值。

## 安装与构建

### 环境要求

- Windows 10 / 11
- [Node.js](https://nodejs.org/) ≥ 20
- [Rust](https://www.rust-lang.org/) stable 工具链（msvc target）
- WebView2 运行时（Windows 10/11 一般已内置）

### 从源码构建

```bash
npm install          # 安装前端依赖
npm run tauri dev    # 开发调试
npm run tauri build  # 产出 NSIS 安装包（src-tauri/target/release/bundle/nsis/）
```

## 配置说明

配置与状态文件均位于 `%APPDATA%`：

| 文件 | 用途 |
| --- | --- |
| `ProjectionSwitcher.json` | 用户配置：快捷键、目标显示器、输入源值列表 |
| `DisplaySwitchToggle.state` | 投影切换的状态记忆（上次切换方向、已学习的分辨率签名） |

`ProjectionSwitcher.json` 示例：

```json
{
  "shortcut": "Ctrl+Alt+D",
  "inputShortcut": "Ctrl+Alt+I",
  "inputMonitor": "XMI27B1",
  "inputValues": [18, 15]
}
```

- `inputMonitor`：目标显示器的 EDID 型号 ID（程序界面中会列出候选）
- `inputValues`：VCP 0x60 的取值（**十进制**），如 `18`（0x12，HDMI）、`15`（0x0F，DisplayPort），切换时在此列表中循环

以上配置均可直接在应用"设置"页中修改，无需手工编辑文件。

## 技术栈

- **后端**：Rust + Tauri 2 + `windows-sys`（直接调用 Win32 API：`SetDisplayConfig`、`SetVCPFeature` 等）
- **前端**：React 19 + TypeScript + Tailwind CSS 4 + Vite
- **插件**：`tauri-plugin-global-shortcut`（全局快捷键）、`tauri-plugin-autostart`（开机自启）

## 目录结构

```
src/                  # React 前端
  features/           # 投影 / 输入源 / 设置 三个功能面板
  components/         # 通用 UI 组件
src-tauri/
  src/projection.rs   # 投影切换（SetDisplayConfig + 分辨率签名判定）
  src/monitor_input.rs# 输入源切换（DDC/CI VCP 0x60）
  src/config.rs       # 配置持久化（%APPDATA%\ProjectionSwitcher.json）
  src/lib.rs          # Tauri 命令、托盘、快捷键注册
assets/               # 图标及生成脚本
```
