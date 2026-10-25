# 安装 RPCRad

从 [项目 Releases](https://github.com/eyotang/RPCRad/releases/latest) 下载。包名包含版本、系统与 CPU 架构；每个包都有同名 `.sha256` 校验文件。应用设置保存在用户目录，升级时替换应用即可。

## macOS

v0.2.0 跟随 [GPUI Kit 的平台要求](https://gpui-kit.com/docs/installation/)，要求 macOS 15 或更新版本。Apple Silicon 选择 `macos-arm64.dmg`，Intel 选择 `macos-x64.dmg`，打开后拖入 Applications。

当前包采用 ad-hoc 签名，尚未 Apple Developer ID 签名、公证。若下载后被 macOS 隔离，在确认文件来自本仓库且校验一致后，可以按需删除 **RPCRad 这一个应用**的隔离属性：

```sh
xattr -dr com.apple.quarantine /Applications/RPCRad.app
open /Applications/RPCRad.app
```

如果装在 `~/Applications`，请把命令中的路径改成实际位置。此命令不是更新程序，也不关闭系统全局 Gatekeeper；正常能打开时无需运行。校验下载文件可运行 `shasum -a 256 -c RPCRad-版本-macos-arm64.dmg.sha256`（与实际文件名一致）。

升级：退出旧版，下载并替换 Applications 中的应用，重新打开。顶部系统菜单 **RPCRad → 检查更新…** 可以主动检查；**关于 RPCRad…** 显示当前版本。

## Windows 10 / 11 x64

下载 `RPCRad-版本-windows-x64-setup.exe` 并运行，默认安装到当前用户的 `%LOCALAPPDATA%\Programs\RPCRad`，创建开始菜单入口和卸载项。也可以下载 ZIP，完整解压后运行 `RPCRad.exe`；不要单独搬走 exe，旁边的 grpcurl 与 resources 目录需要保留。

安装器暂未 Authenticode 签名。更新时退出旧版，再运行新安装器；便携版完整替换解压目录。个人配置位于 `%LOCALAPPDATA%\RPCRad`。

Windows 和 Linux 没有 macOS 顶部系统菜单，在应用窗口右上方 **RPCRad** 下拉菜单中选择 **关于 / 检查更新**。发送、搜索和保存快捷键分别为 **Ctrl+Enter / Ctrl+K / Ctrl+S**。

## Ubuntu 24.04+ x64

```sh
sudo apt install ./rpcrad_版本_amd64.deb
```

安装后可从应用菜单启动，或执行 `rpcrad`。用同样的命令安装新版本即可升级。卸载应用使用 `sudo apt remove rpcrad`，不会删除个人工作区。

需要 Vulkan 图形驱动及桌面会话；仅安装 Vulkan loader 不够。Intel / AMD 一般使用 Mesa，NVIDIA 使用匹配硬件的驱动。应用支持 X11 与 Wayland；文件选择需要正常工作的 `xdg-desktop-portal` 后端。软件渲染只用于 CI 验证，不代替实际显卡驱动。

## Omarchy / Arch Linux x64

[Omarchy](https://omarchy.org/) 使用 Arch Linux 和 Hyprland。下载 `.pkg.tar.zst` 后用 pacman 安装（不要使用 Ubuntu 的 deb）：

```sh
sudo pacman -U ./rpcrad-版本-1-x86_64.pkg.tar.zst
```

随后从应用启动器打开 RPCRad。更新使用同样命令，卸载使用 `sudo pacman -R rpcrad`。该包使用与 Ubuntu 24.04 相容的 glibc 基线；在更新到当前状态的 Arch 环境中安装并进行 Wayland 渲染验证。CI 使用 Arch + Weston 无头 Wayland，不等同于在每一种 Omarchy / Hyprland 显卡组合上完成了人工测试。

确保有适合显卡的 Vulkan 驱动；保留 Omarchy 自带的 portal 配置，文件选择由 `xdg-desktop-portal` 及相应后端处理。中文界面建议安装 `noto-fonts-cjk`。CI 用 Xvfb 给 Weston 提供虚拟键鼠输入；RPCRad 的测试进程移除 DISPLAY，仅连接 Wayland。

## Linux 便携包

完整解压 `.tar.gz`，进入 `RPCRad` 目录执行 `./rpcrad`。二进制、`grpcurl` 和 `resources` 必须放在一起；系统图形库、Vulkan 驱动和 portal 仍由发行版提供。便携包不自动创建应用菜单，日常安装优先使用 deb 或 Arch 包。

Linux 工作区：`${XDG_DATA_HOME:-~/.local/share}/RPCRad/workspace.json`。所有平台均可通过 `RPCRAD_DATA_DIR` 指定测试用隔离目录。

## 从源码构建

所有系统先安装 Rust stable、Go 1.21+ 和 Python 3.11+，运行 `python tools/setup.py`。脚本按 `.go-version` 固定使用 Go 1.26.10 构建 grpcurl 和示例服务，必要时由 Go 自动下载对应工具链。macOS 需 Xcode Command Line Tools；Windows 需 Visual Studio 2022 C++ Build Tools、Windows SDK、CMake，使用 MSVC 工具链；Ubuntu 的原生依赖清单位于 `.github/workflows/ci.yml`。

```sh
cargo run --locked
python tools/release.py --tag v0.2.0
```

发布脚本在当前平台原生构建：Mac 使用 `hdiutil`，Windows 安装器需要 Inno Setup 6，Linux 包需要 `dpkg-deb`、`tar` 与 `zstd`。生成最终更新清单时必须集齐所有平台包：`python tools/release.py --tag v0.2.0 --manifest-only`。
