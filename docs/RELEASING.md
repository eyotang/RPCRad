# 发布与更新

使用 Metra 风格的 GitHub Release + `latest.json` + SHA-256 契约，所有平台打开官方版本下载页，由用户安装。没有签名验证链的自动执行安装器，不复用 Metra 的任何私钥。

## 自动发布

1. 修改 Cargo.toml 与 Cargo.lock 版本，添加 `docs/releases/vX.Y.Z.md`。
2. 本地执行 `cargo fmt --all -- --check`、`cargo test --locked`、Clippy、`python -m unittest discover -s tools/tests`。
3. 提交 main。工作流在 Apple Silicon、Intel Mac、Windows x64、Ubuntu x64 原生构建安装包，测试被移动到新目录后的包，验证真实请求、签名、原生窗口帧；另在 Arch 容器安装包并测试 Wayland。
4. 创建并推送与版本严格匹配的标签。所有平台与 Arch 验证通过后，汇总七种包及各自 SHA-256，检查没有遗漏或意外文件，再生成兼容旧客户端的 latest.json，先上传到草稿，再公开发布为 latest。

`.dmg`（arm64/x64）、Windows `.exe` 安装器及 `.zip`、Linux `.tar.gz`、Ubuntu `.deb`、Arch `.pkg.tar.zst`。不能只上传某个平台就更新 latest。公开版本不能覆写；修复递增版本。失败停在草稿阶段可以重跑。

主分支每次通过后的构建产物保存 14 天，也可以下载这些**同一提交、全部通过验证**的产物发布；先创建对应标签，避免与自动发布任务重复写入。只有发布任务获得 contents:write，构建验证任务只读。

## 本机构建

`python tools/setup.py` 安装固定版本 grpcurl 和本地示例服务。`python tools/release.py --tag vX.Y.Z` 构建当前 OS/架构，产物在 release-assets/vX.Y.Z。Mac 的 `.app` 在 dist/release，Windows/Linux 的完整目录在 dist/平台/RPCRad。

资源和用户配置分离。外部 grpcurl 使用当前系统可执行文件后缀，Windows 调用不弹出命令窗口；内置 Proto 随应用移动。Windows EXE 内嵌图标，Linux 系统包安装 desktop entry 与图标。

## 验证边界

原生 UI smoke test 等待实际绘制两帧后才成功；CLI 成功不作为 UI 成功的替代。Linux 的 CI 覆盖 Ubuntu/X11 和 Arch/Wayland。Omarchy 的 Hyprland 日常桌面、不同显卡与显示缩放仍需用户设备验证。Windows / Linux ARM64 暂无构建。

目前 macOS ad-hoc 签名且未公证，Windows 安装器未 Authenticode 签名。macOS 的按需 xattr 命令在 [安装说明](INSTALL.md) 中；程序、安装器和构建脚本不会自动清除用户应用隔离属性或关闭安全保护。
