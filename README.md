# RPCRad

本地 gRPC 桌面调试工具。原生界面使用 Rust、GPUI 与 GPUI-KIT 0.8，真实协议调用使用随应用打包的 grpcurl 1.9.3。没有 WebView。

## 安装与更新

从 [GitHub Releases](https://github.com/eyotang/RPCRad/releases/latest) 下载 DMG，将 RPCRad 拖入 Applications。首版支持 macOS 14+ 的 Apple Silicon，包内含 grpcurl 和示例 Proto，无需 Rust / Go。安装包采用 ad-hoc 签名，尚未使用 Apple Developer ID 签名或公证。

沿用 Metra 的 macOS 更新流程：启动时检查 `latest.json`；也可在 **设置 → 检查更新** 手动检查。发现新版本后点击下载，打开对应 GitHub Release，下载并替换应用。工作区配置保留，Secret 仍需在新会话填写。

## 源码本地示例

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
- 本地请求头插件：MD5 AppID + 毫秒时间戳签名、HMAC-SHA256 和 AES-256-GCM 示例，参数配置、密钥输入与头部预览。
- 响应正文、Headers、Trailers、复制、会话调用历史与请求重载。
- 点击底部调用记录或左侧历史，查看当次请求详情：插件处理前/最终请求头、请求体、响应和连接设置；支持显示头部值、复制及只读查看。
- 请求草稿、服务定义、连接设置、收藏自动保存到 `~/Library/Application Support/RPCRad/workspace.json`。从原项目源码构建的新版会在首次启动时读取该项目的旧工作区；下载的发行版无法定位任意位置的旧源码目录。

认证信息与 Metadata 不落盘。调用历史（包含请求头快照）仅保存在本次会话，最多 50 条；插件 Secret 不写入记录。最终请求头来自 grpcurl 发送进程启动时使用的业务 Metadata，包含插件结果，不包含 HTTP/2 自动添加的协议头。连接失败不代表服务端已经收到这些头。插件失败或发送前取消的记录会明确标记为未发送。

查看记录不会改变当前请求；使用“载入请求”才恢复地址、请求体和连接设置，请求头仍使用当前配置，插件在下次发送时重新计算。

预览版暂不包含客户端流、双向流、自定义 CA / mTLS、多环境管理与完整的 Proto import-root 管理。Proto 的父目录会自动作为 import path；复杂跨目录依赖需要后续增加自定义 import path。响应展示最多保留 8 MiB。示例 Proto 随应用打包，发布版不依赖源码目录。Windows、Linux 与 Intel 安装包尚未提供。

## 请求头签名

进入 **插件 → MD5 · AppID + 时间戳**，填写 AppID 和 Secret，点击“启用插件”。默认以当前毫秒时间戳生成 `ts`，计算 `MD5(secret + appid + ts + secret)`，发送 `appid`、`ts`、`sign` 三个头。每次发送重新计算，Secret 不保存。

可自定义头名称、时间戳单位及大小写。插件对工作区所有业务调用生效。新增本地插件无需重编译，参见 [插件开发说明](plugins/README.md)。

有多个插件时，点击插件名称切换，名称前的 **✓** 表示当前选择；总开关为 **已启用** 时才执行。当前一次调用只运行一个插件，不支持插件串联或按接口分别绑定。切换插件会清空 Secret，需要重新填写；各插件的普通参数分别保存。

核对发送结果：重新发送后打开新生成的调用记录，选择 **最终请求头**，普通头部（包括 AppID、时间戳和签名）默认完整显示，Secret、密码、Token 等敏感字段分别提供显示/隐藏按钮；复制与当前显示一致。这里保存的是当次调用的结果，不会因之后切换插件而变化。若记录为 **插件失败 · 未发送**，先查看失败原因（例如缺少 Secret）；修正后产生新记录，旧失败记录保持原样。预览按钮生成的时间戳和签名仅用于预览，实际发送时会重新计算。

## 开发与验证

需要 Rust stable、Go、Python 3.11+ 和 macOS Command Line Tools。

```sh
sh tools/setup.sh
cargo run --locked
sh tools/package.sh
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

推送与 Cargo 版本一致的 `vX.Y.Z` 标签，GitHub Actions 会执行验证、构建 DMG、生成 SHA-256 与 `latest.json` 并发布。详见 [发布说明](docs/RELEASING.md)。

## 目录

- `src/ui.rs`：原生界面、交互、工作区保存。
- `src/rpc.rs`：grpcurl 进程、服务发现、流式结果与 Metadata 校验。
- `examples/protos/`：可调用的示例协议。
- `tools/demo/`：本地示例服务源代码。
- `design/`：设计稿与实际预览截图。

参考：BloomRPC、Kreya、Yaak。依赖和组件的许可证以各自上游项目为准。
