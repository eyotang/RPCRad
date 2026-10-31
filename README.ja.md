# RPCRad

[简体中文](README.md) | [English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Русский](README.ru.md)

Rust、GPUI、GPUI-KIT 0.8 で構築したローカル gRPC デスクトップデバッグツールです。実際の通信には同梱の grpcurl 1.9.3 を使用します。WebView を使わないネイティブ UI です。

## インストールと更新

[GitHub Releases](https://github.com/eyotang/RPCRad/releases/latest) から OS と CPU に合うパッケージをダウンロードしてください。

| OS | アーキテクチャ | パッケージ |
| --- | --- | --- |
| macOS 15 以降 | Apple Silicon / Intel | 個別の `.dmg` |
| Windows 10/11 | x64 | `-setup.exe` またはポータブル `.zip` |
| Ubuntu 24.04 以降 | x64 | `.deb` または `.tar.gz` |
| Omarchy / Arch Linux | x64 | `.pkg.tar.zst`、Wayland / X11 対応 |

grpcurl、サンプル Proto、依存ライブラリのライセンスを同梱しています。利用時に Rust や Go は不要です。Linux では Vulkan ドライバーと正常なデスクトップ portal が必要です。日本語などの表示には `noto-fonts-cjk` などを導入してください。Windows/Linux ARM64 パッケージはありません。

Ubuntu は `sudo apt install ./rpcrad_VERSION_amd64.deb`、Arch は `sudo pacman -U ./rpcrad-VERSION-1-x86_64.pkg.tar.zst` でインストールします。VERSION を実際のバージョンに置き換えてください。ポータブル版は全体を展開し、実行ファイル・grpcurl・resources を一緒に配置します。詳しくは[インストール説明（中国語）](docs/INSTALL.md)をご覧ください。

macOS はシステムメニューの **RPCRad → RPCRad について / 更新を確認**、Windows/Linux はアプリ上部の **RPCRad** メニューから操作します。起動時にも更新を確認します。GitHub Releases、`latest.json`、SHA-256 を使用し、新しいパッケージは利用者がダウンロードしてインストールします。ワークスペースは維持され、バックグラウンドでの自動置換は行いません。

macOS 版は Developer ID 署名・公証、Windows 版は Authenticode 署名に未対応です。macOS の隔離によって起動できない場合、公式リポジトリからのダウンロードであることと SHA-256 の一致を確認したうえで、必要に応じて実行してください。

```sh
xattr -dr com.apple.quarantine /Applications/RPCRad.app
open /Applications/RPCRad.app
```

別の場所にインストールした場合はパスを変更します。これは更新コマンドではなく、必要な場合のみ行う対処です。アプリが自動実行することはありません。

## 表示言語

**RPCRad → 設定… → 言語** で **简体中文 / English / 日本語 / 한국어 / Français / Deutsch / Русский** を即時に切り替えられます。選択は保存され、再起動後も有効です。**自動検出（システム）** が新規および言語未設定のワークスペースの既定値です。起動時または自動検出を選んだとき、システムの優先順で対応言語を選びます。地域別の設定は該当言語に対応し、中国語は簡体字を使用します。対応言語がない場合や検出失敗時は英語になります。既存の手動選択は保持され、自動検出時は現在の言語も表示します。`--language=auto` でも有効にできます。起動引数は `--language=ja` など、`zh-CN`、`en`、`ja`、`ko`、`fr`、`de`、`ru` を指定できます。

設定は独立したウィンドウで開き、テーマは **システムに従う（既定）/ ライト / ダーク** から選べます。変更は即時反映・自動保存され、システムに従う場合は OS の外観変更に追従します。更新確認とアプリ情報も独立したウィンドウで開き、再度開くと既存のウィンドウが前面に表示されます。リクエスト画面は置き換えません。設定を開くには macOS で **⌘ ,**、Windows/Linux で **Ctrl+,**、補助ウィンドウを閉じるには **Esc** または **⌘/Ctrl+W** を使います。

アプリ文言と内蔵プラグインの説明を翻訳します。ユーザーが付けた名前、変数の値、リクエスト・レスポンス、既存の履歴スナップショットは変更しません。サーバーのメッセージや翻訳を持たない外部プラグインは自動翻訳しません。

## 機能と最初のリクエスト

Proto のインポートまたはサービスリフレクションでメソッドを選び、接続先を入力して送信します。macOS は **⌘ Enter**、Windows/Linux は **Ctrl+Enter** で送信、**⌘/Ctrl+K** で検索、**⌘/Ctrl+S** で保存できます。サイドバーと編集領域の幅は変更可能です。

- サービス・メソッドツリー、検索、お気に入り、タブ、リクエスト草稿。
- JSON の構文強調、整形、Proto に基づくサンプル生成。
- Unary とサーバーストリーミング、キャンセル、タイムアウト、状態・所要時間。
- Metadata、Bearer トークン、システム CA による TLS 検証。
- 環境・グローバル変数、リクエストヘッダー署名プラグイン。
- レスポンス本文・Headers・Trailers、コピー、セッション内最大 50 件の履歴。

履歴を開くと、送信時の本文、接続設定、処理前ヘッダーと grpcurl に渡した最終 Metadata を確認できます。通常のヘッダーは表示し、機密項目のみ個別に表示・非表示を切り替えます。コピー内容は現在の表示と一致します。ゴミ箱で履歴を消去しても実行中の呼び出しは中断しません。「リクエストを読み込む」はアドレス・本文・接続設定を復元し、ヘッダーは現在の設定、署名は次回送信時の再計算を使用します。

送信前の失敗・キャンセルは未送信と明示します。記録に HTTP/2 の自動ヘッダーは含まれず、記録されたこと自体はサーバーへの到達を保証しません。クライアント・双方向ストリーミング、カスタム CA/mTLS、完全な import-root 管理は未対応です。レスポンス表示は最大 8 MiB です。

## 環境と変数

環境メニューと「変数」で、環境の作成・名前変更・削除と値の編集を行います。現在の環境が同名のグローバル変数に優先します。環境未選択時はグローバル変数だけを使います。

アドレスに `{{endpoint}}`、プラグイン AppID に `{{appid}}`、Secret に `{{secret}}` を指定できます。本文の例：

```text
{
  "user_id": "{{user_id}}",
  "include_profile": {{include_profile}}
}
```

引用符内の参照は JSON 文字列としてエスケープします。引用符なしでは有効な JSON の数値・真偽値・オブジェクト・配列が必要です。Metadata の値は文字列にしてください。`{{host}}:{{port}}` のような入れ子参照も使えます。未定義・重複・循環参照は送信を停止します。変数名は英字またはアンダースコアで開始し、数字・ドット・ハイフンを含められます。

通常変数は保存します。機密変数の値、Secret の平文、認証、Metadata、履歴はセッション内のみです。`{{secret}}` という参照は保存しますが、機密値は再起動後に再入力します。機密値を本文やヘッダーに参照した場合、その値は送信され、当該セッションの履歴にも含まれます。ディスク上の草稿はテンプレートのままです。

## ヘッダープラグイン

「MD5 · AppID + タイムスタンプ」を選択し、AppID と Secret を入力して有効化します。既定では現在の Unix ミリ秒を `ts` とし、`MD5(secret + appid + ts + secret)` の小文字 16 進数を `sign` に設定して、`appid`・`ts`・`sign` を送信します。同じ時刻を署名とヘッダーに使います。ヘッダー名、単位、文字ケースを変更できます。

HMAC-SHA256 と AES-256-GCM のサンプルも付属します。ワークスペースの呼び出しで実行するのは選択中の 1 プラグインのみで、連結やメソッド別の割り当ては未対応です。切り替えると Secret は消去され、通常設定はプラグインごとに保存されます。プレビューの結果は再利用せず、送信ごとに計算します。送信内容は新しい履歴で確認してください。[プラグイン開発説明（中国語）](plugins/README.md)に Rhai と翻訳方法を記載しています。

## 開発と検証

Rust stable、Go、Python 3.11 以降、OS のネイティブビルドツールが必要です。Go のバージョンは `.go-version` で固定します。

```sh
python tools/setup.py
cargo run --locked
cargo test --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
python -m unittest discover -s tools/tests
```

`tools/bin/rpcrad-demo` で `127.0.0.1:50051` のサンプルサーバーを起動できます。GetUser、WatchUsers、`user_id = missing` による NotFound を試せます。macOS の `Launch RPCRad.command` はサンプルとアプリを起動します。アカウントやクラウドは不要です。起動時に GitHub の更新を確認しますが、リクエスト内容はアップロードしません。

各リリースは Mac 2 アーキテクチャ、Windows、Ubuntu、Arch/Wayland で検証します。リリース説明には毎回中国語・英語と xattr の説明を含めます。[公開手順（中国語）](docs/RELEASING.md)、[AGENTS.md](AGENTS.md)も参照してください。主なコードは `src/ui.rs`、`src/rpc.rs`、翻訳は `locales/`、サンプルは `examples/protos/` と `tools/demo/` です。設計参考：BloomRPC、Kreya、Yaak。依存ライブラリのライセンスは各提供元に従います。
