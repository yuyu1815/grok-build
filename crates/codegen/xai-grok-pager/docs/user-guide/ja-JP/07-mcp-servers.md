<a id="mcp-servers"></a>

# MCP サーバー

MCP（Model Context Protocol）サーバーは、外部ツールとの連携によって Grok を拡張します。MCP 標準を実装するあらゆるサービスと Grok がやり取りできるようになります。

---

<a id="what-are-mcp-servers"></a>

## MCP サーバーとは

MCP サーバーは、標準化されたプロトコルを介して Grok にツールを公開するプロセスです。MCP サーバーを設定すると、そのツールを Grok の組み込みツールと併せてモデルが利用できるようになります。モデルはセッション中にこれらのツールを検出して呼び出せます。

たとえば、GitHub MCP サーバーは `create_issue`、`list_pull_requests`、`search_code` などのツールを公開できます。データベースサーバーは `query`、`list_tables`、`describe_schema` などを公開できます。

プロトコルの詳細については、[MCP 仕様](https://modelcontextprotocol.io)を参照してください。

---

<a id="configuration"></a>

## 設定

MCP サーバーは、`~/.grok/config.toml` の `[mcp_servers.<name>]` セクションで設定します。

<a id="stdio-transport-local-process"></a>

### stdio トランスポート（ローカルプロセス）

Grok はローカルプロセスを起動し、stdin/stdout を介して通信します。

```toml
[mcp_servers.my-server]
command = "/path/to/server"           # サーバーの実行ファイル
args = ["--flag", "value"]            # コマンド引数
env = { API_KEY = "sk-..." }          # 環境変数
enabled = true                        # サーバーを有効または無効にする（デフォルト: true）
startup_timeout_sec = 30              # サーバー起動タイムアウト（秒、デフォルト: 30）
tool_timeout_sec = 6000               # ツール呼び出しごとのフォールバックタイムアウト（秒、デフォルト: 6000）
tool_timeouts = { slow_op = 120 }     # ツールごとのタイムアウト上書き（秒）
```

> **グローバルな起動タイムアウトの上書き:** サーバーごとに `startup_timeout_sec`
> を設定する代わりに、`MCP_TIMEOUT` 環境変数（ミリ秒、Claude Code 互換）または
> `GROK_MCP_STARTUP_TIMEOUT_SECS`（秒）で、すべてのサーバーのデフォルト値を変更できます。
> サーバーごとの `startup_timeout_sec` は、引き続き両方より優先されます。初回起動時に
> パッケージをダウンロードするコールドスタートの `npx` / `uvx` サーバーでは、これが必要になることがよくあります。
> デフォルトは 30 秒です。
>
> **MCP ツール結果のサイズ上限:** 大きな MCP / `use_tool` の結果はインラインで切り詰められ
> （完全なペイロードはセッションの `mcp/` フォルダー以下に保存されます）、デフォルトは
> **20_000 バイト**です。次の方法で上書きできます。
>
> - 環境変数 `GROK_MAX_MCP_OUTPUT_BYTES` または `MAX_MCP_OUTPUT_BYTES`（バイト。両方が設定されている場合は
>   Grok ネイティブの名前が優先されます。後者は Claude 形式の名前ですが、上限の単位はトークンではなく**バイト**です）
> - `config.toml` — ユーザーレベル（`~/.grok/config.toml`）**またはリポジトリレベル**
>   （cwd から git ルートまでの経路上にある `.grok/config.toml`。最も深い位置の
>   ファイルが優先され、リポジトリの値はフォルダーが信頼された後にのみ適用されます）
>
> ```toml
> [mcp]
> max_output_bytes = 40000
> ```
>
> 優先順位: requirements.toml > 環境変数 > リポジトリの `.grok/config.toml` >
> ユーザー/管理対象設定 > デフォルト。リポジトリの編集内容は、設定のホットリロードにより、
> そのディレクトリで実行中のセッションにも適用されます。

<a id="httpsse-transport-remote-server"></a>

### HTTP/SSE トランスポート（リモートサーバー）

HTTP 経由でアクセスできるリモート MCP サーバーの場合は、次のように設定します。

```toml
[mcp_servers.remote-api]
url = "https://mcp.example.com/api"
headers = { "Authorization" = "Bearer token" }
```

<a id="streamable-http-with-session-id"></a>

### セッション ID を使用する Streamable HTTP

```toml
[mcp_servers.my-streamable-server]
url = "https://mcp.example.com/api/mcp"
headers = { "x-mcp-session-id" = "{{session_id}}" }
```

---

<a id="cli-management"></a>

## CLI での管理

設定ファイルを編集せずに、コマンドラインから MCP サーバーを管理できます。

```bash
# 設定済みの MCP サーバーを一覧表示
grok mcp list
grok mcp list --json          # 機械可読形式で出力

# stdio サーバーを追加する。-- 以降はすべてサーバーコマンドとして扱われるため、
# -y などのフラグは grok に解析されず、サーバーへ渡される。
grok mcp add filesystem -- npx -y @modelcontextprotocol/server-filesystem /path/to/dir

# 環境変数を指定して stdio サーバーを追加する（-e は繰り返し指定可能）
grok mcp add postgres -e DATABASE_URL=postgres://localhost/mydb -- npx -y @modelcontextprotocol/server-postgres

# リモート HTTP サーバーを追加
grok mcp add --transport http sentry https://mcp.sentry.dev/mcp

# 認証ヘッダーを指定してリモートサーバーを追加する（--header は繰り返し指定可能）
grok mcp add --transport http api https://mcp.example.com/mcp --header "Authorization: Bearer YOUR_TOKEN"

# リモート SSE サーバーを追加
grok mcp add --transport sse linear https://mcp.linear.app/sse

# サーバーを削除
grok mcp remove github

# サーバーの設定と接続を診断
grok mcp doctor               # 設定済みの全サーバーを確認
grok mcp doctor github        # 1 台のサーバーを確認
grok mcp doctor --json        # 機械可読形式で出力
```

デフォルトのトランスポートは `stdio` です。リモートサーバーには `--transport http` または `--transport sse` を渡します。

デフォルトでは、`grok mcp add` は `~/.grok/config.toml` に書き込みます（`--scope user`）。代わりに現在のディレクトリの `.grok/config.toml` へ書き込むには `--scope project` を使用します。このファイルはコミットしてチームと共有できます（[プロジェクトスコープの MCP サーバー](#project-scoped-mcp-servers)を参照）。ヘッダーと環境変数の値はそのまま保存されるため、コミットするプロジェクト設定にシークレットを直接貼り付けず、`${VAR}` で参照してください（[設定例](#example-configurations)を参照）。`grok mcp list` は両方のスコープのサーバーを表示し、プロジェクトスコープのサーバーには `(project)` を付けます。

`grok mcp remove` は両方のスコープを検索し、サーバーを削除すると終了コード 0 で終了します。名前が見つからない場合、または同じ名前がユーザーとプロジェクトの両方のスコープで定義されている場合は、終了コード 1 で終了します。後者の場合は `--scope` を渡して削除対象を指定してください。

以前のリリースからの破壊的変更: `--env` はフラグ 1 つにつき `KEY=value` を 1 つ受け取るようになりました（`--env A=1 B=2` ではなく `-e A=1 -e B=2` を使用）。また、サーバー名に使用できるのは英字、数字、ハイフン、アンダースコアだけです。

---

<a id="project-scoped-mcp-servers"></a>

## プロジェクトスコープの MCP サーバー

リポジトリに `.grok/config.toml` を配置すると、プロジェクト単位で MCP サーバーを設定できます。

```
my-project/
  .grok/
    config.toml
  src/
  ...
```

```toml
# .grok/config.toml
[mcp_servers.linear]
url = "https://mcp.linear.app/mcp"
enabled = true
```

サーバーがネイティブの HTTP/SSE エンドポイントを公開している場合は、`npx mcp-remote <url>` のような stdio プロキシでラップせず、`url` 形式を優先してください。Grok は HTTP/SSE と OAuth を直接処理するため、ネイティブ形式ならセッションごとの余分なサブプロセスを回避できます。また、プロバイダーには Grok 独自の OAuth クライアントが登録されます。

Grok は現在のディレクトリから git リポジトリのルートまで上位へたどり、各階層の `.grok/config.toml` を読み込みます。

| 場所 | スコープ | 優先度 |
|----------|-------|----------|
| `~/.grok/config.toml` | すべてのプロジェクト | 最低 |
| `<repo-root>/.grok/config.toml` | このリポジトリ | 中 |
| `<cwd>/.grok/config.toml` | 現在のディレクトリ | 最高 |

プロジェクトでグローバル設定と同じ名前のサーバーを定義すると、グローバル設定はプロジェクト版に完全に置き換えられます（フィールドはマージされません）。

プロジェクトスコープのファイルからは、`[mcp_servers]`、`[plugins]`、`[permission]` のエントリが読み込まれます。その他のほとんどの設定セクションは、`~/.grok/config.toml` からのみ読み込まれます。

---

<a id="tool-naming"></a>

## ツール名

名前の衝突を避けるため、MCP ツールにはサーバー名の名前空間が付与されます。

- サーバー `filesystem` のツール `read_file` は `filesystem__read_file` になる
- サーバー `github` のツール `create_issue` は `github__create_issue` になる

---

<a id="toggle-servers-at-runtime"></a>

## 実行時のサーバーの有効／無効切り替え

Grok を再起動せずに、セッション中に MCP サーバーを有効または無効にできます。

<a id="the-mcps-modal"></a>

### /mcps モーダル

TUI で MCP サーバーのモーダルを開きます。

- スラッシュコマンドとして `/mcps` を実行する
- または `Ctrl+L`（VS Code 系以外）を押して MCP Servers タブへ移動する。VS Code 系では `/plugins` または `/mcp` を使用して MCP Servers タブを開く

モーダルでは次の操作ができます。

- 各サーバーの取得元、有効状態、ツール数を確認する
- `Space` でサーバーを有効または無効にする
- サーバーを展開して、提供されるツールを表示する
- `config.toml` の編集後に `r` で一覧を更新する
- `i` で OAuth サーバーを認証する
- `a` でサーバーを追加し、`x` で削除する

<a id="tool-discovery"></a>

### ツールの検出

モデルは、MCP サーバーを操作するための 2 つの組み込みツールを利用できます。

- `search_tool` — 有効なすべての MCP サーバーから、利用可能な連携ツールを検出します。名前や説明でツールを検索する際に使用します。
- `use_tool` — `search_tool` で検出した連携ツールを呼び出します。完全修飾ツール名（例: `github__create_issue`）を指定します。

---

<a id="compatibility"></a>

## 互換性

互換性のため、Grok は複数の取得元から MCP サーバー設定を読み込みます。

| 取得元 | 形式 | 場所 | 設定方法 |
|--------|--------|----------|-------------|
| `config.toml` | Grok ネイティブ設定 | `~/.grok/config.toml`、`.grok/config.toml` | 常に有効 |
| `.claude.json` | Claude Code 形式 | `~/.claude.json` | `[compat.claude] mcps` |
| `.cursor/mcp.json` | Cursor 形式 | `~/.cursor/mcp.json`、`<project>/.cursor/mcp.json` | `[compat.cursor] mcps` |
| `.mcp.json` | MCP 標準形式 | プロジェクトルート（cwd から git ルートまで） | Claude のインポート案内でインポートを実行したか、案内を閉じた場合（インポートマーカーが設定されている場合）を除き読み込まれる |

すべての取得元は、config.toml > Claude > Cursor > `.mcp.json` の優先順位でマージされます。名前が競合した場合は、優先度の高い取得元のサーバーが優先されます。

Claude と Cursor の MCP 取得元はデフォルトでスキャンされます。特定ベンダーのスキャンを無効にするには、`~/.grok/config.toml` で `[compat.<vendor>] mcps = false` を設定するか、対応する環境変数（`GROK_CURSOR_MCPS_ENABLED`、`GROK_CLAUDE_MCPS_ENABLED`）を設定します。詳細については、[設定](05-configuration.md#harness-compatibility)を参照してください。読み込まれた MCP サーバーとベンダーの取得元（`[cursor]`、`[claude]`）を確認するには、`grok inspect` を使用します。

---

<a id="mcp-oauth"></a>

## MCP OAuth

OAuth 認証が必要な MCP サーバーでは、Grok が OAuth 認証フローを自動的に処理します。MCP サーバーが OAuth 認証情報を要求すると、Grok はブラウザベースの認可フローを開き、取得したトークンを今後の利用のために保存します。

---

<a id="example-configurations"></a>

## 設定例

ホスト型 MCP サーバーには `url` 形式を、ローカルの stdio ツールには `command` / `args` 形式を使用します。

<a id="native-http-hosted-services"></a>

### ネイティブ HTTP（ホスト型サービス）

OAuth ベースの MCP サーバーは、使用前に認証する必要があります。Grok は取得したトークンを `~/.grok/mcp_credentials.json` に保存します。`config.toml` を編集した後は、`/mcps` モーダルで `r` を押してサーバー一覧を更新してください。

```toml
[mcp_servers.linear]
url = "https://mcp.linear.app/mcp"
enabled = true

[mcp_servers.sentry]
url = "https://mcp.sentry.dev/mcp"
enabled = true

[mcp_servers.mixpanel]
url = "https://mcp.mixpanel.com/mcp"
enabled = true
```

OAuth ではなく静的な bearer トークンで認証する社内サーバーまたはセルフホスト型サーバーでは、`Authorization` ヘッダーを明示的に設定します。

```toml
[mcp_servers.internal-tools]
url = "https://mcp.internal.example.com/mcp"
enabled = true

[mcp_servers.internal-tools.headers]
Authorization = "Bearer <token>"
```

設定ファイルにシークレットを記載しないよう、`${VAR}`（または `${VAR:-default}`）で環境変数を参照できます。Grok は読み込み時に、`[mcp_servers.*]` 内の文字列フィールド（`url`、`command`、`args`、および `env` と `headers` の値）を展開します。

```toml
[mcp_servers.internal-tools]
url = "https://mcp.internal.example.com/mcp"
enabled = true
headers = { "Authorization" = "Bearer ${INTERNAL_MCP_TOKEN}" }
```

<a id="local-stdio"></a>

### ローカル stdio

ローカルで実行する必要があるツール（ファイルシステムへのアクセス、ローカルデータベース、社内サーバー）には stdio を使用します。

```toml
# アクセス範囲をディレクトリに限定したファイルシステム
[mcp_servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/allowed/directory"]

# ローカル Postgres
[mcp_servers.postgres]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-postgres", "postgresql://user:pass@localhost/db"]

# 起動タイムアウトを延長し、ツールごとのタイムアウトを調整したカスタムサーバー
[mcp_servers.my-tools]
command = "/usr/local/bin/my-mcp-server"
args = ["--config", "/etc/my-mcp.json"]
startup_timeout_sec = 30
tool_timeout_sec = 120
tool_timeouts = { slow_analysis = 300, quick_lookup = 10 }
```

Windows では、npm は `npx`、`npm`、`pnpm`、`yarn` などのランチャーを `.cmd` バッチシムとしてインストールします（`npx.exe` はありません）。Grok はプロセスを起動する前に、`npx` のようにパスを含まない `command` を、`PATH` 上の実際のランチャーパスへ解決します（`PATHEXT` も考慮します）。そのため、手動で `cmd /c` にラップしなくても動作します。絶対パス、またはパス区切り文字を含む `command` は、そのまま使用されます。

---

<a id="available-mcp-servers"></a>

## 利用可能な MCP サーバー

上記の `url` または `command` 形式で設定できる MCP サーバーの一部を以下に示します。使用前に、現在のエンドポイントまたはパッケージ名を各プロバイダーで確認してください。

| サーバー | トランスポート | エンドポイント / パッケージ |
|--------|-----------|--------------------|
| Linear | HTTP (OAuth) | `https://mcp.linear.app/mcp` |
| Sentry | HTTP (OAuth) | `https://mcp.sentry.dev/mcp` |
| Mixpanel | HTTP (OAuth) | `https://mcp.mixpanel.com/mcp` |
| Filesystem | stdio | `@modelcontextprotocol/server-filesystem` |
| Git | stdio | `@modelcontextprotocol/server-git` |
| GitHub | stdio | `@modelcontextprotocol/server-github` |
| GitLab | stdio | `@modelcontextprotocol/server-gitlab` |
| PostgreSQL | stdio | `@modelcontextprotocol/server-postgres` |
| SQLite | stdio | `@modelcontextprotocol/server-sqlite` |
| Puppeteer | stdio | `@modelcontextprotocol/server-puppeteer` |

コミュニティサーバーの全一覧については [MCP Server Registry](https://github.com/modelcontextprotocol/servers)を、プロトコルの詳細については [MCP 仕様](https://modelcontextprotocol.io)を参照してください。

---

<a id="troubleshooting"></a>

## トラブルシューティング

<a id="server-not-starting"></a>

### サーバーが起動しない

```bash
# サーバーコマンドを手動でテスト
npx -y @modelcontextprotocol/server-filesystem /path

# 起動タイムアウトを延長
# config.toml 内:
[mcp_servers.filesystem]
startup_timeout_sec = 30
```

stdio サーバーでは、Grok はプロセスの標準エラーを `~/.grok/logs/mcp/<server>.stderr.log` に記録し、起動のたびに切り詰めます。サーバーは起動するもののハンドシェイクに失敗する場合は、このファイルを確認してください。

```bash
tail -f ~/.grok/logs/mcp/filesystem.stderr.log
```

<a id="viewing-server-status"></a>

### サーバー状態の表示

読み込まれたすべての MCP サーバーとその取得元を確認するには、`grok inspect` を使用します。

```bash
grok inspect          # 人が読みやすい形式
grok inspect --json   # 機械可読形式
```

<a id="debug-logging"></a>

### デバッグログ

```bash
RUST_LOG=debug GROK_LOG_FILE=/tmp/grok.log grok
tail -f /tmp/grok.log
```

サーバーの起動、ツールの検出、ツール呼び出しの実行を追跡するには、`mcp` を含むログエントリを確認してください。
