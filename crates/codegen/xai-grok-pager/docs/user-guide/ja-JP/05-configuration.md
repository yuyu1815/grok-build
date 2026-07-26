<a id="configuration"></a>

# 設定

Grok はローカル設定ファイル、環境変数、CLI フラグから設定を読み込みます。このドキュメントでは、一般的なオプションについて説明します。

---

<a id="precedence"></a>

## 優先順位

設定は次の順序で解決されます（上ほど優先）。

1. **CLI フラグ**（例: `--yolo`、`--model`、`--sandbox`）
2. **環境変数**（例: `XAI_API_KEY`、`GROK_MEMORY`）
3. **config.toml**（`~/.grok/config.toml`）
4. **管理対象 / requirements 設定**（組織が配布する場合があるローカルファイル。例:
   `managed_config.toml` / `requirements.toml`）
5. **組み込みのデフォルト値**

---

<a id="configtoml-main-configuration"></a>

## config.toml（メイン設定）

場所: `~/.grok/config.toml`

ファイルが存在しない場合、Grok は組み込みのデフォルト値を使用します。上書きしたい値だけを指定してください。

<a id="general-settings"></a>

### 一般設定

```toml
[cli]
auto_update = true                     # 起動時に更新を確認

[models]
default = "grok-build"           # 新しいセッションで使用するモデル
web_search = "grok-4.20-multi-agent"   # web_search ツールが使用するモデル

# すべてのモデルに適用するデフォルト値。モデルごとの [model.<id>] の値が常に優先される。
# モデルごとの上書きと詳細については「カスタムモデル」を参照。
extra_headers = { "X-Request-Tags" = "team=example,env=prod" }
temperature = 0.7
top_p = 0.95
max_completion_tokens = 8192
max_retries = 8
inference_idle_timeout_secs = 600
stream_tool_calls = true

[ui]
language = "ja-JP"                    # 表示言語: "en-US" または "ja-JP"（再起動後に適用）
simple_mode = true                      # readline 形式のプロンプト編集（デフォルト）。false = プロンプトで vim 編集
vim_mode = false                       # vim 形式のスクロールバック移動キー（デフォルト: false）
max_thoughts_width = 120               # 推論表示の最大列幅
default_selected_permission = "always_allow_all_sessions" # 最初の権限確認で事前選択する行
remember_tool_approvals = false        # 権限確認にコマンドごとの「常に許可」を表示。
                                       # 許可はプロジェクト単位で記憶される（デフォルト: false）。22-permissions-and-safety.md を参照
show_thinking_blocks = true            # TUI にエージェントの思考ブロックを表示（デフォルト: true）
group_tool_verbs = true                # read/search/list ツール呼び出しとサブエージェント行の連続、および
                                       # その間に完了した思考を 1 行にまとめる（デフォルト: true）
collapsed_edit_blocks = false          # 編集を 1 行の +N/-M diffstat 要約で表示し、展開すると
                                       # diff を表示（デフォルト: false。pager.toml の [scrollback.blocks.edit]
                                       # expanded_by_default/line_summary が設定されている場合はそちらが優先）
screen_mode = "fullscreen"             # 固定される描画モード: "minimal" または "fullscreen"。
                                       # --minimal/--fullscreen および /minimal//fullscreen により自動的に書き込まれる

[features]
telemetry = false                      # 匿名の使用状況テレメトリ
feedback = true                        # フィードバックシステム（デフォルト: true）
lsp_tools = false                      # lsp ツールを公開
codebase_indexing = true               # コードグラフのインデックス作成
two_pass_compaction = false            # 事前実行型の 2 パス圧縮（デフォルト: false、オプトイン）
remote_fetch = true                    # オンラインのモデルカタログ取得を許可（デフォルト: true。
                                       # ファイアウォール内またはエアギャップ環境では false に設定。
                                       # バックグラウンドの管理対象設定同期には独自のスイッチ managed_config がある）

[session]
auto_compact_threshold_percent = 85    # コンテキストウィンドウ使用率がこの割合に達したら自動圧縮
load_envrc = true                      # .envrc の環境変数を読み込む

[tools]
respect_gitignore = false              # デフォルト: false。true にするとすべてのツールが gitignore 対象ファイルをスキップ
```

<a id="display-language"></a>

#### 表示言語

`[ui].language` には `"en-US"` または `"ja-JP"` を指定します。Grok は起動時に一度だけ、次の優先順位で active locale を解決します。

```text
GROK_LANG > [ui].language > OS locale > en-US
```

無効または未対応の値は読み飛ばされ、次の情報源が使われます。どの情報源も対応ロケールへ解決できない場合は `en-US` にフォールバックします。変更は Grok の再起動後に適用されます。明示設定がない場合、日本語 OS では日本語が使われます。従来どおり常に英語で使用するには、`GROK_LANG=en-US` または `[ui].language = "en-US"` を設定してください。

```toml
[ui]
language = "ja-JP"
```

`GROK_LANG` は、そのプロセスに限り設定ファイルより優先されます。英語へ固定する例:

```powershell
# PowerShell
$env:GROK_LANG = "en-US"
grok
```

```bash
# Unix シェル
GROK_LANG=en-US grok
```

active locale は UI だけでなく、起動時に `~/.grok/docs/user-guide/` へ抽出される組み込みガイドにも適用されます。このディレクトリの番号付き Markdown ファイルは Grok の管理対象であり、起動のたびに active locale 版へ更新されます。モデルは TUI の利用方法に関する質問へ回答する際にこのガイドを読むことがあるため、モデル向けガイドも locale に依存します。

機械可読値と canonical 値は翻訳されません。コマンド名、オプション名、設定キーと値、JSON とプロトコルのフィールド、モデル ID は canonical のままです。特に、ガイドの locale を変更しても、モデル ID やプロトコル値は変わりません。人間向け CLI 出力を解析するスクリプトでは、`GROK_LANG=en-US`（または想定する対応ロケール）を設定してください。機械可読の出力形式がある場合は、そちらを優先してください。

<a id="input-mode"></a>

#### 入力モード

`[ui]` の `simple_mode` 設定は、**プロンプト**（入力エディター）でのテキスト編集方法を制御します。スクロールバックの移動方法は変更しません。そちらは別の [`vim_mode`](#vim-mode) で制御します。

| 値 | 動作 |
|-------|----------|
| `true`（デフォルト） | **Readline 編集。** プロンプトでは通常の readline 形式でテキストを入力します。 |
| `false` | **Vim 編集（実験的）。** プロンプトでは vim 形式のモーダル編集（ノーマルモードと挿入モード）を使用します。プロンプトが空の場合は、スクロールバックにフォーカスしたノーマルモードで開始します。 |

プロンプトを vim 形式の編集へ切り替えるには、次のように設定します。

```toml
[ui]
simple_mode = false
```

設定ペイン（`/settings` → **Disable vim input mode**）から切り替えることもできます。Grok は選択内容を `config.toml` の `[ui] simple_mode` に書き込みます。

`simple_mode` と `vim_mode` は独立しています。`simple_mode` はプロンプトエディターを、`vim_mode` はスクロールバックの移動方法を変更します。すべてのキーバインドについては、[キーボードショートカット](03-keyboard-shortcuts.md)を参照してください。

<a id="default-selected-permission"></a>

#### デフォルトで選択する権限

エージェントがコマンド（またはその他のツール操作）の実行許可を求めると、承認メニューではデフォルトで 1 行（カーソル行）が強調表示されます。`[ui]` の `default_selected_permission` 設定は、セッションの**最初の**確認でどの行を選択するかを制御します。

| 値 | 事前選択される行 |
|-------|-----------------|
| `always_allow_all_sessions`（デフォルト） | 「すべてのセッションで常に許可」の行。 |
| `allow_command_always` | 「このコマンドを常に許可」の行。 |
| `allow_once` | 「はい」/ 1 回だけ許可する行。 |
| `reject` | 拒否する行。 |

```toml
[ui]
default_selected_permission = "allow_once"
```

最初の確認に回答すると、カーソル位置は**固定的に引き継がれます**。以降の確認では、最後に確定したものと同種の選択肢が事前選択されます（たとえば「いいえ」を選ぶと、それ以降は拒否行から始まります）。この状態は edit / bash / MCP の確認をまたいで、再起動するまで維持されます。そのため、`default_selected_permission` が設定するのは開始位置だけです。

使用できる値は `always_allow_all_sessions`、`allow_command_always`、`allow_once`、`reject` です（大文字と小文字は区別しません）。キーが未設定、または認識できない値の場合は `always_allow_all_sessions` にフォールバックします。`allow_command_always` の行は、承認対象の特定操作（command / tool / domain / edit-session）だけに適用され、すべてを許可するグローバル設定ではありません。グローバル設定は `always_allow_all_sessions` です。なお、コマンドごとの「常に許可」行は `[ui] remember_tool_approvals = true`（デフォルト: false）の場合のみ表示されます。[22-permissions-and-safety.md](22-permissions-and-safety.md)を参照してください。

この設定は環境変数 `GROK_DEFAULT_SELECTED_PERMISSION` でも上書きできます。`config.toml` を変更したくないヘッドレス実行やエージェントのテストに便利です。優先順位: 環境変数 → `config.toml` → `always_allow_all_sessions`（デフォルト）。

<a id="vim-mode"></a>

#### Vim モード

`[ui]` の `vim_mode` 設定は、**スクロールバック**ペインで vim 形式のキーバインドを有効にするかを制御します。入力プロンプトには影響しません。

| 値 | 動作 |
|-------|----------|
| `false`（デフォルト） | スクロールバックでは、修飾キーなしの文字キーと `Shift+letter`（`j`/`k`、`h`/`l`、`g`/`G`、`y`/`Y`、`o`/`O`、`r`、`x`、`e`/`E`、`H`/`L`、および `i`）が無効になります。これらの文字を押すとプロンプトにフォーカスし、その文字が入力されます。矢印、`Tab`、`Space`、`PageUp`/`PageDown`、すべての `Ctrl+letter` ショートカットでは、引き続きスクロールバックを移動できます。`Esc` はスクロールバック移動キーではなく、クリア / 巻き戻し / ターン途中の入力破棄ポリシーに従います（[キーボードショートカット](03-keyboard-shortcuts.md#escape)を参照）。 |
| `true` | [キーボードショートカット](03-keyboard-shortcuts.md)に記載された vim 形式のスクロールバック用キーバインドがすべて有効になります。 |

`/vim-mode`、または設定ペイン（`/settings` → **Vim scrollback navigation**）で実行時に `vim_mode` を切り替えられます。Grok は変更を `~/.grok/config.toml` の `[ui] vim_mode` に即座に書き込み、同じプロセスで開始する新しいエージェントやサブエージェントを含め、今後のすべての pager セッションへ適用します。セッション単位の上書きはありません。次回起動時は `config.toml` が正となります。

`vim_mode` と `simple_mode` は独立しています。`vim_mode` はスクロールバックの移動を、`simple_mode` はプロンプトでの編集を制御します。

<a id="screen-mode"></a>

#### 画面モード

`[ui]` の `screen_mode` 設定は、**保持される描画モードの設定**です。最後に明示的に選択したモードで、次回の通常の `grok` が開きます。

| 値 | 動作 |
|-------|----------|
| 未設定（デフォルト） | 従来の解決順序: pager.toml の `[terminal] minimal`、次に代替画面ポリシー。 |
| `"minimal"` | minimal（スクロールバックネイティブ）モードで開く。 |
| `"fullscreen"` | 標準 TUI で開く。fullscreen と inline の選択は引き続き代替画面ポリシー（`--no-alt-screen`、`[terminal] alt_screen`、ターミナルの自動検出）に従うため、Zellij や tmux control mode などの環境では自動的に inline へフォールバックします。 |

通常、このキーを手動で編集する必要はありません。明示的な `--minimal` / `--fullscreen` フラグを渡すか、`/minimal` / `/fullscreen` を実行すると、Grok が書き込みます。通常の `grok` 起動では読み取りだけを行います。その起動では CLI フラグが常に設定値より優先され（同時に設定も更新され）、`screen_mode` は `pager.toml` の従来の `[terminal] minimal` キーより優先されます。従来の動作へ戻すには、このキーを削除してください。

<a id="scrolling"></a>

#### スクロール

4 つの `[ui]` 設定で、スクロールバックにおけるマウスホイールとトラックパッドのスクロールを調整できます。すべて即座に適用され（再起動不要）、設定ペイン（`/settings` → **Scroll speed** / **Scroll input** / **Scroll lines** / **Invert scroll**）から編集できます。

| キー | 値（デフォルト） | 動作 |
|-----|------------------|----------|
| `scroll_speed` | `1`–`100`（`50`） | ホイールとトラックパッド両方の速度倍率。`50` = 1.0x、`1` = 0.1x、`100` = 6.0x。 |
| `scroll_mode` | `auto` \| `wheel` \| `trackpad`（`auto`） | ホイールとトラックパッドの判定はヒューリスティックです（ターミナルのスクロールイベントには移動量がありません）。自動検出がデバイスを誤認する場合（ホイール 1 ノッチで移動しすぎる、トラックパッドが段階的に感じられるなど）は、種類を固定してください。 |
| `scroll_lines` | `1`–`10`（未設定） | 1 回のスクロールで移動する行数。ホイールとトラックパッドの**両方**に適用されます。未設定の場合は、各ターミナル固有のプロファイルが適用されます（例: tmux では控えめな 1 行/イベント）。設定ペインに表示される `3` を含め、いずれかの値を確定すると、明示的な上書きへ恒久的に切り替わります。 |
| `invert_scroll` | `false` \| `true`（`false`） | 垂直スクロールの方向を反転（「ナチュラル」スクロール）。 |

```toml
[ui]
scroll_speed = 50
scroll_mode = "auto"     # auto | wheel | trackpad
invert_scroll = false
# scroll_lines はデフォルトで未設定: ターミナルごとのプロファイルが引き続き適用される。
# scroll_lines = 3
```

各設定には環境変数による上書きもあり、最初の読み込み時だけ適用されます。`config.toml` を変更したくないヘッドレス実行やテストに便利です。`GROK_SCROLL_SPEED`、`GROK_SCROLL_MODE`、`GROK_INVERT_SCROLL`（`1`/`true`/`0`/`false`）、`GROK_SCROLL_LINES` を使用できます。優先順位: 環境変数 → `config.toml` → デフォルト。認識できない値はデフォルトにフォールバックし、範囲外の数値は許容範囲内に丸められます。

<a id="tool-configuration"></a>

### ツール設定

```toml
[toolset.bash]
timeout_secs = 120.0                   # フォアグラウンドコマンドのタイムアウト（秒、デフォルト: 120）
output_byte_limit = 20000              # 取得する出力の最大バイト数（デフォルト: 20000）

[toolset.ask_user_question]
timeout_enabled = true                 # false = 回答を無期限に待つ（デフォルト: true）
timeout_secs = 1800                    # 有効時に待機する秒数（デフォルト: 1800 / 30 分）

[toolset.web_fetch]
proxy_endpoint = "https://proxy.example.com"   # 外向きプロキシ URL
allowed_domains = ["docs.rs", "x.ai"]           # 組み込み許可リストを上書き
```

`[toolset.ask_user_question]` は **requirements.toml**、**管理対象設定**、**ユーザーの `config.toml`** のすべてで有効です。優先順位: requirements → 環境変数（`GROK_ASK_USER_QUESTION_TIMEOUT_ENABLED` / `GROK_ASK_USER_QUESTION_TIMEOUT_SECS`）→ ユーザー設定 → 管理対象設定 → デフォルト。自分の自動質問タイムアウトを無効にするには、ユーザー設定で `timeout_enabled = false` を設定します。`timeout_secs` は正の整数である必要があります。`timeout_enabled` は設定ペイン（`/settings` → Agent & Approval の **Ask-Question timeout**）からも切り替えられ、変更は新しく開始するセッションに適用されます。

<a id="authentication"></a>

### 認証

詳細については[認証](02-authentication.md)を参照してください。

```toml
[auth]
auth_provider_command = "/usr/local/bin/my-auth-provider"
auth_provider_label = "Acme Corp"
auth_token_ttl = 3600

[grok_com_config.oidc]
issuer = "https://acme.okta.com"
client_id = "0oa1b2c3d4e5f6g7h8i9"
# scopes = ["openid", "profile", "email", "offline_access", "api:access"]
# audience = "https://api.acme.com"
```

<a id="custom-models"></a>

### カスタムモデル

代替プロバイダーやセルフホスト型モデルを使用するため、カスタムモデルのエンドポイントを追加します。

```toml
[model.my-model]
model = "model-id"                    # API へ送信するモデル識別子
base_url = "https://api.example.com/v1"  # OpenAI 互換エンドポイント
name = "Display Name"                 # モデル選択画面に表示
description = "Model description"     # 任意
api_key = "sk-..."                    # このプロバイダーの API キー
env_key = "XAI_API_KEY"               # API キーを保持する環境変数。文字列または配列（最初に設定された空でない値が優先）
temperature = 0.7                     # サンプリング温度（0.0～2.0）
top_p = 0.95                          # nucleus sampling パラメーター
max_completion_tokens = 8192          # 応答ごとの最大トークン数
context_window = 128000               # コンテキストウィンドウのサイズ（自動圧縮用）
```

認証情報の解決順序: `api_key` > `env_key` > サインイン済みセッショントークン > `XAI_API_KEY`。

組み込みモデルを上書きするには、その名前をセクションキーとして使用します。

```toml
[model.grok-build]
api_key = "my-api-key"               # 必要なフィールドだけを上書き
```

<a id="mcp-servers"></a>

### MCP サーバー

Model Context Protocol を介して外部ツール連携を設定します。

```toml
[mcp_servers.github]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
env = { GITHUB_PERSONAL_ACCESS_TOKEN = "ghp_xxx" }
enabled = true                        # 有効化/無効化（デフォルト: true）
startup_timeout_sec = 30              # 初期化タイムアウト（秒、デフォルト: 30）
tool_timeout_sec = 6000               # ツール呼び出しのタイムアウト（秒、デフォルト: 6000）
tool_timeouts = { create_issue = 120 }  # ツールごとのタイムアウト上書き

[mcp_servers.postgres]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-postgres", "postgresql://user:pass@localhost/db"]

[mcp_servers.my-streamable-server]
url = "https://mcp.example.com/api/mcp"  # HTTP/SSE トランスポート
headers = { "x-mcp-session-id" = "{{session_id}}" }
```

MCP サーバーは、プロジェクト単位の `.grok/config.toml` でも設定できます。プロジェクトスコープの設定では `[mcp_servers]`、`[plugins]`、`[permission]` ルールが追加されます。その他のセクションは `~/.grok/config.toml` からのみ読み込まれます。

`[mcp_servers]` と `[plugins]` の優先順位: `.grok/config.toml`（現在のディレクトリ）> `<repo-root>/.grok/config.toml` > `~/.grok/config.toml`。`[permission]` ルールは優先順位で上書きされず、すべてのファイルから `deny` > `ask` > `allow` の順でマージされます（[22-permissions-and-safety.md](22-permissions-and-safety.md)を参照）。

<a id="memory"></a>

### メモリ

セッションをまたいで知識を保持します（`--experimental-memory` または `GROK_MEMORY=1` が必要）。

```toml
[memory]
enabled = false                       # メモリを有効化

[memory.session]
save_on_end = true                    # セッション終了時にメタデータ要約を書き込む

[memory.watcher]
enabled = true                        # メモリファイルの外部編集を監視

[memory.search]
max_results = 6                       # デフォルトの結果数
min_score = 0.35                      # 最小関連度スコア

[memory.initial_injection]
enabled = true                        # 最初のターンでメモリを自動挿入
min_score = 0.0                       # 最初のターンで挿入するスコアしきい値

[memory.embedding]
model = "embedding-model"             # 埋め込みモデル名
dimensions = 1024                     # ベクトルの次元数
```

<a id="subagents"></a>

### サブエージェント

```toml
[subagents]
enabled = true

[subagents.toggle]
explore = true                        # 特定の種類を有効化/無効化
plan = false

[subagents.models]
explore = "grok-build"               # 別のモデルへ振り分け
```

サブエージェントが使用するモデルを固定するには、`[subagents.models]` にそのエントリを設定します。

<a id="skills"></a>

### スキル

```toml
[skills]
paths = ["~/my-team-skills"]          # 追加でスキャンするディレクトリ
ignore = ["~/my-team-skills/wip"]     # 除外するパス
disabled = ["wip-skill"]              # 一覧には残すが無効にするスキル名
```

<a id="harness-compatibility"></a>

### ハーネス互換性

Cursor、Claude、Codex とのベンダー互換性を制御します。すべてのセルのデフォルト値は `true` です。セッション用セルは、外部セッションスキャナーが使用するまで準備済みの不活性状態に保たれます。

セッション用セルは、外部セッションスキャナーが使用するまで準備済みのままです。各ツールでは、その `sessions` セルと、対応する `resume-claude`、`resume-codex`、`resume-cursor` スキルの両方が必要です。スキルがない場合、外部セッションに対するファイルシステム I/O は一切行われません。

```toml
[compat.cursor]
skills = true     # ~/.cursor/skills/ と <cwd>/.cursor/skills/ をスキャン
rules = true      # <cwd>/.cursor/rules/ をスキャン
agents = true     # ~/.cursor/ で AGENTS.md ファイルをスキャン
mcps = true       # ~/.cursor/mcp.json と <cwd>/.cursor/mcp.json をスキャン
hooks = true      # ~/.cursor/hooks.json と <cwd>/.cursor/hooks.json をスキャン
sessions = true   # 準備済み。スキャナー側の利用機能はまだない

[compat.claude]
skills = true     # ~/.claude/skills/ と <cwd>/.claude/skills/ をスキャン
rules = true      # <cwd>/.claude/rules/ をスキャン
agents = true     # ~/.claude/ で CLAUDE.md / CLAUDE.local.md をスキャン
mcps = true       # ~/.claude.json で MCP サーバーをスキャン
hooks = true      # ~/.claude/settings.json でフックをスキャン
sessions = true   # 準備済み。スキャナー側の利用機能はまだない

[compat.codex]
sessions = true   # 準備済み。スキャナー側の利用機能はまだない
```

Codex の `skills`、`rules`、`agents`、`mcps`、`hooks` セルは予約済みで、現在は不活性です。これらを設定しても `.codex` の検出は有効になりません。

各セルは環境変数または `config.toml` で切り替えられます。環境変数名については、環境変数のリファレンスを参照してください。解決順序: 環境変数 > config.toml > デフォルト（有効）。

`grok inspect` では、セッション開始時の解決が必要なセルは、値を取得できるまで `?` と表示されます。環境変数または TOML で明示されたセルは、その値を使用します。影響を受ける検出項目は、JSON 出力では `compatibilityStatus: "unresolved"`、人が読みやすい出力では `[compat unresolved]` と表示されます。

<a id="plugins"></a>

### プラグイン

```toml
[plugins]
paths = ["~/my-plugins/custom-tools"]
disabled = ["user/a1b2c3d4/noisy-plugin"]
```

<a id="hints"></a>

### ヒント

`[hints]` テーブルには、永続化される小さな UI 設定（主に「今後は確認しない」オプトアウト）が格納されます。TUI で「今後は確認しない」/「config.toml でリセット」オプションを選択すると、Grok が自動的に書き込みますが、手動で編集または削除することもできます。キーを削除するとデフォルトの動作に戻ります。

`[hints]` は**有効な設定のマージ結果**から読み込まれます（他の設定と同じ優先順位）: システムの管理対象設定 → ユーザーの `managed_config.toml` → ユーザーの `config.toml` → ユーザーの `requirements.toml` → システムの `requirements.toml`。高優先度のレイヤーが低優先度のレイヤーを上書きします。TUI がオプトアウトを書き込むのは、ユーザーの `~/.grok/config.toml` だけです。

```toml
[hints]
project_picker_disabled = false        # プロジェクトディレクトリの選択画面をスキップ
memory_modal_fullscreen = false        # メモリモーダルの全画面状態を記憶
new_session_worktree_mode = "never"    # /new の worktree 確認: "ask" | "always" | "never"
fork_worktree_mode = "ask"             # /fork の worktree 確認: "ask" | "always" | "never"
```

| キー | 型 | デフォルト | 説明 |
|-----|------|---------|-------------|
| `project_picker_disabled` | bool | `false` | `true` の場合、プロジェクト外のディレクトリ（ホーム、Desktop、Downloads、`/tmp`）から Grok を起動した際、最初のプロンプトでプロジェクトディレクトリを選択する画面をスキップします。この画面で **「今後は確認しない」**を選ぶと自動的に設定されます。チームは `managed_config.toml` または `requirements.toml` で `[hints] project_picker_disabled = true` を指定し、固定できます。 |
| `memory_modal_fullscreen` | bool | `false` | メモリモーダルを最後に全画面で開いたかを記憶します。 |
| `new_session_worktree_mode` | string | `"never"` | `/new` の worktree 確認。`ask` はポップアップを表示し、`always` は worktree を作成し、`never` はスキップします。 |
| `fork_worktree_mode` | string | `"ask"` | `/fork` の worktree 確認。`ask`、`always`、`never` のいずれか。 |

<a id="notifications"></a>

### 通知

エージェントがターンを完了したとき、または承認が必要なときにターミナル通知を送信します。通知にはターミナルネイティブのプロトコル（OSC 9、OSC 99、OSC 777、BEL）を使用します。デフォルトではフォーカス状態によって制限され、ターミナルを見ていないときだけ送信されます。

```toml
[ui.notifications]
method = "auto"           # auto|osc9|osc99|osc777|bel|none
condition = "unfocused"   # unfocused|always|never
idle_threshold_secs = 3   # 通知するまでにフォーカスが外れている秒数
events = ["turn_complete", "approval_required"]
sleep_prevention = true   # エージェントのターン中にディスプレイのスリープを防止
progress_bar = true       # タブに進捗バーを表示（OSC 9;4）

[ui.notifications.title]
enabled = true
items = ["action-required", "spinner", "activity", "session-name", "grok"]
```

| オプション | 型 | デフォルト | 説明 |
|--------|------|---------|-------------|
| `method` | string | `"auto"` | 通知プロトコル。`auto` はターミナルに最適なものを選択します。 |
| `condition` | string | `"unfocused"` | 通知する条件: `unfocused`（ターミナルからフォーカスが外れている場合のみ）、`always`、`never`。 |
| `idle_threshold_secs` | integer | `3` | 通知するまでにターミナルからフォーカスが外れている必要がある最小秒数。 |
| `events` | array | `["turn_complete", "approval_required"]` | 通知を発生させるイベント。選択肢: `turn_complete`、`approval_required`、`session_ready`、`task_complete`、`agent_error`。 |
| `sleep_prevention` | bool | `true` | エージェントの処理中にディスプレイを起動状態に保ちます（macOS/Linux）。 |
| `progress_bar` | bool | `true` | ターミナルのタブに進捗インジケーターを表示します（OSC 9;4）。 |
| `title.enabled` | bool | `true` | エージェントの状態を反映するようターミナルタイトルを設定します。 |
| `title.items` | array |（上記参照） | タイトルバーに表示する項目。選択肢: `action-required`、`spinner`、`activity`、`session-name`、`cwd`、`model`、`turn-timer`、`grok`。 |

<a id="terminal-support-matrix"></a>

#### ターミナル対応表

| ターミナル | 自動選択プロトコル | フォーカス追跡 | 進捗バー |
|----------|---------------|----------------|--------------|
| iTerm2 | OSC 9 | 対応 | 対応 |
| Kitty | OSC 99 | 対応 | 非対応 |
| Ghostty | OSC 777 | 対応 | 対応 |
| WezTerm | OSC 9 | 対応 | 対応 |
| Warp | OSC 9 | 対応 | 非対応 |
| Alacritty | BEL | 対応 | 非対応 |
| VS Code | BEL | 対応 | 非対応 |
| Apple Terminal | BEL | 非対応 | 非対応 |
| VTE (GNOME Terminal) | OSC 777 | 対応 | 非対応 |
| Grok Desktop | なし（ネイティブ） | 該当なし | 該当なし |
| 不明 | BEL | 非対応 | 非対応 |

`method = "auto"` の場合、Grok はターミナルの種類を検出し、最適なプロトコルを自動選択します。自動検出を上書きするには、`method` を明示的に設定してください。

<a id="notification-hooks"></a>

#### 通知フック

イベント発生時にカスタムコマンドを実行します。フックには環境変数 `$GROK_EVENT`、`$GROK_MESSAGE`、`$GROK_SESSION_ID` が渡されます。

```toml
# macOS ネイティブ通知
[[ui.notifications.hooks]]
command = "terminal-notifier -title 'Grok' -message '$GROK_MESSAGE'"
events = ["turn_complete", "approval_required"]
only_unfocused = true
timeout_secs = 10

# ntfy サーバーへプッシュ
[[ui.notifications.hooks]]
command = "curl -s -d '$GROK_MESSAGE' ntfy.sh/my-grok-alerts"
events = ["turn_complete"]
only_unfocused = true
timeout_secs = 10

# サウンドを再生
[[ui.notifications.hooks]]
command = "afplay /System/Library/Sounds/Glass.aiff"
events = ["turn_complete"]
only_unfocused = true
timeout_secs = 5
```

| フックオプション | 型 | デフォルト | 説明 |
|-------------|------|---------|-------------|
| `command` | string |（必須） | 実行するシェルコマンド。 |
| `events` | array | `[]` | このフックを発生させるイベント（空 = すべてのイベント）。 |
| `only_unfocused` | bool | `true` | ターミナルからフォーカスが外れている場合のみ発生させます。 |
| `timeout_secs` | integer | `10` | この秒数の経過後にフックプロセスを終了します（デフォルト: 10）。 |

<a id="troubleshooting"></a>

#### トラブルシューティング

**tmux で通知が動作しない場合:**
tmux はデフォルトでエスケープシーケンスをブロックします。ターミナルのパススルーを有効にしてください。

```bash
# ~/.tmux.conf 内
set -g allow-passthrough on
```

その後、tmux を再起動します。パススルーを利用できない場合（tmux < 3.3）は、`method` を明示的に `"bel"` へ設定してください。パススルーなしで動作します。

**フォーカス追跡が動作しない場合:**
一部のターミナルはフォーカスイベントを報告しません。`condition = "unfocused"` で通知されない場合は、フォールバックとして `condition = "always"` を試してください。Grok は Apple Terminal と認識できないターミナルを除き、検出対象のすべてのターミナルでフォーカス追跡に対応しています。

**スリープ防止が機能しない場合:**
macOS では、CoreFoundation を介して `IOPMAssertionCreateWithName` を使用します。Linux では `systemd-inhibit` を使用します（`$PATH` 上に必要）。該当するツールが利用可能か確認してください。スリープ防止が有効なのはエージェントのターン中だけで、ターン終了時に自動解除されます。

<a id="keyboard-shortcuts"></a>

### キーボードショートカット

キーボードショートカットは設定ファイルでは**変更できません**。すべてのキーバインドが組み込まれています。完全なリファレンスについては、[キーボードショートカット](03-keyboard-shortcuts.md)を参照してください。

<a id="telemetry"></a>

### テレメトリ

上記の `[features]` ブロックにある `[features] telemetry` トグルは、匿名の使用状況テレメトリ全体を制御するスイッチです。テレメトリが有効な場合、独自のコレクターを運用する企業は、`[telemetry]` で送信先を変更したり、一部を無効にしたりできます。

```toml
[telemetry]
events_url = "https://telemetry.your-company.com/events"  # 独自のコレクターへイベントを送信
events_api_key = "your-collector-token"                   # 必要な場合のコレクター認証
mixpanel_enabled = false                                  # Mixpanel の製品分析を無効化
trace_upload = false                                      # セッション/トレースのアップロードを無効化（未設定時は telemetry トグルを継承）
```

これらは、テレメトリを独自インフラストラクチャへ送る場合、または一部を無効にする場合にだけ設定してください。組み込みのエンドポイントと認証情報は Grok が管理します。デフォルトを使用する場合は未設定のままにしてください。

同じ `[telemetry]` テーブルでは、**外部 OpenTelemetry ストリーム**も設定できます。これは独立したオプトイン機能（上記の telemetry トグルを有効にする必要はありません）で、厳選されたコンテンツを含まない使用状況スキーマを*独自の* OTLP コレクターへ送信します。コレクター認証は `OTEL_EXPORTER_OTLP_HEADERS` で指定し、ディスクには保存されません。外部 OpenTelemetry ガイドがドキュメント配布物に含まれる場合は、完全なスキーマ、環境変数、プライバシーモデルをそのガイドで確認できます。

```toml
[telemetry]
otel_enabled = true                                       # 外部 OTEL のマスタースイッチ（= GROK_EXTERNAL_OTEL）
otel_metrics_exporter = "otlp"                            # otlp | console | none
otel_logs_exporter = "otlp"                               # otlp | console | none
otel_endpoint = "https://collector.corp.example:4318"     # OTLP ベースエンドポイント
otel_protocol = "http/protobuf"                           # http/protobuf | grpc
otel_log_user_prompts = false                             # コンテンツ制御（管理者が requirements で固定可能）
otel_log_tool_details = false                             # コンテンツ制御（管理者が requirements で固定可能）
```

<a id="enterprise-deployment"></a>

### エンタープライズ展開

エンタープライズ向けの完全な設定例:

```toml
[cli]
auto_update = false

[auth]
auth_provider_command = "/usr/local/bin/my-company-auth-provider"
auth_provider_label = "Acme Corp"
auth_token_ttl = 3600

[models]
default = "company-grok"

[model.company-grok]
model = "grok-build"
base_url = "https://grok-proxy.acme.com/"
name = "Grok Build Latest (Proxy)"
context_window = 128000

[features]
telemetry = false
```

---

<a id="pagertoml-appearance-configuration"></a>

## pager.toml（外観設定）

場所: `~/.grok/pager.toml`

TUI の外観と動作を制御します。変更は再起動後に適用されます。

<a id="terminal"></a>

### ターミナル

```toml
[terminal]
alt_screen = "auto"                   # 全画面モード: "auto"、"always"、"never"
```

- `auto`（デフォルト）: ターミナルが対応している場合は代替画面を使用
- `always`: 常に代替画面を使用
- `never`: ターミナルのメインスクロールバックバッファ内で inline 実行

<a id="animation"></a>

### アニメーション

```toml
[animation]
fps = 30                              # アニメーションのフレームレート（1 秒あたりの tick 数）
wave_rows = 32                        # アクセントアニメーションの 1 波周期あたりの行数
```

<a id="prompt"></a>

### プロンプト

```toml
[prompt]
collapse_unfocused = true             # スクロールバックにフォーカスしているときにプロンプトを折りたたむ
mouse_hover = true                    # プロンプトウィジェットでホバー強調を表示
show_prefix = true                    # プロンプトの接頭文字を表示
```

コンパクトモードはここには保存されません。実行時に `[ui] compact_mode` または `/compact-mode` コマンドで制御してください。

<a id="scrollback"></a>

### スクロールバック

```toml
[scrollback.layout]
outer_vpad = 1                        # 垂直パディング
outer_hpad_left = 2                   # 左の水平パディング
outer_hpad_right = 2                  # 右の水平パディング
block_pad_left = 2                    # ブロック内、コンテンツ左側のパディング
block_pad_right = 2                   # ブロック内、コンテンツ右側のパディング

[scrollback.scrollbar]
enabled = true                        # スクロールバーを表示
gap_left = 0                          # コンテンツとスクロールバーの間隔
gap_right = 0                         # スクロールバーと画面端の間隔

[scrollback.scroll]
margin = 0                            # 選択項目の上下に確保する最小コンテキスト行数
min_page_fraction = 0                 # ビューポートに対する最小スクロール割合（0～100）
follow_indicator = "center"           # 追従インジケーター: "center" または "none"
follow_auto_select = true             # 追従モードで最新項目を自動選択
follow_by_overscroll = true           # 最下部を越えてスクロールすると追従モードを開始
anchor_on_fold = true                 # 折りたたみ時にブロック位置を維持
respect_manual_folds = true           # オプトイン（デフォルト: false）: ストリーミング中/完了時も手動で折りたたんだブロックを維持。追従中に展開すると自動スクロールを停止

[scrollback.display]
sticky_headers = true                 # ユーザープロンプトを固定ヘッダーとして表示
tab_width = 4                         # タブ文字あたりのスペース数
expandable_indicator = true           # 折りたたみ可能な項目に展開インジケーターを表示
expandable_indicator_running = true   # 実行中の項目にインジケーターを表示
expandable_indicator_char = "›"       # 展開インジケーターの文字（デフォルト: "›"）
selection_buttons = false             # 選択時にコピー/表示ボタンを表示
line_under_last_entry = false         # 最後の項目の下に水平線を表示
group_selection_split = true          # 展開されたブロックの選択枠を分割
highlight_overlays_border = false     # 強調表示を選択枠の境界まで広げる
dim_accent = 0.5                      # 折りたたまれたアクセントの減光係数（0.0～1.0）
```

`respect_manual_folds` はデフォルトで無効です。オプトインするには `true` に設定します。有効にすると、手動で折りたたんだブロックが固定されます。ストリーミング更新や完了イベント（思考ブロックの終了など）でも折りたたみ状態は変わりません。また、追従モードで新しい内容を末尾追跡しているときにブロックを展開すると、自動スクロールが停止して表示位置が維持されます。追従は、`Shift+G`、最後の項目での `j`、最下部を越えるスクロール、新しいプロンプトの送信により再開します。`Shift+E` はすべての固定を解除し、`Ctrl+E` は思考ブロックの固定を解除します。

<a id="block-configuration"></a>

### ブロック設定

```toml
[scrollback.blocks.edit]
indent = true                         # diff コンテンツをインデント
vpad = false                          # 垂直パディング
# expanded_by_default = true          # 未設定: config.toml の [ui] collapsed_edit_blocks に従う
                                      # （フラグが有効 = 折りたたまれた 1 行表示）。どちらかの表示に固定するにはコメントを解除
dual_line_numbers = false             # 2 列の行番号（変更前 + 変更後）
# line_summary = false                # 折りたたまれたヘッダーに +N/-M を表示。未設定の場合は同じフラグに従う
hunk_separator = "…"                  # diff hunk 間の区切り文字（デフォルト: "…"）

[scrollback.blocks.prompt]
vpad = true                           # 垂直パディング
show_prefix = true                    # プロンプトの接頭文字を表示
min_lines = 2                         # 固定表示モードでの最小コンテンツ行数

[scrollback.blocks.thinking]
animate = true                        # 思考中のアクセントをアニメーション表示
truncated_lines = 3                   # 省略表示モードでの行数
```

<a id="todo"></a>

### Todo

```toml
[todo]
badge_format = "default"              # "default"、"colon"、"comma" のいずれか
```

バッジ形式の例:
- `default`: `2/5` -- `完了数/合計数` の進捗比率（完了数 = completed、合計数 = cancelled を除く全タスク）
- `colon`: `[>:1 [ ]:4 ok:3 x:2]` -- アイコン:件数
- `comma`: `[1 >, 4 [ ], 3 ok, 2 x]` -- 件数 アイコン、カンマ区切り

<a id="plugins-1"></a>

### プラグイン

```toml
disable_plugins = false               # フック/プラグイン UI 全体を非表示
```

---

<a id="environment-variables"></a>

## 環境変数

主要な環境変数を以下に示します。完全な一覧については README を参照してください。

<a id="authentication-1"></a>

### 認証

| 変数 | 説明 |
|----------|-------------|
| `XAI_API_KEY` | console.x.ai で取得した API キー |
| `GROK_AUTH_PROVIDER_COMMAND` | 外部認証バイナリのパス |
| `GROK_AUTH_PROVIDER_LABEL` | TUI ログイン画面での表示名 |
| `GROK_AUTH_TOKEN_TTL` | トークンの有効期間（秒） |
| `GROK_AUTH_EARLY_INVALIDATION_SECS` | 有効期限の何秒前に更新するか（デフォルト: 300） |
| `GROK_OIDC_ISSUER` | OIDC issuer URL |
| `GROK_OIDC_CLIENT_ID` | OIDC client ID |

<a id="endpoints"></a>

### エンドポイント

| 変数 | 説明 |
|----------|-------------|
| `GROK_CLI_CHAT_PROXY_BASE_URL` | API プロキシのベース URL を上書き |

<a id="features"></a>

### 機能

| 変数 | 説明 |
|----------|-------------|
| `GROK_LANG` | 表示言語（`en-US` または `ja-JP`）。そのプロセスでは `[ui].language` より優先 |
| `GROK_MEMORY` | セッション間メモリを有効（`1`）または無効（`0`）にする |
| `GROK_SUBAGENTS` | サブエージェントを有効（`1`）または無効（`0`）にする |
| `GROK_WEB_FETCH` | web_fetch ツールを有効（`1`）または無効（`0`）にする |
| `GROK_AGENT` | カスタムエージェント定義のパスまたは名前 |
| `GROK_SANDBOX` | サンドボックスプロファイル（off、workspace、devbox、read-only、strict、またはカスタムプロファイル名） |

<a id="logging"></a>

### ログ

| 変数 | 説明 |
|----------|-------------|
| `GROK_LOG_FILE` | このファイルパスへログを書き込む（値はそのままパスとして使用） |
| `RUST_LOG` | ログレベルフィルター（例: `debug`）。`GROK_LOG_FILE` のログとヘッドレス実行時の stderr 出力を制御 |

<a id="paths"></a>

### パス

| 変数 | 説明 |
|----------|-------------|
| `GROK_HOME` | 設定ディレクトリを上書き（デフォルト: `~/.grok`） |
| `GROK_RESPECT_GITIGNORE` | gitignore フィルタリングを強制的に有効（`1`）または無効（`0`）にする。`[tools] respect_gitignore` を上書き |

<a id="telemetry-1"></a>

### テレメトリ

| 変数 | 説明 |
|----------|-------------|
| `GROK_TELEMETRY_ENABLED` | テレメトリを有効化/無効化 |
| `GROK_FEEDBACK_ENABLED` | フィードバックシステムを有効化/無効化 |
| `GROK_DEPLOYMENT_KEY` | エンタープライズ向け管理 API キー |

---

<a id="file-locations"></a>

## ファイルの場所

| パス | 説明 |
|------|-------------|
| `~/.grok/config.toml` | メイン設定ファイル |
| `~/.grok/pager.toml` | TUI の外観設定 |
| `~/.grok/auth.json` | 認証情報（自動管理） |
| `~/.grok/sessions/` | 永続化されたセッション（作業ディレクトリ別に整理） |
| `~/.grok/memory/` | セッション間メモリのファイルとインデックス |
| `~/.grok/skills/` | ユーザースコープのスキル定義 |
| `~/.grok/plugins/` | ユーザースコープのプラグイン |
| `~/.grok/agents/` | ユーザースコープのエージェント定義 |
| `~/.grok/lsp.json` | LSP サーバー設定（ユーザースコープ） |
| `~/.grok/logs/` | 内部ログファイル（例: `unified.jsonl`、MCP サーバーログ） |
| `.grok/config.toml` | プロジェクトスコープの MCP サーバー、プラグイン、権限ルール |
| `.grok/skills/` | プロジェクトスコープのスキル定義 |
| `.grok/plugins/` | プロジェクトスコープのプラグイン |
| `.grok/agents/` | プロジェクトスコープのエージェント定義 |
| `.grok/hooks/` | プロジェクトスコープのフック |
| `.grok/lsp.json` | LSP サーバー設定 |

---

<a id="project-scoped-configuration"></a>

## プロジェクトスコープの設定

リポジトリ内の `.grok/` にファイルを配置すると、一部の設定をプロジェクト単位で指定できます。

| ファイル | 設定対象 |
|------|--------------------|
| `.grok/config.toml` | MCP サーバー、プラグイン、権限ルール、`[mcp] max_output_bytes` のツール結果上限（その他のセクションは `~/.grok/config.toml` からのみ読み込まれる） |
| `.grok/skills/` | プロジェクト固有のスキル |
| `.grok/hooks/` | プロジェクト固有のライフサイクルフック |
| `.grok/agents/` | プロジェクト固有のエージェント定義 |
| `.grok/lsp.json` | LSP サーバー設定 |
| `.grok/sandbox.toml` | カスタムサンドボックスプロファイル |
| `AGENTS.md` | プロジェクト指示（システムプロンプト） |

プロジェクトスコープの MCP サーバーは、同名のグローバル設定を上書きします（マージではなく完全置換）。

---

<a id="lsp-servers"></a>

## LSP サーバー

言語サーバーは、パッシブ診断と任意の `lsp` ツールを提供します（[`lsp_tools`](#general-settings) 機能フラグを参照）。サーバー定義は 3 つの取得元から集められ、サーバー名でマージされます。

| 取得元 | 場所 | スコープ |
|--------|----------|-------|
| ユーザー | `~/.grok/lsp.json` | すべてのプロジェクト |
| プロジェクト | `.grok/lsp.json` | 現在のリポジトリ |
| プラグイン | 信頼済みプラグインの `.lsp.json` ファイル、または `plugin.json` 内のインライン `lspServers` ブロック | プラグインが有効な場所 |

同じサーバー名が複数の取得元で定義されている場合、次の順序で解決されます（上ほど優先）。

1. **プロジェクト** -- `.grok/lsp.json`
2. **ユーザー** -- `~/.grok/lsp.json`
3. **プラグイン** -- ファイルベースの `.lsp.json`、次にインライン `lspServers`（プラグインの読み込み順）

プロジェクトとユーザーのエントリは、同名の低優先度エントリを置き換えます。プラグインのエントリが追加するのは、ローカルファイルでまだ定義されていない名前のサーバーだけです。そのため、ローカルの `lsp.json` は常にプラグインより優先されます。プラグインの LSP サーバーは、プラグインが信頼された後にだけ読み込まれます（[プラグイン](09-plugins.md)を参照）。
