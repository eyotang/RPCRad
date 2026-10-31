# 请求头插件

在应用“插件”页选择插件并启用；设置应用于当前工作区的所有业务调用。服务反射不执行插件。

## MD5 规则

默认生成以下字符串 Metadata：

- `appid`：输入的 AppID 原值，不额外修剪或编码。
- `ts`：发送前读取的 Unix 毫秒时间戳（当前为 13 位）；点击时间戳单位可切换为秒。
- `sign`：`MD5(secret + appid + ts + secret)`，UTF-8 字节直接拼接，无分隔符，默认 32 位小写十六进制。

时间戳只读取一次，签名与 `ts` 头完全一致。三个头名称及大小写可调整；与已有 Metadata 同名时覆盖。每次发送重新计算，预览不会被复用。

Secret 使用密码框，明文仅保留在当前会话。AppID、单位、头名称等普通参数会存到 用户数据目录的 `workspace.json`（macOS：`~/Library/Application Support/RPCRad/`）；不要把密钥填入普通参数。脚本不会直接拿到 Secret 框的原文，只能通过宿主提供的密码函数使用它。

所有参数与 Secret 框均可使用 `{{变量名}}`，例如 AppID 为 `{{appid}}`、Secret 为 `{{secret}}`。当前环境覆盖同名全局变量，发送和预览前先解析变量再执行脚本。Secret 框中的完整变量引用会保存，敏感变量值不会落盘。变量不是脚本全局对象：将需要的普通变量通过 `config` 字段传入，密钥通过 Secret 字段传入。

## 添加自己的插件

自带插件嵌入应用，无需启动时扫描磁盘。在本目录或自己的插件目录新增子目录，其中包含 `plugin.json` 和 `main.rhai`；点击“加载本地插件…”选择其父目录。之后修改文件可点击“重新加载插件”。读取在后台完成，同 ID 的本地插件覆盖内置版本；“恢复内置插件”可解除本地目录绑定。无需重编译应用。示例可复制 `md5-appid-ts/` 后修改唯一的 `id` 和规则。

`plugin.json` 格式：

```json
{
  "api_version": 1,
  "id": "my-signature",
  "name": "我的签名",
  "description": "规则说明",
  "secret_encoding": "utf8",
  "fields": [
    { "key": "appid", "label": "AppID", "default": "" },
    { "key": "unit", "label": "单位", "default": "milliseconds", "options": ["milliseconds", "seconds"] }
  ]
}
```

`fields` 最多 16 个字符串字段。所有字段均可直接输入变量引用；有 `options` 时额外显示“切换选项”按钮。密钥编码为 `utf8`、`hex` 或 `base64`，在宿主解码后交给密码函数。

### 多语言文案

`plugin.json` 可选填 `name_en`、`description_en`，每个字段可选填 `label_en`。切换到英文后优先使用这些文案；缺省时保留原文，兼容已有本地插件。内置插件通过 `locales/` 提供简体中文、英文、日语、韩语、法语、德语和俄语七种语言。未提供对应翻译的自定义插件在其他语言下使用英文或原文，不翻译用户配置值。

脚本错误可以写为 `throw ui_text("请填写 AppID", "Enter an AppID");`。`ui_text(中文, 英文)` 在简体中文下使用首个参数，其他语言优先查找内置翻译，未匹配时回退到第二个参数；它只选择显示文案，请勿用它生成签名输入或请求数据。配置键、选项值和签名规则不随语言改变。使用 `ui_text` 的脚本需要新版宿主，旧版宿主请继续使用普通字符串错误。

Rhai 脚本中可使用：

| 名称 | 内容 |
| --- | --- |
| `config` | 变量解析后的普通参数对象，值均为字符串 |
| `metadata` | 原始 Metadata，已合并 Bearer token，可修改 |
| `request.method` | `/完整服务名/方法名` |
| `request.endpoint` | 规范化后的目标地址 |
| `request.body` | 变量解析后即将发送的 JSON 文本，不是 protobuf 二进制消息 |
| `request.timestamp_ms` | 当前 Unix 毫秒时间戳字符串 |
| `request.timestamp` | 同一时刻的 Unix 秒时间戳字符串 |
| `md5_secret_wrap(data)` | 小写 hex MD5(secret + data + secret) |
| `hmac_sha256(data)` | 小写 hex HMAC-SHA256，使用会话 secret |
| `sha256(data)` | 小写 hex SHA-256 |
| `aes256_gcm(text, aad)` | 使用 32 字节 secret，每次使用新的随机 nonce |

**脚本最后必须返回完整的 Metadata 对象**，不是增量。保留原有头时先修改 `metadata`，最后返回它；移除原文头用 `metadata.remove(key)`。`throw "原因"` 会拦截本次请求。

所有输出值必须为可打印 ASCII 字符串，键必须为小写合法 Metadata 名称；不支持 `grpc-` 保留头或 `-bin` 二进制头。当前原始 Metadata 输入同样遵守 ASCII 限制。

## 示例的协议约定

- **HMAC-SHA256**：签名内容为 `request.method + "\n" + request.timestamp_ms + "\n" + request.body`，结果放入 `x-signature`，时间戳放入 `x-timestamp`。
- **AES-256-GCM**：输入 `x-payload`，输出 `x-encrypted` 并移除原文头。密钥为 64 个 hex 字符，AAD 默认为空。输出为 `Base64(nonce[12] || ciphertext || tag[16])`。这是示例格式，接入实际服务需要双方使用同一格式。

## 执行限制

脚本在后台线程执行；不开放文件、网络、进程和导入功能，禁用 eval，print/debug 不输出。限制 50,000 次操作、250 ms 执行时间、16 层调用、1 MiB 字符串、64 个输出头及 32 KiB 输出总长度。文件最大 64 KiB。插件错误、超限或取消时不会调用 grpcurl。

仅加载可信的本地插件：插件可修改即将发给目标服务的请求头。该机制是受限脚本扩展，不是操作系统进程隔离。

## 验证

```sh
cargo test --locked
GOPATH="$PWD/.cache/go" GOCACHE="$PWD/.cache/go-build" go build -C tools/demo -o ../bin/rpcrad-demo .
tools/bin/rpcrad-demo -addr 127.0.0.1:50052 -verify-demo-signature
# 另一个终端
cargo run --locked -- --plugin-smoke-test
```

验签模式使用公开测试 AppID `rpcrad-test` 与测试 secret `rpcrad-test-secret`。它通过 Go 标准库独立计算 MD5，要求毫秒时间戳与服务器时间相差不超过 60 秒；仅验证示例 user/order 方法。默认示例服务无需签名。
