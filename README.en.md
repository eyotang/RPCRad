# RPCRad

[简体中文](README.md) | [English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Русский](README.ru.md)

A local gRPC desktop workbench built with Rust, GPUI and GPUI-KIT 0.8. Real requests use the bundled grpcurl 1.9.3. The interface is native, without a WebView.

## Install and update

Download the package for your system from [GitHub Releases](https://github.com/eyotang/RPCRad/releases/latest).

| System | Architecture | Package |
| --- | --- | --- |
| macOS 15+ | Apple Silicon / Intel | Separate `.dmg` files |
| Windows 10/11 | x64 | `-setup.exe` installer or portable `.zip` |
| Ubuntu 24.04+ | x64 | `.deb` or portable `.tar.gz` |
| Omarchy / Arch Linux | x64 | `.pkg.tar.zst`, Wayland and X11 |

Packages include grpcurl, example protos and dependency licenses. Rust and Go are not required to run the application. Linux needs a working Vulkan driver and desktop portal; CJK text also needs appropriate fonts, such as `noto-fonts-cjk`. Windows and Linux ARM64 packages are not available.

Install Linux packages with `sudo apt install ./rpcrad_VERSION_amd64.deb` or `sudo pacman -U ./rpcrad-VERSION-1-x86_64.pkg.tar.zst`, replacing VERSION with the downloaded version. Portable packages must be extracted completely; keep the executable, grpcurl and resources together. See the [installation guide (Chinese)](docs/INSTALL.md) for details.

On macOS use **RPCRad → About RPCRad / Check for updates** in the system menu. On Windows/Linux use the **RPCRad** dropdown in the app. Updates are also checked at startup, using GitHub Releases, `latest.json` and SHA-256. Download and install the new package yourself; your workspace is preserved. There is no silent background replacement.

macOS packages currently lack Developer ID signing and notarization; Windows installers lack Authenticode signing. If macOS quarantine prevents opening the app, first verify that the download is from this repository and its SHA-256 matches. Only then, if needed, run:

```sh
xattr -dr com.apple.quarantine /Applications/RPCRad.app
open /Applications/RPCRad.app
```

Adjust the path if installed elsewhere. This is optional troubleshooting, not an update command. The application never runs it automatically.

## Interface languages

The language dropdown at the top supports **简体中文, English, 日本語, 한국어, Français, Deutsch and Русский**. Switching takes effect immediately and the preference is saved. New and older workspaces default to Simplified Chinese. You can also launch with `--language=zh-CN`, `en`, `ja`, `ko`, `fr`, `de` or `ru` (for example, `--language=ja`).

Application messages and built-in plugin descriptions are translated. User-defined names, variable values, request/response data and existing call snapshots stay unchanged. Server messages and custom plugins without translations are not automatically translated.

## Features and first request

Import a Proto or use server reflection, choose a method, enter the server address and send. **⌘ Enter** sends on macOS; **Ctrl+Enter** sends on Windows/Linux. **⌘/Ctrl+K** searches and **⌘/Ctrl+S** saves. Sidebars and editor columns can be resized.

- Service/method tree, search, favorites, tabs and saved request drafts.
- Native JSON editor with highlighting, formatting and proto-based examples.
- Unary and server-streaming calls, cancellation, timeouts, status and duration.
- Metadata, Bearer tokens and TLS with system-trusted CAs.
- Environments, global variables and header-signing plugins.
- Response body, headers, trailers, copy and up to 50 session history entries.

Open a history entry to inspect the actual body, connection settings, original headers and final metadata passed to grpcurl. Ordinary headers are visible; sensitive fields have individual reveal buttons. Copy follows the current visibility. The trash icon clears history without interrupting an active call. **Load request** restores address, body and connection settings; headers use current settings and plugins run again. A failed or cancelled pre-send request is explicitly marked unsent. Captured metadata does not include automatic HTTP/2 headers and is not proof the server received it.

Client/bidirectional streaming, custom CAs/mTLS and full proto import-root management are not yet supported. Response display is limited to 8 MiB.

## Environments and variables

Use the environment dropdown and **Variables** to create, rename or delete environments and edit values. Active environment values override globals of the same name. Without an environment, only globals apply. Reference variables in addresses, Metadata, Bearer tokens, plugin settings and Secret:

```text
Address: {{endpoint}}
Plugin AppID: {{appid}}
Plugin Secret: {{secret}}
```

```text
{
  "user_id": "{{user_id}}",
  "include_profile": {{include_profile}}
}
```

Quoted references are escaped as JSON strings; unquoted values must be valid JSON numbers, booleans, objects or arrays. Metadata values must be strings. Nested references are supported, for example `{{host}}:{{port}}`. Missing, duplicate or circular variables prevent sending. Names start with a letter or underscore and may contain digits, dots and hyphens.

Ordinary values are saved. Sensitive variable values, raw plugin secrets, authentication, Metadata and history are session-only. A complete `{{secret}}` reference is saved, but its sensitive value is not. Sensitive values deliberately used in a request body or header are sent and appear in that session’s call snapshot. Draft templates remain unresolved on disk.

## Header plugins

Select **MD5 · AppID + Timestamp**, enter AppID and Secret, then enable the plugin. By default, each send generates the current Unix timestamp in milliseconds and sends `appid`, `ts` and `sign`, where `sign = MD5(secret + appid + ts + secret)` in lowercase hex. The signature and timestamp use the same instant. Header names, time units and letter case are configurable.

HMAC-SHA256 and AES-256-GCM examples are included. Exactly one selected plugin runs per workspace call; chaining and per-method binding are not supported. Switching plugins clears Secret and preserves each plugin’s ordinary settings. Preview is a snapshot only; sending always recomputes. Inspect a new history entry to verify what was actually sent. See [plugin development (Chinese)](plugins/README.md) for local Rhai plugins and translation fields.

## Development

Requires Rust stable, Go, Python 3.11+ and native build tools. Setup uses the Go version pinned in `.go-version`.

```sh
python tools/setup.py
cargo run --locked
cargo test --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
python -m unittest discover -s tools/tests
```

Run `tools/bin/rpcrad-demo` for the local example at `127.0.0.1:50051`. Try GetUser, WatchUsers, or `user_id = missing` for NotFound. On macOS, `Launch RPCRad.command` can start the example and open the app. No account or cloud service is needed; startup checks contact GitHub without uploading request data.

Release packages are built natively on both Mac architectures, Windows and Ubuntu, with additional Arch/Wayland checks. Every release description contains Chinese and English sections plus the macOS xattr instructions. See [release workflow (Chinese)](docs/RELEASING.md) and [AGENTS.md](AGENTS.md).

Source: `src/ui.rs` (UI/workspace), `src/rpc.rs` (transport), `locales/` (translations), `examples/protos/` (examples), `tools/demo/` (test server). Design references: BloomRPC, Kreya and Yaak. Dependency licenses remain those of their respective upstream projects.
