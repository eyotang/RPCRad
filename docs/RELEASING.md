# 发布与更新

RPCRad 使用 Metra 当前的 macOS 发布契约：GitHub Release + `latest.json` + DMG + SHA-256。应用仍使用 GPUI，不迁移到 Tauri。

## 自动发布

1. 修改 `Cargo.toml` 版本，运行 `cargo check` 更新 `Cargo.lock`，添加 `docs/releases/vX.Y.Z.md`。
2. 运行格式、测试、Clippy 和 `python3 -m unittest discover -s tools/tests`。
3. 提交到 `main`，创建并推送对应的 `vX.Y.Z` 标签。
4. GitHub Actions 先验证，再构建 DMG、校验和与 `latest.json`，执行打包程序的实际 gRPC 验签测试，最后上传到草稿 Release 并发布为 latest。版本号必须完全一致。

工作流仅使用仓库的 `GITHUB_TOKEN`（发布任务需要 contents:write）。首版仅发布 macOS arm64，不需要也不复用 Metra 的 Windows 签名私钥。已发布版本禁止覆写；修复请递增版本。失败在草稿阶段可重新运行同一标签任务。

## 本地构建

需要 Apple Silicon Mac、macOS Command Line Tools、Rust stable、Go、Python 3.11+。

```sh
sh tools/setup.sh
python3 tools/release.py --tag v0.1.0
```

产物位于 `release-assets/v0.1.0/`。脚本检查资源集合、版本、DMG 和 SHA-256；输出目录出现额外文件会阻止生成清单。应用包不包含本机 `.rpcrad`、配置、密钥或调用记录。

应用内“设置 → 检查更新”读取 `https://github.com/eyotang/RPCRad/releases/latest/download/latest.json`。只有更高的稳定版本会触发通知；HTTPS 超时、无效版本、超大清单会显示错误。下载按钮只打开此仓库的对应版本页。

macOS 安装包当前采用 ad-hoc 签名，没有 Developer ID 或 Apple 公证。若以后增加自动替换安装或 Windows 安装器，需要单独配置 RPCRad 的签名与验证链，不能仅凭清单中的下载地址执行文件。

## 本地数据

macOS 工作区位置：`~/Library/Application Support/RPCRad/workspace.json`。示例 Proto 使用与安装路径无关的引用保存。插件 Secret、Bearer Token、Metadata 和调用历史不持久化。设置 `RPCRAD_DATA_DIR` 可在测试时使用隔离工作区。
