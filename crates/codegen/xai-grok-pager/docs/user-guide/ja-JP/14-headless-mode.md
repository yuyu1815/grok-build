<a id="headless-mode-and-scripting"></a>

# ヘッドレスモードとスクリプト

ヘッドレスモードでは、Grok をコマンドラインから非対話的に実行します。1 つのプロンプトを受け取り、すべてのツールにアクセスして実行し、結果を返します。タスクの自動化、ワークフローのスクリプト化、連携機能の構築、出力のプログラムによる解析に使用できます。

---

<a id="basic-usage"></a>

## 基本的な使い方

プロンプトを非対話的に渡すと、ヘッドレスモードが起動します。最も一般的な方法は `-p` フラグ（`--single` の短縮形）です。`--prompt-json` と `--prompt-file` でも起動します。

```bash
grok -p "Your prompt here"
```

Grok はプロンプトを処理し、必要なツールを実行して、結果を stdout に出力します。応答が完了するとプロセスは終了します。

---

<a id="command-line-options"></a>

## コマンドラインオプション

| フラグ                    | 説明                                           |
| ------------------------- | ---------------------------------------------- |
| `-p, --single <PROMPT>` | 送信するプロンプト（または `--prompt-json` / `--prompt-file` を使用） |
| `-m, --model <MODEL>`   | 使用するモデル（例: `grok-build`）              |
| `-s, --session-id <ID>` | この **UUID** で**新しい**セッションを作成（無効な UUID、または対象セッションディレクトリですでに使用中の場合はエラー。再開はしないため `-r`/`-c` を使用） |
| `--fork-session`        | `-r`/`-c` と併用し、元のセッションに追記せず新しいセッション ID にフォーク |
| `-r, --resume <ID>`     | 既存のセッションを再開（見つからない場合はエラー）      |
| `-c, --continue`        | 現在のディレクトリで最も新しいセッションを継続  |
| `--cwd <PATH>`          | 作業ディレクトリを設定                                 |
| `--output-format <FMT>` | 出力形式: `plain`、`json`、`streaming-json`      |
| `--yolo`                | すべてのツール実行を自動承認                      |
| `--rules <TEXT>`        | システムプロンプト用のカスタムルール                    |
| `--tools <TOOLS>`       | 組み込みツールの許可リスト（カンマ区切り）。拒否されない限り MCP メタツールは引き続き利用可能。ヘッドレス専用。 |
| `--disallowed-tools <TOOLS>` | 削除する組み込みツールの拒否リスト（カンマ区切り）。`Agent` エントリに対応。ヘッドレス専用。 |
| `--max-turns <N>`       | 停止するまでのエージェントターンの最大数。ヘッドレス専用。 |
| `--reasoning-effort` / `--effort <LEVEL>` | 推論モデルの推論の強度。正規のレベル: `none`、`minimal`、`low`、`medium`、`high`、`xhigh`、`max`（`xhigh` のエイリアス）。モデルごとのメニューオプション ID（例: `deep` → wire 上の値にマッピング）も `/effort` と同様に使用可能。TUI とヘッドレスの両方で動作。 |
| `--permission-mode <MODE>` | 権限モード。このフラグでは `bypassPermissions` で常時承認を有効化（[22-permissions-and-safety.md](22-permissions-and-safety.md)を参照）。デフォルト拒否には `.claude/settings.json` の `defaultMode` を使用。 |
| `--allow <RULE>`        | glob パターンを使用する権限許可ルール（複数指定可）。TUI とヘッドレスの両方で動作。 |
| `--deny <RULE>`         | glob パターンを使用する権限拒否ルール（複数指定可）。TUI とヘッドレスの両方で動作。 |
| `--prompt-json <JSON>`  | JSON コンテンツブロック形式のプロンプト                         |
| `--prompt-file <PATH>`  | ファイルからプロンプトを読み込み                                    |
| `--verbatim`            | プロンプトを指定どおりに送信                          |
| `--no-auto-update`      | このセッションの更新確認を無効化                |
| `--sandbox <PROFILE>`   | ファイルシステム／ネットワークアクセス用のサンドボックスプロファイル         |

> **注:** `--tools`、`--disallowed-tools`、`--max-turns`、`--agents` はヘッドレス専用フラグです。対話型 TUI で使用すると警告が出力され、フラグは無視されます。`--reasoning-effort`/`--effort`、`--permission-mode`、`--allow`、`--deny` は両方のモードで動作します。その他のフラグ（エージェント、検証、worktree）については、[その他のヘッドレスフラグ](#additional-headless-flags)を参照してください。

<a id="tool-filtering"></a>

### ツールの絞り込み

`--tools` でエージェントを明示的なツールセット（許可リスト）に制限するか、`--disallowed-tools` でデフォルトセットから特定のツールを削除（拒否リスト）します。どちらもカンマ区切りのツール名を受け取ります。

ツール名には内部ツール ID を使用します（例: シェルツールは `bash` ではなく `run_terminal_cmd`）。

```bash
# 読み取り専用ツールだけを許可
grok -p "Explain this codebase" --tools "read_file,grep,list_dir"

# Web アクセスとファイル編集を削除
grok -p "Review this code" --disallowed-tools "web_search,web_fetch,search_replace"

# シェルアクセスを削除
grok -p "Review this code" --disallowed-tools "run_terminal_cmd"
```

`--disallowed-tools` では、サブエージェントの起動を制御する特別な `Agent` エントリも使用できます。

| エントリ                  | 効果                                  |
| ------------------------- | ------------------------------------- |
| `Agent`                | すべてのサブエージェント起動をブロック             |
| `Agent(explore)`       | `explore` サブエージェントタイプだけをブロック  |
| `Agent(explore, plan)` | 複数の特定タイプをブロック           |

```bash
# すべてのサブエージェント起動を禁止
grok -p "Fix this bug" --disallowed-tools "Agent"

# explore サブエージェントだけをブロック
grok -p "Refactor this module" --disallowed-tools "Agent(explore)"
```

`--tools` は、選択したエージェントプロファイルのツール追加方針を維持します。標準提供のプロファイルでは、許可リストを適用する前に有効なオプションツールが追加されますが、用途に合わせて選定されたプロファイルでは厳格な構成が維持されます。最終的なツールセットには、要求したツールと常時有効な MCP メタツールが残ります。両方のフラグがある場合は、`--disallowed-tools` が優先されます。

<a id="permission-rules-allow-deny"></a>

### 権限ルール（`--allow` / `--deny`）

権限ルールは、特定のツール呼び出しを自動承認するか、拒否するか、ユーザー確認を必要とするかを制御します。ツールを完全に削除する `--disallowed-tools` とは異なり、権限ルールではツールを利用可能なままにして、その実行を制限します。

ルールには `ToolPrefix(glob_pattern)` 構文を使用します。

| プレフィックス        | 制御対象                   |
| --------------------- | -------------------------- |
| `Bash(...)`   | シェルコマンドの実行            |
| `Edit(...)`   | ファイル編集（パス glob）           |
| `Write(...)`  | ファイル書き込み（パス glob）           |
| `Read(...)`   | ファイル読み取り（パス glob）           |
| `Grep(...)`   | 検索操作（パス glob）      |
| `WebFetch(...)` | URL 取得（glob または `domain:host`） |
| `MCPTool(...)` | MCP ツール呼び出し              |

パスルール（`Read`、`Edit`、`Write`、`Grep`）では、`*` は単一階層のワイルドカード、`**` は再帰ワイルドカードです。`Bash` ルールでは、`*` は空白を含む任意の文字に一致します。括弧のないプレフィックスはその種類のすべての呼び出しに一致し、`Bash(cmd:*)` は `cmd` のプレフィックス一致と同等です。完全な照合セマンティクスについては、[22-permissions-and-safety.md](22-permissions-and-safety.md#rule-matching-reference)を参照してください。

```bash
# "rm*" に一致するシェルコマンドを拒否
grok -p "Clean up this project" --deny "Bash(rm*)"

# npm コマンドを許可し、sudo を拒否
grok -p "Set up the project" --allow "Bash(npm*)" --deny "Bash(sudo*)"

# すべての bash コマンドを許可（確認なしで自動承認）
grok -p "Build the project" --allow "Bash"
```

`--allow` と `--deny` は複数回指定できます。拒否ルールは許可ルールより優先されます。

---

<a id="output-formats"></a>

## 出力形式

ヘッドレスモードは 3 種類の出力形式に対応し、`--output-format` で選択します。

<a id="plain-default"></a>

### plain（デフォルト）

人が読みやすいテキストです。直接表示したり、パイプで渡したりする用途に適しています。

```
Here's a summary of the codebase...
```

<a id="json"></a>

### json

応答完了後に 1 つの JSON オブジェクトを出力します。応答テキスト、停止理由、セッション ID、リクエスト ID（推論がある場合は `thought` も）を含みます。プロンプトがモデルに到達した場合、同じオブジェクトに支出フィールド（`usage`、`num_turns`、`modelUsage`、cost）も含まれます。

```json
{
  "text": "Here's a summary of the codebase...",
  "stopReason": "EndTurn",
  "sessionId": "abc123",
  "requestId": "xyz789",
  "num_turns": 7,
  "usage": {
    "input_tokens": 7210,
    "cache_read_input_tokens": 41000,
    "output_tokens": 1893,
    "reasoning_tokens": 412,
    "total_tokens": 50103
  },
  "modelUsage": {
    "grok-build": {
      "inputTokens": 7210,
      "outputTokens": 1893,
      "cacheReadInputTokens": 41000,
      "modelCalls": 7,
      "costUSD": 0.01268905
    }
  },
  "total_cost_usd": 0.01268905,
  "total_cost_usd_ticks": 126890500
}
```

使用量に関する注意事項:

- `usage` は、プロンプトのトークンと、ターン終了前に完了したサブエージェントのトークンを合計します（各サブエージェント独自の `modelUsage` キーにも記録）。圧縮やその他の補助モデル呼び出しは除外されます。
- **トークンフィールドのポリシー（ヘッドレス結果 / `end` / エラー時の支出）:**
  - `usage.input_tokens` と `modelUsage.*.inputTokens` は**キャッシュされていない分だけ**です。
  - `cache_read_input_tokens` / `cacheReadInputTokens` はキャッシュヒットです。
  - `total_tokens` は入力 + 出力の全量（キャッシュを含む）です。
    `total_tokens = input_tokens + cache_read_input_tokens + output_tokens`。
  - ACP `_meta.usage.inputTokens`（PromptUsage）は引き続きプロンプトの**全量**です。キャッシュ分を差し引くのは、ヘッドレス結果への変換処理だけです。支出の自動処理にはヘッドレスフィールドを推奨します。
- `num_turns` は、プロンプト台帳に記録されたメインエージェントのモデルラウンド（使用量を報告したツールループのラウンド）を数えます。サブエージェントによるサンプラーの呼び出しでは増えません。モデルごとの呼び出し回数（サブエージェントを含む）は `modelUsage.*.modelCalls` に記録されます。これは `--max-turns` と同系統のカウンターであり、ラウンドに使用量がない場合やゲートに達した場合に正確に一致する保証はありません。
- `total_cost_usd` は、サーバーが**完全な**コストを報告した場合にのみ含まれます。存在しない場合は未報告または不完全であり、無料という意味ではありません。現在、API キー経由の通信にはコストが記録されますが、pool/OAuth 経由では、サーバーがコストを記録するまで省略されることがよくあります。一部の呼び出しでコストが欠けていた場合は `cost_is_partial` が true になり、利用側がモデル行を合計して偽の完全な請求額を作らないよう、**すべて**のコスト浮動小数点値（`total_cost_usd` とすべての `modelUsage.*.costUSD`）が省略されます。
- `total_cost_usd_ticks` は、同じ値を正確な整数 tick（1 USD = 10^10 ticks）で表し、同じ条件で含まれます。請求照合にはこれを使用してください。呼び出しごとの tick の合計はサーバーの使用量エクスポートと正確に一致しますが、浮動小数点のドル値では保証できません。
- サブエージェントの使用量を適用できなかった場合、ネストしたサブエージェントの使用量が不完全だった場合、または正常終了時の完了待ち処理がタイムアウトした場合（ターンタスクで最大 120 秒）、`usage_is_incomplete` が true になり、同様にコスト浮動小数点値が省略されます（トークン合計でサブエージェント分が過少になる可能性があります）。キャンセル時のスナップショットでは、この長い完了待ち処理を行わず、サブエージェントがまだ動作中なら不完全と記録します。記録済みトークンがなく不完全な場合は、ゼロ値の `usage` オブジェクトを出さず、`usage_is_incomplete` だけを出力します。
- モデルに到達しなかったプロンプトでは、支出フィールドが省略されます。

`sessionId` フィールドは、後で会話を再開する際に役立ちます。

失敗時、Grok はエラーオブジェクトを出力します（プロセスの終了コードは 0 以外）。プロンプトレベルの失敗でも、使用量が記録されていれば確定した支出フィールドを含む場合があります。

```json
{"type":"error","message":"Couldn't start session: ..."}
```

<a id="streaming-json"></a>

### streaming-json

改行区切りの JSON イベントをリアルタイムに出力します。各行は、`type` フィールドを持つ自己完結した JSON オブジェクトです。

```json
{"type":"text","data":"Here's"}
{"type":"text","data":" a summary"}
{"type":"thought","data":"Analyzing the directory structure..."}
{"type":"end","stopReason":"EndTurn","sessionId":"abc123","requestId":"xyz789","usage":{...},"num_turns":7,"modelUsage":{...}}
```

イベントタイプ:

| タイプ       | 説明                                                    |
| ------------ | ------------------------------------------------------- |
| `text`     | エージェントの応答テキストの断片                            |
| `thought`  | 内部推論（thinking tokens）                            |
| `end`      | 利用可能な場合はメタデータと支出フィールドを含む最終イベント       |
| `error`    | エラー発生（`message` と、存在する場合は支出フィールドを含む）  |

`end` は常に最後のイベントです。`end` の支出フィールドは json オブジェクトと同じ形状です（snake_case のキャッシュされていない `input_tokens`、安全なコスト浮動小数点値）。

Grok は `max_turns_reached` や `auto_compact_*` イベントも出力する場合があります。リストは網羅的ではないものとして扱い、`type` で分岐してください。

---

<a id="session-management-in-headless-mode"></a>

## ヘッドレスモードでのセッション管理

デフォルトでは、`grok -p` を呼び出すたびに新しいセッションが作成されます。呼び出しをまたいでコンテキストを維持するには、セッションフラグを使用します。

<a id="named-sessions-s"></a>

### 名前付きセッション（`-s`）

ヘッドレス呼び出しをまたいでコンテキストを引き継ぐには、`-r/--resume` または `-c/--continue` を使用します。`-s/--session-id` は、**UUID** を指定して**新しい**セッションを作成する場合にのみ使用します（UUID でない場合、または対象ディレクトリですでに使用中の場合はエラー）。以前の非公開 `-s` upsert/再開動作は廃止されました。継続するには `-r`/`-c` を使用してください。`-r`/`-c` と併用する場合、`-s` には `--fork-session` が必要です。

```bash
# ヘッドレスセッションを開始し、その ID を取得
grok -p "Review the changes in this PR" --output-format json | jq -r '.sessionId'

# 同じセッションで継続
grok -p "Now check for security issues" --resume "<id>"

# 任意: クライアントが選んだ UUID で作成（未使用であること）
grok -p "hello" --session-id "$(uuidgen | tr '[:upper:]' '[:lower:]')" --output-format json
```

> **注:** `-s/--session-id` は新しいセッションだけを作成します（有効な UUID が必要で、使用中の場合はエラー）。再開には `-r` を使用してください。

<a id="resume-r"></a>

### 再開（`-r`）

`-r/--resume` フラグは、ID を指定して特定のセッションを再開します。セッションが存在しない場合はエラーになります。

```bash
# 以前の JSON 応答からセッション ID を取得
grok -p "Remember: the secret number is 42" --output-format json
# 出力に "sessionId": "abc123" が含まれる

# そのセッションを正確に再開
grok -p "What's the secret number?" --resume abc123
```

<a id="continue-c"></a>

### 継続（`-c`）

`-c/--continue` フラグは、現在の作業ディレクトリで最も新しいセッションを継続します。

```bash
grok -p "Continue where we left off" -c
```

<a id="extracting-session-ids"></a>

### セッション ID の抽出

`--output-format json` を使用し、`sessionId` フィールドを解析します。

```bash
grok -p "Hello" --output-format json | jq -r '.sessionId'
```

---

<a id="piping-input-and-output"></a>

## 入出力のパイプ処理

ヘッドレスモードは Unix のパイプやリダイレクトと自然に連携します。

<a id="standard-output"></a>

### 標準出力

```bash
# 出力をファイルへパイプ
grok -p "Generate a README" > README.md

# jq で JSON 出力を解析
grok -p "List files" --output-format json | jq -r '.text'
```

<a id="standard-input"></a>

### 標準入力

ヘッドレスモードは、パイプされた stdin をプロンプトに読み込みません。外部コンテンツはコマンド置換または `--prompt-file` で渡します。

```bash
# コマンド置換で git diff をコンテキストに含める
grok -p "Write a concise commit message for these changes:

$(git diff --staged)"

# またはファイルからプロンプトを読み込む
grok --prompt-file ./prompt.txt
```

---

<a id="ci-cd-integration-examples"></a>

## CI/CD 連携の例

<a id="automated-code-review"></a>

### コードレビューの自動化

```bash
grok -p "Review changes for bugs and security issues." \
  --output-format json --yolo | jq -r '.text' > review.md
```

<a id="pre-commit-hook"></a>

### Pre-Commit フック

```bash
grok -p "Review staged changes for obvious bugs. Reply OK if fine, or list issues." \
  --yolo --output-format json | jq -r '.text' | grep -q "^OK" || exit 1
```

<a id="batch-processing"></a>

### バッチ処理

```bash
for file in src/*.js; do
  grok -p "Migrate $file from CommonJS to ES modules." --yolo
done
```

---

<a id="scripting-patterns"></a>

## スクリプトのパターン

<a id="python-wrapper"></a>

### Python ラッパー

Grok のヘッドレスモードは、OpenAI 互換のチャット補完 API としてラップできます。

```python
import asyncio
import json
import os

class GrokChat:
    """ヘッドレスモードを使用する簡単な OpenAI 互換ラッパー。"""

    def __init__(self, cwd="."):
        self.cwd = cwd
        self.env = {**os.environ}

    def _build_cmd(self, prompt, model, stream):
        return ["grok", "-p", prompt, "-m", model, "--cwd", self.cwd,
                "--output-format", "streaming-json" if stream else "json",
                "--yolo"]

    async def create(self, messages, model="grok-build", stream=False):
        prompt = messages[-1]["content"] if len(messages) == 1 else "\n".join(
            f"{m['role']}: {m['content']}" for m in messages
        )
        cmd = self._build_cmd(prompt, model, stream)

        if stream:
            return self._stream(cmd)

        proc = await asyncio.create_subprocess_exec(
            *cmd, env=self.env, stdout=asyncio.subprocess.PIPE
        )
        stdout, _ = await proc.communicate()
        data = json.loads(stdout.decode()) if stdout else {"text": ""}
        return {
            "choices": [{
                "message": {"role": "assistant", "content": data.get("text", "")},
                "finish_reason": "stop"
            }]
        }

    async def _stream(self, cmd):
        proc = await asyncio.create_subprocess_exec(
            *cmd, env=self.env, stdout=asyncio.subprocess.PIPE
        )
        async for line in proc.stdout:
            if not line.strip():
                continue
            event = json.loads(line)
            if event.get("type") == "text":
                yield {"choices": [{"delta": {"content": event["data"]}}]}
            elif event.get("type") == "end":
                yield {"choices": [{"delta": {}, "finish_reason": "stop"}]}

async def main():
    client = GrokChat(cwd=".")
    response = await client.create(
        [{"role": "user", "content": "What files are here?"}]
    )
    print(response["choices"][0]["message"]["content"])

asyncio.run(main())
```

<a id="shell-script"></a>

### シェルスクリプト

```bash
#!/bin/bash
# コードレビューを実行し、問題が見つかった場合は失敗として終了

RESULT=$(grok -p "Review this PR for bugs. Output JSON with 'issues' array." \
  --output-format json --yolo | jq -r '.text')

ISSUE_COUNT=$(echo "$RESULT" | jq '.issues | length' 2>/dev/null || echo "0")

if [ "$ISSUE_COUNT" -gt 0 ]; then
  echo "Found $ISSUE_COUNT issues"
  echo "$RESULT" | jq '.issues[]'
  exit 1
fi

echo "No issues found"
```

---

<a id="fully-automated-runs-with-yolo"></a>

## `--yolo` による完全自動実行

`--yolo` フラグは常時承認モード（`--permission-mode bypassPermissions` および `--always-approve` と同じモード）を有効にし、ツール実行（ファイル書き込み、コマンド実行など）を確認なしで自動承認します。明示的な `deny` ルールと `PreToolUse` フックは引き続き適用され、管理者は `requirements.toml` でこのモードを無効化できます（[22-permissions-and-safety.md](22-permissions-and-safety.md)を参照）。無人自動化にはこのフラグが必要です。

```bash
# 確認せずにすべてのファイルを整形
grok -p "Format all files" --yolo

# テストを実行して失敗を修正
grok -p "Run the tests and fix any failures" --cwd ~/projects/my-app --yolo
```

**`--yolo` は慎重に使用してください。** エージェントに、ファイル変更やコマンド実行を行う完全な自律性を与えます。信頼できる環境、または範囲を明確に限定したプロンプトでのみ使用してください。

---

<a id="environment-variables-for-headless"></a>

## ヘッドレスモードの環境変数

ヘッドレスモードに影響する主な環境変数:

| 変数                        | 説明                                                   |
| --------------------------- | ------------------------------------------------------ |
| `XAI_API_KEY`        | 認証用 API キー（ブラウザーログインがない場合は必須）   |
| `GROK_HOME`                    | 設定ディレクトリを上書き（デフォルト: `~/.grok`）                |
| `GROK_LOG_FILE`                | ログファイルのパス（指定値をそのままパスとして使用。ヘッドレスと TUI の両方で動作し、`RUST_LOG` に従う） |
| `RUST_LOG`                     | ログレベルフィルター（例: `debug`）。ヘッドレスでは stderr に記録。     |

ブラウザーにアクセスできない CI 環境では、[console.x.ai](https://console.x.ai) で取得した API キーを `XAI_API_KEY` に設定します。

```bash
export XAI_API_KEY="xai-..."
grok -p "Run the test suite" --yolo
```

---

<a id="exit-codes"></a>

## 終了コード

| コード | 意味                              |
| ------ | --------------------------------- |
| `0`  | 成功 -- プロンプトが正常に完了 |
| `1`  | エラー -- 認証失敗、ネットワークエラー、または実行時エラー |
| `130` | SIGINT（Ctrl+C）による中断                                   |
| `143` | SIGTERM による終了                                            |

---

<a id="authentication-for-headless-environments"></a>

## ヘッドレス環境での認証

ヘッドレスで使用する場合は、次のいずれかで認証します。

- **`XAI_API_KEY`** -- CI では最も簡単です。上記の[環境変数](#environment-variables-for-headless)を参照してください。
- **`grok login --device-auth`**（または `--device-code`）-- 対象マシンにブラウザーは不要です。
  [認証 > デバイスコードフロー](02-authentication.md#device-code-flow)を参照してください。
- **`grok login`** -- GUI を備えたマシンでのブラウザーベース OAuth2。

以前にログイン済みの場合、キャッシュされた認証情報が自動的に使用されます。

---

<a id="tips"></a>

## ヒント

- ヘッドレスモードはデフォルトで**新しいセッション**を開始します。呼び出しをまたいでコンテキストを維持するには、`-r/--resume` または `-c/--continue` を使用します。
- `--output-format json` の応答には、後続の呼び出しで `--resume` に使用できる `sessionId` が必ず含まれます。
- `--yolo` と `--rules` を組み合わせてガードレールを設定できます: `grok -p "..." --yolo --rules "Never delete files"`。
- デバッグ時はログレベルを上げ、stderr を取得します: `RUST_LOG=debug grok -p "..." 2> debug.log`。

---

<a id="project-root-discovery"></a>

## プロジェクトルートの検出

Grok は起動時、`--cwd`（または現在のディレクトリ）から上位へたどり、`.git` ディレクトリを見つけることでプロジェクトルートを検出します。

注: `--cwd` が大規模リポジトリ（モノレポなど）の内部にある場合、Grok はそのリポジトリをプロジェクトルートとして検出し、検出処理（AGENTS.md、skills、git 履歴）の範囲をリポジトリ全体にするため、起動が遅くなることがあります。作業対象のサブプロジェクトを `--cwd` に指定し、範囲を小さくしてください。

---

<a id="file-locations"></a>

## ファイルの保存場所

Grok はデータを `~/.grok` に保存します（`GROK_HOME` で上書き可能。[ヘッドレスモードの環境変数](#environment-variables-for-headless)を参照）。

| パス                     | 内容                              |
| ------------------------ | --------------------------------- |
| `config.toml`            | ユーザー設定                    |
| `auth.json`              | キャッシュされた OAuth2/API 認証情報         |
| `version.json`           | 更新確認用のバージョンキャッシュ       |
| `sessions/`              | セッション記録（SQLite）          |
| `memory/`                | セッション間メモリストア            |
| `logs/`                  | 内部ログファイル（例: `unified.jsonl`） |
| `logs/mcp/`              | MCP サーバーログ                       |
| `skills/`                | ユーザースキル定義                |
| `personas/`              | ユーザー単位のエージェント人格設定            |
| `crash/`                 | クラッシュレポート                         |
| `trace-exports/`         | セッショントレースのエクスポート                 |
| `worktrees/`             | Git worktree メタデータ                 |

<a id="read-only-grok"></a>

### 読み取り専用の `~/.grok`

コンテナや CI では、`~/.grok` を読み取り専用でマウントできます。

- `auth.json` を事前に配置するか、`XAI_API_KEY` を使用
- セッションの永続化は通知なしで失敗（一時セッションになる）
- 更新確認は警告をログに記録してスキップ

```bash
export XAI_API_KEY="xai-..."
export GROK_DISABLE_AUTOUPDATER=1
grok -p "..." --no-auto-update
```

---

<a id="update-check-suppression"></a>

## 更新確認の抑制

| 方法                          | スコープ     |
| ----------------------------- | ------------ |
| `--no-auto-update`              | セッション   |
| `GROK_DISABLE_AUTOUPDATER=1`    | プロセス   |
| TTY ではない stderr（自動検出）  | 自動 |
| `[cli] auto_update = false`     | 永続 |

更新メッセージは **stderr** に出力されます。stdout は `--output-format json` のためにクリーンな状態を保ちます。[ヘッドレスモードの環境変数](#environment-variables-for-headless)も参照してください。

---

<a id="additional-headless-flags"></a>

## その他のヘッドレスフラグ

以下のフラグは、上記の[コマンドラインオプション](#command-line-options)の表を補足するものです。すでに記載したフラグ（`--prompt-json`、`--prompt-file`、`--verbatim`、`--sandbox`、`--no-auto-update`）は重複して掲載しません。

| フラグ                          | 説明                                       |
| ------------------------------- | ------------------------------------------ |
| `--agent <NAME>`              | エージェント名または定義ファイルのパス                |
| `--agents <JSON>`             | JSON 形式のインラインサブエージェント定義               |
| `--system-prompt-override`    | エージェントのシステムプロンプトを上書き                |
| `--check` / `--self-verify`   | 検証ループを追加（ヘッドレス専用）          |
| `--best-of-n <N>`             | タスクを N 通り実行し、最良を選択（ヘッドレス専用）         |
| `--no-plan`                   | プランモードを無効化                                 |
| `--no-subagents`              | サブエージェントの起動を無効化                         |
| `--no-memory`                 | セッション間メモリを無効化                      |
| `--disable-web-search`        | Web 検索・取得ツールを無効化                |
| `--no-alt-screen`             | インラインで実行（代替画面を使用しない）                  |
| `--worktree [NAME]`           | 新しい git worktree でセッションを開始               |
| `--ref <REF>` / `--worktree-ref <REF>` | worktree の基点とするブランチ／タグ／コミット（`--worktree` と併用） |

---

<a id="interrupted-headless-runs"></a>

## 中断されたヘッドレス実行

SIGINT/SIGTERM を受信した場合:

- 最後に完了したツール呼び出しまでのセッション状態を保存
- ツールによるファイル変更は**ロールバックされない**
- 終了コードは SIGINT で **130**（`128 + 2`）、SIGTERM で **143**（`128 + 15`）。CI パイプラインでは通常のエラー（終了コード `1`）と区別可能
- 再開: `grok -p "continue" --resume "<id>"` または `grok -p "continue" --continue`

名前付きセッションと `-s`/`-r`/`-c` フラグの詳細については、[ヘッドレスモードでのセッション管理](#session-management-in-headless-mode)を参照してください。
