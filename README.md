# RPCRad

本地 gRPC 桌面调试工具。原生界面使用 Rust、GPUI 与 GPUI-KIT 0.8，真实协议调用使用随应用打包的 grpcurl 1.9.3。没有 WebView。

## 安装与更新

从 [GitHub Releases](https://github.com/eyotang/RPCRad/releases/latest) 下载对应系统和架构的包。v0.2.0 起提供：

| 系统 | 架构 | 包 |
| --- | --- | --- |
| macOS 15+ | Apple Silicon / Intel | 各自独立的 `.dmg` |
| Windows 10/11 | x64 | `-setup.exe` 安装器、便携 `.zip` |
| Ubuntu 24.04+ | x64 | `.deb`、便携 `.tar.gz` |
| Omarchy / Arch Linux | x64 | [下载 0.4.0 pacman 安装包](https://github.com/eyotang/RPCRad/releases/download/v0.4.0/rpcrad-0.4.0-1-x86_64.pkg.tar.zst)，原生 Wayland 与 X11 |

所有包包含 grpcurl、示例 Proto 和依赖许可证，不需要安装 Rust 或 Go。Windows 安装器创建开始菜单入口并支持卸载；Linux 系统包安装应用菜单和图标。安装步骤、依赖与 macOS `xattr` 命令见 [安装说明](docs/INSTALL.md)。

沿用 Metra 的 GitHub Release + `latest.json` 更新发现方式。macOS 顶部 **RPCRad → 关于 RPCRad… / 检查更新…**；Windows/Linux 在窗口顶部 **RPCRad** 下拉菜单中操作，设置页也保留入口。启动时自动检查，手动检查会直接显示结果与下载按钮。下载并安装对应平台的新包后，工作区配置保留。当前所有平台均由用户确认安装更新，没有后台静默替换程序。

Secret 和调用历史仅保存在当前会话，重启后需要重新填写 Secret。macOS 包暂未 Developer ID 签名及公证，Windows 安装器暂未 Authenticode 签名。

## macOS 源码本地示例

打开 `dist/RPCRad.app`。本次预览的本地示例服务位于 `localhost:50051`。

如果示例服务没有运行，在项目目录执行：

```sh
tools/bin/rpcrad-demo
```

也可以双击 `Launch RPCRad.command`，它会在端口空闲时启动示例服务并打开桌面应用。

- 选择 **GetUser**，修改 `user_id`，点击发送或按 **⌘ Enter**。
- 选择 **WatchUsers**，可以看到真实的连续响应，也可以点击停止。
- 将 `user_id` 改为 `missing`，可查看服务返回的 NotFound 错误。
- 点击 **导入 Proto** 选择自己的文件，或填写地址后点击 **服务反射**。
- **⌘ K** 搜索方法，**⌘ S** 保存工作区。两列编辑区和侧边栏可拖动调整宽度。

## 当前能力

- Proto 文件导入、服务与方法树、搜索、收藏、多标签、请求草稿。
- JSON 原生编辑器、语法高亮、格式化、按 Proto 生成请求示例。
- 真实 Unary 与服务端流请求，取消、超时、错误状态和耗时。
- Metadata、Bearer token、TLS 系统 CA 验证。
- 环境与全局变量管理，使用 `{{变量名}}` 引用地址、包体、请求头和插件参数；敏感变量只在会话内保存。
- 本地请求头插件：MD5 AppID + 毫秒时间戳签名、HMAC-SHA256 和 AES-256-GCM 示例，参数配置、密钥输入与头部预览。
- 响应正文、Headers、Trailers、复制、会话调用历史与请求重载。
- 左侧历史和底部调用记录旁的垃圾桶图标可一键清空记录，不打断正在进行的请求。
- 点击底部调用记录或左侧历史，查看当次请求详情：插件处理前/最终请求头、请求体、响应和连接设置；支持显示头部值、复制及只读查看。
- 请求草稿、服务定义、连接设置、收藏自动保存到系统用户数据目录，具体路径见 [安装说明](docs/INSTALL.md)。从原项目源码构建的新版会在首次启动时读取该项目的旧工作区；下载的发行版无法定位任意位置的旧源码目录。

认证信息与 Metadata 不落盘。调用历史（包含请求头快照）仅保存在本次会话，最多 50 条；插件 Secret 不写入记录。最终请求头来自 grpcurl 发送进程启动时使用的业务 Metadata，包含插件结果，不包含 HTTP/2 自动添加的协议头。连接失败不代表服务端已经收到这些头。插件失败或发送前取消的记录会明确标记为未发送。

查看记录不会改变当前请求；使用“载入请求”才恢复地址、请求体和连接设置，请求头仍使用当前配置，插件在下次发送时重新计算。

预览版暂不包含客户端流、双向流、自定义 CA / mTLS 与完整的 Proto import-root 管理。Proto 的父目录会自动作为 import path；复杂跨目录依赖需要后续增加自定义 import path。响应展示最多保留 8 MiB。示例 Proto 随应用打包，发布版不依赖源码目录。Windows / Linux Arm64 安装包尚未提供。

## 界面语言

顶部 **简体中文 / English** 下拉框可即时切换语言，无需重启，选择会随工作区保存。菜单、请求编辑、插件、历史详情、环境管理与应用错误提示均支持中英文。首次启动和旧工作区默认使用简体中文。

请求/响应数据、变量值、环境名称和已记录的调用快照保持原样。服务端错误及第三方插件自行提供的文案不会自动翻译。内置插件提供两种语言；本地插件可提供可选的英文名称、说明和字段标签，详见插件开发说明。启动参数 `--language=en` 或 `--language=zh-CN` 可覆盖保存的选择。

## 环境与变量

右上角环境下拉框用于切换环境，旁边的 **变量** 打开管理页。可以新建、重命名、删除环境，添加、编辑、删除变量。全局变量对所有环境生效，当前环境中的同名变量优先；选择“无环境”时只使用全局变量。名称、普通变量和环境选择自动保存。

例如全局添加 `endpoint = localhost:50051`、`include_profile = true`，开发环境添加 `appid`、`user_id` 和标记为 **敏感 · 会话** 的 `secret`：

- 服务地址填 `{{endpoint}}`。
- MD5 插件 AppID 填 `{{appid}}`，Secret 填 `{{secret}}`。时间戳单位仍选择 `milliseconds`，每次发送生成当前毫秒时间。
- 包体填写：

```json
{
  "user_id": "{{user_id}}",
  "include_profile": {{include_profile}}
}
```

带引号的引用按字符串替换，自动转义引号、换行和反斜杠；不带引号的引用必须是有效 JSON，可用于数字、布尔值、对象或数组。Metadata 的值必须是字符串，例如 `{"appid":"{{appid}}"}`。变量值也可引用其他变量，例如 `endpoint = {{host}}:{{port}}`。变量名使用字母或下划线开头，可包含数字、点和短横线。

未定义、重复或循环引用会阻止发送并显示变量名。模板在发送时解析，随后插件使用解析后的参数和包体计算签名；编辑器与磁盘中的草稿仍保留模板。调用详情记录发送当时的环境名称、解析后地址和包体，不随之后的环境切换改变。“载入请求”载入的是该次调用的实际值，插件和请求头仍使用当前配置。

普通变量直接显示并持久保存。敏感变量有独立显示/隐藏按钮，其值不写入工作区文件，重启后需重新填写；插件 Secret 框里的完整 `{{secret}}` 引用会保存，明文 Secret 不保存。若主动将敏感变量引用到包体或业务请求头，它会作为请求内容发送并出现在会话调用快照中。

## 请求头签名

进入 **插件 → MD5 · AppID + 时间戳**，填写 AppID 和 Secret，点击“启用插件”。默认以当前毫秒时间戳生成 `ts`，计算 `MD5(secret + appid + ts + secret)`，发送 `appid`、`ts`、`sign` 三个头。每次发送重新计算，Secret 不保存。

可自定义头名称、时间戳单位及大小写。插件对工作区所有业务调用生效。新增本地插件无需重编译，参见 [插件开发说明](plugins/README.md)。

有多个插件时，点击插件名称切换，名称前的 **✓** 表示当前选择；总开关为 **已启用** 时才执行。当前一次调用只运行一个插件，不支持插件串联或按接口分别绑定。切换插件会清空 Secret，需要重新填写；各插件的普通参数分别保存。

核对发送结果：重新发送后打开新生成的调用记录，选择 **最终请求头**，普通头部（包括 AppID、时间戳和签名）默认完整显示，Secret、密码、Token 等敏感字段分别提供显示/隐藏按钮；复制与当前显示一致。这里保存的是当次调用的结果，不会因之后切换插件而变化。若记录为 **插件失败 · 未发送**，先查看失败原因（例如缺少 Secret）；修正后产生新记录，旧失败记录保持原样。预览按钮生成的时间戳和签名仅用于预览，实际发送时会重新计算。

## 开发与验证

需要 Rust stable、Go、Python 3.11+，以及对应系统的本地构建工具，见 [安装说明](docs/INSTALL.md)。

```sh
python tools/setup.py
cargo run --locked
# 在当前系统构建对应发布包
python tools/release.py --tag v0.4.0
```

首次构建通过 Cargo / Go 下载公开依赖；调试无需账号或云服务。启动时会向 GitHub 检查版本，不上传请求内容。示例服务只监听 127.0.0.1。

```sh
cargo test --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
# 保持示例服务运行，然后：
cargo run --locked -- --smoke-test
```

端到端检查覆盖 Proto 导入、普通请求、服务反射、流式响应、NotFound、超时和取消。

## 发布

推送与 Cargo 版本一致的 `vX.Y.Z` 标签，GitHub Actions 会分别在 macOS 两种架构、Windows 与 Ubuntu 构建，并在 Arch/Wayland 验证安装。所有平台通过后才汇总 SHA-256 与 `latest.json` 并发布。详见 [发布说明](docs/RELEASING.md)。

## 目录

- `src/ui.rs`：原生界面、交互、工作区保存。
- `src/rpc.rs`：grpcurl 进程、服务发现、流式结果与 Metadata 校验。
- `examples/protos/`：可调用的示例协议。
- `tools/demo/`：本地示例服务源代码。
- `design/`：设计稿与实际预览截图。

参考：BloomRPC、Kreya、Yaak。依赖和组件的许可证以各自上游项目为准。
