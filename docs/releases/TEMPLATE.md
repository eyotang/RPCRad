# RPCRad VERSION

## 中文

本版本的具体变更：填写用户可见的功能、修复与兼容性变化。

升级：在菜单中检查更新，下载对应平台安装包，退出旧版后安装。保留工作区设置；敏感值和调用历史仍仅保存在当前会话。列出本版本的适用平台、验证结果与限制。

macOS：如果系统隔离导致无法打开，先确认文件来自官方仓库且 SHA-256 一致，再由用户按需执行；路径不同请替换为实际安装路径。此命令不是更新步骤，程序和发布脚本不会自动执行。

```sh
xattr -dr com.apple.quarantine /Applications/RPCRad.app
open /Applications/RPCRad.app
```

## English

Describe this release's user-visible features, fixes and compatibility changes, matching the Chinese section.

Upgrade: check for updates in the menu, download the package for your system, quit the old version and install. Workspace settings are preserved; sensitive values and history remain session-only. State supported platforms, verification results and limitations for this release.

macOS: if quarantine prevents launching, first verify the official source and matching SHA-256, then run the following only if needed. Replace the path if installed elsewhere. This is not an update step; the application and release scripts never execute it automatically.

```sh
xattr -dr com.apple.quarantine /Applications/RPCRad.app
open /Applications/RPCRad.app
```
