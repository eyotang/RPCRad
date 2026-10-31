# RPCRad 项目约定

## 多语言

维护简体中文、English、日本語、한국어、Français、Deutsch、Русский的界面与 README。修改用户可见文案时同步翻译；保留格式占位符、变量引用、请求/响应数据和用户命名的原值。新增语言或调整翻译结构时运行翻译完整性检查及语言切换、保存恢复测试。

## 每个版本的发布说明

创建或修改任何版本的发布说明（`docs/releases/vX.Y.Z.md` 和 GitHub Release 正文）时，必须同时包含 `## 中文` 与 `## English` 两个实质内容章节，分别说明该版本的改动、升级方式和适用限制；两种语言的信息必须一致。

每个版本的发布说明正文都必须直接保留以下 macOS 命令，不能仅链接到安装文档：

```sh
xattr -dr com.apple.quarantine /Applications/RPCRad.app
open /Applications/RPCRad.app
```

在中英文说明中解释：仅在确认下载来自官方仓库、SHA-256 校验一致且 macOS 隔离导致无法打开时，才由用户按需执行；路径不同需替换为实际安装路径。此命令不属于更新步骤，程序和发布脚本不得自动执行它。

发布前读取 `docs/RELEASING.md`，通过发布说明检查、全平台构建和安装验证后再公开版本。发布后核对 GitHub Release 正文仍包含中英文内容及上述 xattr 命令，并验证下载清单与校验值。
