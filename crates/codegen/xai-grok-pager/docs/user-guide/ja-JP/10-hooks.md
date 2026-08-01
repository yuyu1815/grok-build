<a id="hooks"></a>

# フック

フックを使うと、Grok セッションの重要なタイミングでスクリプトを実行したり、HTTP リクエストを送信したりできます。タスクの自動化、安全性チェックの実施、アクティビティの記録、通知の送信、独自ツールとの連携に利用できます。

---

<a id="what-are-hooks"></a>

## フックとは

フックとは、特定のライフサイクルイベントが発生したときに Grok が呼び出すシェルコマンドまたは HTTP エンドポイントです。フックでは次のことができます。

- **アクションをブロックする** -- `PreToolUse` フックは、危険なコマンドが実行される前に拒否できます。
- **イベントに反応する** -- `PostToolUse` フックは、すべてのツール実行をファイルに記録できます。
- **コンテキストを設定する** -- `SessionStart` フックは、環境変数をエクスポートしたり、セットアップスクリプトを実行したりできます。

---

<a id="common-use-cases"></a>

## 一般的な用途

- **安全対策**: `rm -rf /` などのコマンドを実行前にブロックする。
- **監査ログ**: ツールの使用状況やセッションをファイルまたは外部サービスに記録する。
- **通知**: タスクの完了時にメッセージを送信する。
- **自動フォーマット**: 編集後に `cargo fmt` や `prettier` を実行する。
- **環境のセットアップ**: セッション開始時に変数をエクスポートする。
- **カスタムワークフロー**: 特定のイベントでビルド、テスト、デプロイを開始する。

---

<a id="quick-start"></a>

## クイックスタート

1. フック用ディレクトリを作成します。

   ```sh
   mkdir -p ~/.grok/hooks
   ```

2. フックファイル（例: `~/.grok/hooks/session-start.json`）を作成します。

   ```json
   {
     "hooks": {
       "SessionStart": [
         {
           "hooks": [
             { "type": "command", "command": "echo 'Grok session started in '$(pwd)" }
           ]
         }
       ]
     }
   }
   ```

3. Grok セッションを開始（または再起動）します。`SessionStart` でフックが自動的に実行されます。

4. VS Code 系以外のターミナルでは `Ctrl+L` を押し（または任意の環境で `/hooks` を実行。VS Code 系ではこちらを推奨）、Hooks タブで読み込まれたことを確認します。

---

<a id="hook-locations"></a>

## フックの場所

フックは複数の場所から検出され、すべて統合されます。

| スコープ | パス | 信頼済み？ | 備考 |
|----------|------|------------|------|
| グローバル | `~/.grok/hooks/*.json` | 常に信頼済み | 個人用フック |
| グローバル | `~/.claude/settings.json`（および `settings.local.json`） | 常に信頼済み | Claude Code 互換（設定可能） |
| グローバル | `~/.cursor/hooks.json` | 常に信頼済み | Cursor 互換（設定可能） |
| プロジェクト | `<project>/.grok/hooks/*.json` | 信頼が必要 | リポジトリ単位の自動化 |
| プロジェクト | `<project>/.claude/settings.json`（および `settings.local.json`） | 信頼が必要 | Claude 互換（設定可能） |
| プロジェクト | `<project>/.cursor/hooks.json` | 信頼が必要 | Cursor 互換（設定可能） |
| プラグイン | インストール済みプラグインに同梱 | プラグイン単位 | チーム共有フック |

Claude と Cursor のフックソースはデフォルトでスキャンされます。特定ベンダーのスキャンを無効にするには、`~/.grok/config.toml` で `[compat.<vendor>] hooks = false` を設定するか、対応する環境変数を設定します。詳細は[設定](05-configuration.md#harness-compatibility)を参照してください。

**プロジェクトを信頼する**: フックを含むプロジェクトを初めて開いたときは、プロジェクトのフックを実行する前に、そのプロジェクトを信頼する必要があります。それまではフックが通知なくスキップされます。`/hooks-trust` を実行（または `--trust` を付けて起動）して信頼を付与します。この判断は、リポジトリ内の MCP/LSP サーバーにも適用される統合フォルダー信頼ストア（`~/.grok/trusted_folders.toml`）に記録されます。`~/.grok/hooks/` のグローバルフックは常に信頼され、登録は不要です。これにより、信頼されていないリポジトリによる任意コードの実行を防ぎます。

フックはフォルダー信頼に統合されているため、`--trust` / `/hooks-trust` で信頼を付与すると、フォルダー全体の **MCP、LSP、フック**がまとめて信頼され、サブディレクトリにも適用されます。逆に、フォルダー信頼を無効にすると（`GROK_FOLDER_TRUST=0` または `[folder_trust] enabled = false`）、MCP/LSP と同様にプロジェクトフックも信頼確認なしで実行可能になります。

---

<a id="hook-events"></a>

## フックイベント

| イベント | 発生タイミング | ブロック可能？ |
|----------|----------------|----------------|
| `SessionStart` | セッションの開始時。 | いいえ |
| `UserPromptSubmit` | プロンプトの送信時。 | いいえ |
| `PreToolUse` | ツールの実行直前。 | はい — 拒否可能 |
| `PostToolUse` | ツールが正常に完了したとき。 | いいえ |
| `PostToolUseFailure` | ツールが失敗したとき。 | いいえ |
| `PermissionDenied` | 権限システムがツール呼び出しを拒否したとき。 | いいえ |
| `Stop` | エージェントのターンが終了したとき（完了、キャンセル、エラー）。 | いいえ |
| `StopFailure` | API エラーによりターンが終了したとき。 | いいえ |
| `Notification` | エージェントが通知を送信したとき。 | いいえ |
| `SubagentStart` | サブエージェントが開始したとき。 | いいえ |
| `SubagentStop` | サブエージェントが終了したとき。 | いいえ |
| `PreCompact` | 会話の圧縮を開始する直前。 | いいえ |
| `PostCompact` | 会話の圧縮が完了したとき。 | いいえ |
| `SessionEnd` | セッションの終了時。 | いいえ |

`SubagentEnd` は `SubagentStop` の別名として使用できます。ツール呼び出しをブロックできるのは `PreToolUse` だけで、ほかのイベントはすべて受動的です。

<a id="cursor-hook-compatibility"></a>

### Cursor フックとの互換性

Grok は Cursor の camelCase 形式のフックイベント名に対応しているため、`~/.cursor/hooks.json` を変更せずに読み込めます。

| Cursor イベント | 対応先 |
|-----------------|--------|
| `sessionStart`, `sessionEnd` | `SessionStart`, `SessionEnd` |
| `preToolUse`, `postToolUse`, `postToolUseFailure` | `PreToolUse`, `PostToolUse`, `PostToolUseFailure` |
| `beforeShellExecution`, `beforeMCPExecution`, `beforeReadFile` | `PreToolUse` |
| `afterShellExecution`, `afterMCPExecution`, `afterFileEdit` | `PostToolUse` |
| `afterAgentResponse`, `afterAgentThought` | `PostToolUse` |
| `beforeSubmitPrompt` | `UserPromptSubmit` |
| `subagentStart`, `subagentStop` | `SubagentStart`, `SubagentStop` |
| `preCompact`, `stop` | `PreCompact`, `Stop` |

Cursor の操作別フック（`beforeShellExecution`、`afterFileEdit` など）は、汎用の `PreToolUse` / `PostToolUse` イベントに対応します。フックスクリプトは JSON 入力でツール名を受け取り、それに応じて絞り込めます。または `matcher` フィールドを使用できます。

---

<a id="the-hook-json-format"></a>

## フック JSON の形式

各 `.json` ファイルでは、複数のイベントに対するフックを定義できます。

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          { "type": "command", "command": "bin/safety-check.sh", "timeout": 10 }
        ]
      }
    ],
    "PostToolUse": [
      {
        "hooks": [
          { "type": "command", "command": "bin/log-activity.sh" }
        ]
      }
    ]
  }
}
```

<a id="key-fields"></a>

### 主なフィールド

- **イベント名**（最上位キー）: [フックイベント](#hook-events)に記載された任意のイベント。Grok は認識できないイベント名をスキップするため、共有の Claude または Cursor 設定ファイルも読み込めます。
- **matcher**（省略可）: どの呼び出しでフックを実行するかを選択する正規表現。ツールイベント（`PreToolUse`、`PostToolUse`、`PostToolUseFailure`、`PermissionDenied`）ではツール名を、`Notification` では通知タイプを検査します。ライフサイクルイベント（`SessionStart`、`SessionEnd`、`Stop`、`UserPromptSubmit`）では matcher が拒否され、それ以外のイベントでは無視されます。matcher が空または省略されている場合は、すべてに一致します。matcher は実際のツール名を検査します。内部の `use_tool` ディスパッチャーを経由する MCP 呼び出しは、修飾された `server__tool` 名（例: `linear__save_issue`）として現れるため、ディスパッチャー名ではなく、その名前に一致させてください。
- **type**: `"command"`（スクリプトまたはシェルのワンライナーを実行）または `"http"`（イベントを URL に POST）。
- **command**: 実行可能ファイルのパス（JSON ファイルからの相対パス）またはインラインのシェルコマンド。
- **timeout**: フックを強制終了するまでの秒数（デフォルト: 5）。フックの失敗（タイムアウト、クラッシュ、不正な出力、必須環境変数の欠落）はすべてフェイルオープンです。失敗は UI のスクロールバック用に記録されますが、ツール呼び出しはブロックされません。ツール呼び出しをブロックするのは、フックが明示的に返した `deny` 判断だけです。

<a id="tool-name-aliases"></a>

### ツール名の別名

`matcher` では、Claude 形式のツール名が Grok のツール名に対応付けられるため、Claude から移行したフックも正しく実行されます。主な別名は次のとおりです。

- `Bash` → `run_terminal_command`
- `Read` → `read_file`
- `Edit`、`Write`、`MultiEdit` → `search_replace`
- `Grep` → `grep`
- `Glob`、`ListDir` → `list_dir`
- `WebSearch` → `web_search`
- `Task` → `spawn_subagent`

matcher では元の名前も維持されるため、`Bash` は `Bash` と `run_terminal_command` の両方に一致します。

---

<a id="writing-hook-scripts"></a>

## フックスクリプトの作成

<a id="input"></a>

### 入力

イベントは **stdin** に JSON として送信されます（以下は `PreToolUse` イベントの例です。ペイロードには常に `toolUseId` と `toolInputTruncated` も含まれます）。

```json
{
  "hookEventName": "pre_tool_use",
  "sessionId": "abc-123",
  "cwd": "/Users/you/project",
  "workspaceRoot": "/Users/you/project",
  "toolName": "run_terminal_command",
  "toolInput": { "command": "npm test" },
  "timestamp": "2026-04-14T12:00:00Z"
}
```

<a id="output-blocking-hooks"></a>

### 出力（ブロッキングフック）

`PreToolUse` フックでは、**stdout** に JSON を書き込みます。

- **許可**: `{"decision": "allow"}`
- **拒否**: `{"decision": "deny", "reason": "Unsafe command detected"}`

<a id="exit-codes"></a>

### 終了コード

| 終了コード | 意味 |
|------------|------|
| `0` | 成功 / 許可（ブロッキングフックの場合） |
| `2` | 明示的な拒否（ブロッキングフックのみ） |
| その他 | フェイルオープン — 失敗は記録されますが、ツール呼び出しはブロックされません。呼び出しをブロックするには、stdout の JSON で `deny` 判断を出力します（終了コードにかかわらず適用されます）。 |

<a id="passive-hooks"></a>

### 受動フック

`SessionStart` や `PostToolUse` などのイベントでは、stdout は無視されます。成功時は終了コード 0 で終了するだけです。

<a id="environment-variables"></a>

### 環境変数

Grok は、すべてのフックプロセスに複数の環境変数を設定します。コンテキストやプラグインを考慮するフックスクリプトの作成に役立ちます。

<a id="runner-injected-variables-always-available"></a>

#### ランナーが注入する変数（常に利用可能）

以下の変数は、フックランナーが**すべての**フックに設定します。

| 変数 | 説明 |
|------|------|
| `GROK_HOOK_EVENT` | フックを発生させたイベントの名前（例: `pre_tool_use`、`session_start`、`post_tool_use`、`session_end`、`stop`、`notification`）。 |
| `GROK_HOOK_NAME` | このフックに設定された名前（プラグインが提供するフックの場合はプラグインのプレフィックスを含む）。 |
| `GROK_SESSION_ID` | 現在の Grok セッションの一意な識別子。 |
| `GROK_WORKSPACE_ROOT` | 現在のワークスペースのルートへの絶対パス。 |
| `CLAUDE_PROJECT_DIR` | ワークスペースのルートへの絶対パス。すべてのフックに設定される、`GROK_WORKSPACE_ROOT` の Claude Code 互換の別名。 |

これらの変数は**予約済み**です。フック JSON の `env` フィールドでこれらに値を設定しても、読み込み時に削除され（警告が記録されます）、ランナーがプロセス生成時に常に実際の値を注入します。

<a id="plugin-hook-variables"></a>

#### プラグインフックの変数

プラグイン由来のフックには、Grok が次の変数も注入します。

| 変数 | 説明 |
|------|------|
| `GROK_PLUGIN_ROOT` | インストールされたプラグインディレクトリへの絶対パス。 |
| `GROK_PLUGIN_DATA` | プラグインの書き込み可能なデータディレクトリ（プラグインの状態やキャッシュなどの保存先）への絶対パス。 |

これらの値はプラグインシステムから提供されます。プラグイン関連の 4 つのキー（`GROK_PLUGIN_ROOT`、`GROK_PLUGIN_DATA`、およびそれぞれの Claude 互換の別名）については、フックの `env` マップでユーザーが宣言した値よりも、プラグインアダプターが提供する公式の値が常に優先されます。

<a id="user-defined-environment-variables"></a>

#### ユーザー定義の環境変数

`env` フィールドを使用すると、個別のフックハンドラーに追加の環境変数を指定できます。

```json
{
  "type": "command",
  "command": "bin/my-hook.sh",
  "env": {
    "MY_SECRET": "value",
    "LOG_LEVEL": "debug"
  }
}
```

これらの変数はフックプロセスに渡されますが、上記の予約済みランナー変数やプラグイン変数を上書きすることはできません。

<a id="using-variables-in-command-and-url-fields"></a>

#### `command` フィールドと `url` フィールドで変数を使用する

`command` と `url` は、どちらも `${VAR}` と `$VAR` の展開に対応しています。読み込み時と実行時の展開、`env` マップの参照順序、パラメーター展開修飾子（例: `${VAR:-default}`）の処理方法については、custom-hooks リファレンスを参照してください。

---

<a id="http-hooks"></a>

## HTTP フック

ローカルスクリプトの代わりに、リモートエンドポイントを呼び出せます。

```json
{ "type": "http", "url": "https://hooks.example.com/grok-event", "timeout": 15 }
```

イベントの完全なエンベロープが JSON として POST されます。

---

<a id="managing-hooks-in-the-tui"></a>

## TUI でフックを管理する

<a id="the-hooks-tab"></a>

### Hooks タブ

VS Code 系以外のターミナルでは `Ctrl+L` を押して Extensions モーダル（Plugins タブ）を開くか、`/hooks` を実行して（任意のターミナルで使用可能。`Ctrl+L` が割り込みになる VS Code 系では必須）、Hooks タブを開きます。**Hooks** タブでは次のキーを使用します。

| キー | 操作 |
|------|------|
| `r` | ディスクからすべてのフックを再読み込みする |
| `a` | パスを指定してカスタムフックを追加する |
| `x` | 選択したフックを削除する |
| `Space` | 選択したフックを有効または無効にする |
| `f` | 状態フィルター（All / Enabled / Disabled）を切り替える |

フックは、取得元ごとに **Global**、**Project**、**Plugin**、**Custom** に分類されます。

各フックには次の情報が表示されます。
- 発生条件となる**イベント**
- 実行される**コマンド**または **URL**
- **タイムアウト**時間
- **状態** -- 有効または `[disabled]`

<a id="slash-commands"></a>

### スラッシュコマンド

```
/hooks-list           # このセッションに読み込まれたフックを表示する
/hooks-trust          # このプロジェクトでのフック実行を信頼する
/hooks-add <path>     # カスタムフックのファイルまたはディレクトリを追加する
/hooks-remove <path>  # カスタムフックを削除する
/hooks-untrust        # このプロジェクトへの信頼を取り消す
```

TUI ページャーでは、個別の `/hooks-*` コマンドはスラッシュコマンド一覧に表示されません。`/hooks` モーダルでフックの一覧表示、追加、削除、有効化、無効化を行えます。プロジェクトの信頼は `/hooks-trust`（またはモーダルの Trust 操作）で管理され、前述の統合フォルダー信頼ストアに書き込まれます。

<a id="per-hook-enable-disable"></a>

### フック単位の有効化と無効化

Hooks タブで `Space` を押すと、個別のフックを実行中に有効または無効にできます。変更はセッションを再起動せずに、すぐに反映されます。

<a id="mid-session-reload"></a>

### セッション中の再読み込み

Hooks タブで `r` を押すと、ディスクからすべてのフックを再読み込みします。Grok がすべてのフックソースを再度読み込むため、セッション中にフックファイルへ加えた変更も反映されます。

---

<a id="hook-annotations-in-scrollback"></a>

## スクロールバック内のフック注釈

フックが実行されると、その結果が TUI のスクロールバックに注釈として表示されます。どのフックが実行されたか、アクションを許可または拒否したか、どのような出力が生成されたかを確認できます。これらの注釈は、プラグイン UI が有効な場合（デフォルト）のみ表示されます。

---

<a id="example-safe-shell-guard"></a>

## 例: 安全なシェルガード

危険なシェルコマンドをブロックします。

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          { "type": "command", "command": "bin/safe-shell.sh", "timeout": 5 }
        ]
      }
    ]
  }
}
```

`bin/safe-shell.sh` の内容は次のとおりです。

```bash
#!/bin/sh
INPUT=$(cat)
CMD=$(echo "$INPUT" | jq -r '.toolInput.command // empty')

# 破壊的なパターンをブロックする
if echo "$CMD" | grep -qE '(rm -rf /|mkfs|dd if=|:(){ :|& };:)'; then
  echo '{"decision": "deny", "reason": "Blocked potentially destructive command"}'
  exit 2
fi

echo '{"decision": "allow"}'
```

---

<a id="security-notes"></a>

## セキュリティ上の注意

- グローバルフック（`~/.grok/hooks/`）はユーザー権限で実行されます。シェルスクリプトと同様に扱ってください。
- 悪意のあるリポジトリによるサプライチェーン攻撃を防ぐため、プロジェクトフックにはフォルダーの信頼（`/hooks-trust` または `--trust`。リポジトリ内の MCP/LSP と同じ仕組み）が必要です。
- HTTP フックはセッションデータを送信します。信頼できるエンドポイントのみを使用してください。

---

<a id="best-practices"></a>

## ベストプラクティス

1. **フックを高速に保つ** -- 長時間実行されるフックは UI をブロックします。可能な限りバックグラウンドプロセス（`&`）または非同期処理を使用してください。
2. **ブロックには明示的な `deny` を使用する** -- フックはエラー時にフェイルオープンとなるため、クラッシュしたフックはツールをブロックしません。ポリシーを適用するには、フックが最後まで実行され、stdout に `{"decision":"deny","reason":"..."}` を出力する必要があります。明示的な判断を返せるよう、スクリプト内で必ずエラーを処理してください。
3. **絶対パスまたはフックファイルからの相対パスを使用する** -- JSON ファイルの隣にある `bin/` 内のスクリプトは移植可能です。
4. **モーダルでテストする** -- フックに依存する前に、`Ctrl+L` を押す（VS Code 系以外）か `/hooks` を実行して、フックが読み込まれ、一致していることを確認してください。
5. **プロジェクトフックをバージョン管理する** -- `.grok/hooks/` をコミットしてください（ただしシークレットは絶対に含めないでください）。

---

<a id="troubleshooting"></a>

## トラブルシューティング

- **フックが実行されない場合**: VS Code 系以外では `Ctrl+L` を押す（または任意の環境で `/hooks` を実行する）ことで、読み込まれ、一致しているかを確認できます。
- **プロジェクトフックが無視される場合**: フォルダーが信頼されていない可能性があります。`/hooks-trust` を実行してください（または `--trust` を付けて再起動してください）。
- **スクリプトが見つからない場合**: パスが `.json` ファイルからの相対パスであり、実行可能になっているか（`chmod +x`）を確認してください。
- **エラーを確認する場合**: `RUST_LOG=debug GROK_LOG_FILE=/tmp/grok.log grok` で起動してログを取得し、`/tmp/grok.log` を確認してください。
