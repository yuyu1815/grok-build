# 権限と安全制御

Grok は、ファイルの読み取り、コードの検索、ファイルの編集、シェルコマンドの実行ができます。権限システムは、エージェントに許可する操作を制御します。権限ルール、権限モード、フック、OS レベルのサンドボックスという、複数の独立したレイヤーを組み合わせられます。

このガイドでは、ツール呼び出しが承認される仕組み、CLI・ネイティブ設定・Claude 設定から権限ルールを構成する方法、すべてのモードに適用される許可リストを `PreToolUse` フックで実現する方法を説明します。

---

<a id="how-a-tool-call-is-authorized"></a>

## ツール呼び出しの承認方法

モデルがツールを要求すると、次のチェックが順番に行われます。

1. **`PreToolUse` フック**。フックは、ほかのチェックより前にツール呼び出しを拒否できます。フックが呼び出しを許可しても、以降のチェックは省略されません。単に拒否しないだけです。[10-hooks.md](10-hooks.md) を参照してください。

2. **権限ルール**（設定ファイルまたは `--allow` / `--deny` フラグから取得）
   - 一致する `deny` ルールがあると、呼び出しは拒否されます。`deny` はほかのすべてのルールより優先されます。
   - 一致する `ask` ルールがあると、通常なら自動承認されるファイル読み取り、検索、シェルコマンドも含め、ユーザーに確認します。
   - 一致する `allow` ルールがあると、呼び出しは承認されます。

3. **記憶された許可**。以前の確認で保存したコマンド単位の承認が、現在のプロジェクトに限定して適用されます。既存の許可がある場合、`ask` ルールに対して再確認せず承認できます。[危険なコマンドの一覧](#dangerous-commands)にあるコマンドは、記憶されたプレフィックスを使わず再度確認します。[対話型の承認](#interactive-approvals-and-where-they-persist)を参照してください。

4. **組み込みの自動承認**。読み取り専用ツールと、所定の読み取り専用シェルコマンドは確認なしで実行されます（後述）。

5. **確認ポリシー**（[権限モード](#permission-modes)で設定）。ユーザーへの確認、自動承認、自動拒否のいずれかを行います。

常時承認モード（`bypassPermissions`）では、ステップ 2 の後でこのパイプラインを打ち切ります。`deny` ルール、フック、シェルコマンドの各セグメントに一致する `ask` ルールは引き続き適用されますが、記憶された許可（記憶された「常に拒否」エントリを含む）は参照されず、シェル以外のツールに対する `ask` ルールでは確認しません。

---

## デフォルトで確認しない操作

以下の操作は読み取り専用として扱われ、一致する `deny` ルールまたはフックでブロックされない限り、`dontAsk` を含むすべてのモードで確認なしに実行されます。`ask` ルールを指定すると、ファイル読み取り、検索、シェルコマンドで確認を必須にできます（[ツール呼び出しの承認方法](#how-a-tool-call-is-authorized)を参照）。

### 読み取り専用ツール

- `read_file`
- `list_dir`
- `grep`（コンテンツ検索）
- `web_search`
- `todo_write`
- `get_command_or_subagent_output` / `wait_commands_or_subagents` / `kill_command_or_subagent`（サブエージェントの制御）
- スキルの呼び出し

### 読み取り専用シェルコマンド

連結されたコマンドを（`&&`、`||`、`;`、パイプで）分割した後、次のコマンドがプライマリコマンドとして現れる場合は読み取り専用と認識されます。この一覧は単語境界で照合されるため、`ls` が `lsof` や `less` に一致することはありません（独自の `Bash(...)` ルールは異なる方法で照合されます。[ルール照合リファレンス](#rule-matching-reference)を参照してください）。

**ファイルシステム（読み取り専用の表示）：**
- `ls`, `cat`, `pwd`, `date`, `whoami`, `hostname`, `uptime`, `ps`
- `head`, `tail`, `wc`, `sort`, `uniq`, `tr`, `cut`

**Git（読み取り専用）：**
- `git status`, `git branch`, `git log`, `git diff`, `git ls-files`, `git show`, `git rev-parse`

**検索と調査：**
- `grep`, `rg`（ファイルごとにプリプロセッサーを起動する `rg --pre` / `rg --pre=…` は除く）

**ビルドとチェック（読み取り専用）：**
- `cargo check`

**Kubernetes（読み取り専用）：**
- `kubectl get`, `kubectl logs`, `kubectl describe`

> **注:** `tee` は入力を任意のファイルへ書き込めるため、この一覧には含まれません。

これらのチェックはセグメントごとに適用されます。`ls && rm -rf /` のようなコマンドでは、`ls` セグメントは読み取り専用と認識されますが、`rm` セグメントは一覧にありません。`default` モードでは `rm` セグメントについて確認し、`dontAsk` では拒否します。

---

<a id="permission-modes"></a>

## 権限モード

確認ポリシーには、次のいずれかのモード名を指定します。

| モード                | 動作                                                                 | 主な用途                     |
|---------------------|--------------------------------------------------------------------------|---------------------------------|
| `default`           | 事前承認されていないすべての操作について確認する                                     | 日常的な対話操作           |
| `dontAsk`           | 明示的な許可ルールまたは組み込みの自動承認がない操作をすべて拒否する   | ヘッドレス、CI、高セキュリティ     |
| `bypassPermissions` | ツール呼び出しを自動承認する（`deny` ルール、フック、シェルの `ask` ルールは引き続き適用） | 信頼できる環境    |
| `acceptEdits`       | ファイル編集（`search_replace`、`write` など）を自動承認する                | 「編集を承認」するワークフロー        |
| `plan`              | 互換性のために受け付ける。プランセッションは別機能（[19-plan-mode.md](19-plan-mode.md) を参照） | 構造化された計画セッション |

<a id="setting-the-mode"></a>

### モードの設定

モードは `.claude/settings.json` の `defaultMode` で設定します（[Claude Code 互換性](#3-claude-code-compatibility-claudesettingsjson)を参照）。`dontAsk`、`acceptEdits`、`bypassPermissions` は確認ポリシーを変更します。`default` と `plan` では標準の確認動作が維持されます。

`--permission-mode` CLI フラグでは、`bypassPermissions`（常時承認）と `default` が適用されます。フラグで明示した値は、設定内のモードより常に優先されます。このフラグに `dontAsk`、`acceptEdits`、`plan` を渡すことはできますが、そのポリシーは有効になりません。代わりに `defaultMode` で設定してください。

ヘッドレス実行（`-p`）では、確認が必要なツール呼び出しは入力を待たずにキャンセルされ、その旨がモデルに報告されます。自動化でデフォルト拒否にするには、`defaultMode: "dontAsk"` を設定します。

### 常時承認モードの無効化

管理者は常時承認（`bypassPermissions` / `--always-approve`）を無効にし、CLI、TUI のトグル、`/always-approve` コマンドから有効化できないようにできます。`requirements.toml` に専用キーを設定してください。

```toml
[ui]
disable_bypass_permissions_mode = true   # デフォルト: false。true = 無効に固定。
```

この用途に `permission_mode` を使用しないでください。これはユーザーが切り替えられるデフォルト値であり、ロックではありません。後方互換性のため、`requirements.toml` の旧式キー `[ui] yolo = false` でもこのモードを無効化できます。`config.toml` では、同じキーは引き続き切り替え可能な設定です。

ユーザーレベルの `~/.grok/requirements.toml` はユーザー自身が管理できるため、開発者はこのファイルを編集してロックを解除できます。ユーザーが上書きできないよう強制するには、root が所有するシステムファイル `/etc/grok/requirements.toml` に設定を配布してください。

> **注:** Grok は Claude Code の `managed-settings.json` にある権限ルールを尊重しますが、`disableBypassPermissionsMode` ロックは尊重しません。Grok で常時承認を無効にするには、上記のとおり `requirements.toml` を使用してください。

---

## 権限の設定

Grok は、互換性のある 3 種類のソースから権限ルールを読み取ります。すべてのソースのルールは 1 つのセットに統合されます。ルールの効果は取得元のファイルではなく、アクション（`deny` > `ask` > `allow`）で決まります。

### 権限ルールの保存場所（スコープ）

権限ルールは、グローバル（全プロジェクト）、プロジェクト単位（1 つのリポジトリ）、またはプロジェクト内の個人用として設定できます。

| スコープ | ファイル | チームメイトとの共有 |
|-------|------|-----------------------|
| グローバル（全プロジェクト） | `~/.grok/config.toml` | いいえ |
| プロジェクト（コミット対象） | `<project>/.grok/config.toml` | はい（コミットする） |
| プロジェクト（個人用） | `<project>/.claude/settings.local.json` | いいえ（gitignore に追加する） |
| 対話型の許可 | Grok がプロジェクト単位で内部保存 | いいえ |

スコープに関する注意事項：

- Grok は、リポジトリルートから作業ディレクトリまでの各階層で `.grok/config.toml` を検出します。そのため、サブディレクトリではリポジトリルートのルールに追加できます。
- すべてのスコープのルールは 1 つのルールセットに統合され、スコープをまたいで `deny` > `ask` > `allow` が適用されます。そのため、グローバルの `deny` をプロジェクトの `allow` で上書きすることはできません。
- Grok にはネイティブの `config.local.toml` はありません。プロジェクト内で個人用の未コミットルールを使うには、`.claude/settings.local.json` を使用してください。Grok はこれを直接読み取ります（[Claude Code 互換性](#3-claude-code-compatibility-claudesettingsjson)を参照）。
- 対話型の「常に許可」の決定はリポジトリ外に保存され、プロジェクト単位で適用されます（[対話型の承認](#interactive-approvals-and-where-they-persist)を参照）。

1 つのプロジェクトで特定のコマンドに対する確認をなくすには、そのプロジェクトの `.grok/config.toml`（または `.claude/settings.json`）に範囲の狭い許可ルールを追加します。

```toml
[permission]
allow = ["Bash(cargo test *)", "Bash(npm run build)"]
```

これは記載されたコマンドだけを承認します。対照的に、常時承認モードはすべてのツール呼び出しを承認します。

### 1. CLI フラグ

```bash
grok -p "Review the API changes" \
  --allow 'Bash(git *)' \
  --allow 'Bash(gh *)' \
  --allow 'Read' \
  --allow 'Grep' \
  --deny 'Bash(rm -rf *)'
```

`--allow RULE` と `--deny RULE` は繰り返し指定でき、常に適用されます。

ルール構文の例：
- `Bash(git *)` — `git ` で始まる任意のコマンド
- `Bash(npm run build)` — 完全一致するコマンド（またはプレフィックス）
- `Bash(git commit:*)` — `cmd:*` サフィックス形式。`git commit` のプレフィックス照合と同等
- `Read(src/**)` — `src/` 以下の読み取りアクセス
- `Edit(**/*.rs)` — 任意の Rust ファイルを編集
- `Grep` — すべての grep 操作
- `MCPTool(my-server__*)` — 特定のサーバーの MCP ツール

連結コマンドやワイルドカードの評価方法を含む正確な照合動作については、[ルール照合リファレンス](#rule-matching-reference)を参照してください。

### 2. ネイティブ設定（`~/.grok/config.toml` と `.grok/config.toml`）

```toml
[permission]
rules = [
  { action = "allow", tool = "bash", pattern = "git *" },
  { action = "allow", tool = "bash", pattern = "gh *" },
  { action = "allow", tool = "read" },
  { action = "allow", tool = "grep" },
  { action = "deny",  tool = "bash", pattern = "rm -rf *" },  # 危険なパターンをブロック
  { action = "ask",   tool = "edit" },
]
```

構造化された `tool` フィールドでは、小文字の名前 `bash`、`read`、`edit`、`grep`、`mcp`、`webfetch`、`websearch` を使用できます。これらは[ツール名](#tool-names)のツールクラスに対応します。

`deny` は常に優先されるため、「git/gh だけを許可する」という意味で、これらの `allow` ルールと `bash` の包括的な `deny` を併用することはできません。`deny tool = "bash"` ルールは `git` と `gh` もブロックします。デフォルト拒否にするには、`.claude/settings.json` の `defaultMode: "dontAsk"` または `PreToolUse` フック（後述）を使用してください。

グローバルの `~/.grok/config.toml` と、すべてのプロジェクトの `.grok/config.toml`（リポジトリルートから作業ディレクトリまで）のルールは、`.claude/settings.json` のルールとともに 1 つのルールセットへ統合されます。

組織が配布する管理対象設定も `[permission]` ルールを追加します。対象はシステムの `/etc/grok/managed_config.toml` と、Grok が `~/.grok/managed_config.toml` に自動維持するユーザーレベルのコピーです。管理対象ルールはほかのソースのルールと同様に統合されますが、管理対象の `allow` ルールには固有の性質が 2 つあります。ユーザー自身の `deny` と `ask` ルールは管理対象の `allow` より優先され（重大度順）、常時承認が無効に固定されている場合、包括的な管理対象 `allow` は無視されます。ユーザーが編集して解除できないルールには、root が所有するシステムの `/etc/grok/requirements.toml` を使用してください。

すべてのソースの権限ルールは、セッション開始時に一度だけ読み取られます。変更は次のセッションから適用されます。

ネイティブの `[permission]` セクションでは、`--allow` / `--deny` フラグや `.claude/settings.json` と同じルール文字列を使う、簡潔な `allow` / `deny` / `ask` 文字列配列形式も使用できます。

```toml
[permission]
deny = [
  "Read(/Users/you/private/**)",
  "Edit(/Users/you/private/**)",
  "Bash(rm -rf *)",
]
allow = [
  "Bash(git *)",
  "Bash(gh *)",
]
```

順序や取得元にかかわらず、`deny` は常に `allow` より優先されます（評価順は `deny` > `ask` > `allow`）。OS レベルでもプロジェクト外のパスの読み取りをブロックするには、拒否ルールと `strict` サンドボックスプロファイルを組み合わせてください（[18-sandbox.md](18-sandbox.md) を参照）。

<a id="3-claude-code-compatibility-claudesettingsjson"></a>

### 3. Claude Code 互換性（`.claude/settings.json`）

Grok は `~/.claude/settings.json` と `~/.claude/settings.local.json` に加え、プロジェクトレベルの `<project>/.claude/settings.json` と `settings.local.json`（リポジトリルートまで遡る）を読み取ります。権限ルールのネイティブな `.grok` ソースは、前節で説明した `config.toml` です。

例：

```json
{
  "permissions": {
    "defaultMode": "dontAsk",
    "allow": [
      "Read",
      "Grep",
      "Bash(git *)",
      "Bash(gh *)"
    ],
    "deny": [
      "Bash(rm -rf *)"
    ]
  }
}
```

対応する `defaultMode` の値は、`default`、`acceptEdits`、`bypassPermissions`、`dontAsk`、`plan` です。Grok は、`permissions` 内の正規の位置から `defaultMode` を読み取ります。ネストされたキーがない場合は、トップレベルの `defaultMode` も受け付けます。

`permissions.allow`、`permissions.deny`、`permissions.ask` のエントリはネイティブルールに変換され、[ルール照合リファレンス](#rule-matching-reference)の動作で照合されます。変換に関する注意事項：

- MCP ツールのルールには `MCPTool(server__tool)` 形式を使用する必要があります。`mcp__server__tool` 形式は決して一致しません（[MCP ルール](#mcp-rules)を参照）。
- 認識されないツール名のルールと、`Agent(model:opus)` のようなパラメータールールは、読み込みを失敗させるのではなく、警告を表示してスキップされます。
- `permissions.additionalDirectories` は解析されますが、サポートされていません。

**Ctrl+I**（「Import Claude settings」）で、既存の Claude 設定を対話形式でインポートできます。

---

<a id="rule-matching-reference"></a>

## ルール照合リファレンス

このセクションでは、ルールの正確な照合方法を説明します。

### Bash ルール

`Bash(...)` パターンは、次の 2 通りのいずれかでコマンドに一致します。

- **プレフィックス**：コマンドがパターン文字列で始まるかを、1 文字ずつ比較します。単語境界の要件はないため、`Bash(git)` は `git status` だけでなく `gitleaks` にも一致します。プレフィックスを単語全体に限定するには、末尾の空白とワイルドカードを含めます（`Bash(git *)`）。
- **Glob**：パターンを glob としてコマンド全体に照合します。`*` は任意の位置に置くことができ、空白やスラッシュを含む任意の文字に一致します。そのため、`Bash(git * main)` は `git checkout main` に一致します。`?` と `[...]` も使用できます。

照合では大文字と小文字が区別されます。コマンドの先頭の空白は照合前に削除されますが、それ以外は正規化されません。

Bash ルール末尾の `:*` サフィックスは削除され、単純なプレフィックスになります。`Bash(git commit:*)` はプレフィックス `git commit` になります。プレフィックスには単語境界がないため、`Bash(sed:*)` と記述した `deny` は `sed-custom` のようなコマンドもブロックします。

**連結コマンド。** Grok は各コマンドをシェルと同様に解析し、`&&`、`||`、`;`、`|`、改行で分割します。ルールのアクションによって、セグメントの扱いが異なります。

- `deny` と `ask` ルールは、すべてのセグメントと文字列全体に対してチェックされます。1 つでも拒否されるセグメントがあれば、コマンド全体が拒否されます。
- `allow` ルールは、コマンド文字列全体に対してのみチェックされます。そのため、全文字列が `git ` で始まる `git status && rm -rf /` は、`Bash(git *)` によって自動承認されます。範囲の狭い許可ルールには、ブロックしたいパターンの `deny` ルールを組み合わせてください。

単純なセグメントに分割できないコマンド（サブシェル、コマンド置換 `$(...)`、バッククォート、バックグラウンド実行 `&`、制御フロー）は、Bash 制限が設定されている場合、1 つの単位として確認されます。

セグメント単位のチェック（`deny` と `ask` ルール、記憶された許可、読み取り専用コマンド一覧）では、`RUST_LOG=debug` のような環境変数プレフィックスを取り除き、所定のプロセスラッパー（`timeout`、`nice`、`ionice`、`chrt`、`stdbuf`、`env`）を外します。これにより、`deny` と `ask` ルールはラップされたコマンドと内部コマンドのどちらにも一致します。`bash -c` に渡されたインラインスクリプト内でも、`deny` と `ask` ルールがチェックされます。`sudo`、`xargs`、`nohup` など、ほかのラッパーは外されません。それらを明示的に含むルールを記述してください。`allow` ルールにはこの処理が適用されません。記述されたままのコマンド文字列に照合されるため、先頭に環境変数の代入やラッパーがあると `allow` ルールに一致せず、代わりに確認が行われます。

<a id="dangerous-commands"></a>

### 危険なコマンド

組み込みの一覧（`rm`、`chmod`、`chown`、`chgrp`、`chattr`、`pkill`、`kill`、`killall`、`git push`）にあるコマンドは、セグメントが記憶されたコマンドプレフィックスまたは読み取り専用コマンド一覧の対象でも確認されます。設定内の明示的な `allow` ルールでは承認でき、常時承認モードでもほかのコマンドと同様に自動承認されます。無条件にブロックするには `deny` ルールを使用してください。`Bash(rm *)` のようなルールを許可ルールとして追加する前に、慎重に確認してください。

### Read、Edit、Grep ルール

パスパターンは、ツールの呼び出し時に指定されたパス文字列に対して glob として照合されます。

- `*` と `?` は `/` をまたぎませんが、`**` はまたぎます。`Read(src/*)` は `src/main.rs` には一致しますが、`src/nested/mod.rs` には一致しません。ツリー全体には `Read(src/**)` を使用してください。
- ファイル名だけの指定は、その文字列と完全一致する場合に限り一致します。任意の階層の `.env` に一致させるには `**/.env` を使用してください。
- アンカープレフィックスはありません。パターン先頭の `//` や `~/` は、glob のリテラル文字列として扱われます。代わりに絶対パスのパターンまたは `**/` パターンを記述してください。
- パスは正規化されず、指定されたまま照合されます。絶対パスと相対パスのどちらになるかはツールの呼び出し方によるため、境界として使用するパターンでは両方の形式を対象にしてください（たとえば `/repo/secrets/**` と `secrets/**` の両方）。
- `Read` ルールは `grep` 検索も制御します。`Grep(...)` ルールは grep のみに一致します。

`Read` と `Edit` の拒否ルールは、シェルコマンドが操作するファイルパス（たとえば拒否対象パスに対する `cat` や `sed`）にも適用され、そのシェルレベルのチェックではシンボリックリンクが解決されます。直接の `read_file` / `search_replace` ツールのチェックでは、シンボリックリンクは解決されません。すべてのプロセスを対象とする OS レベルの強制には、拒否ルールとサンドボックスを組み合わせてください（[18-sandbox.md](18-sandbox.md)）。

<a id="mcp-rules"></a>

### MCP ルール

`MCPTool(...)` パターンは、glob を使用して `server__tool` 形式の Grok ツール名全体に一致します。`MCPTool(linear__*)` は、`linear` サーバーのすべてのツールに一致します。Grok のツール名には `mcp__` プレフィックスがないため、`mcp__server__tool` と記述したルールが MCP 呼び出しに一致することはありません。代わりに `MCPTool(server__tool)` と記述してください。

### WebFetch ルール

- `WebFetch(domain:example.com)` は、大文字と小文字を区別せず、先頭の `www.` を無視して、そのホストとすべてのサブドメイン（`api.example.com`）に一致します。`domain:` パターン内ではワイルドカードを使用できません。
- `domain:` プレフィックスのないパターンは、URL 全体に対して glob で照合されます：`WebFetch(https://api.example.com/*)`。

<a id="tool-names"></a>

### ツール名

認識されるツール名：`Bash`、`Read`（および `NotebookRead`）、`Edit`（および `Write`、`NotebookEdit`）、`Grep`（および `Glob`）、`MCPTool`、`WebFetch`、`WebSearch`。`*` だけのルールはすべてのツールに一致します。ツール名の位置では glob を使用できません。

認識されないツール名（たとえば `Agent(model:opus)`）のルールは、読み込みを失敗させるのではなく、警告を表示してスキップされます。

### 評価順

すべてのソースのルールは 1 つのセットに統合され、記述順ではなく重大度順に評価されます。一致する `deny` があれば拒否し、それ以外で一致する `ask` があれば確認し、それ以外で一致する `allow` があれば承認します。どのルールにも一致しない場合、[ツール呼び出しの承認方法](#how-a-tool-call-is-authorized)で説明したとおり、組み込みの自動承認、確認ポリシーの順に処理されます。

---

<a id="interactive-approvals-and-where-they-persist"></a>

## 対話型の承認と保存場所

ツール呼び出しに承認が必要な場合、権限確認には次の選択肢が表示されます。

- **1 回だけ許可**：この呼び出しだけを承認します。
- **1 回だけ拒否**：呼び出しを拒否します。必要に応じて、モデルへのメッセージも指定できます。
- **常時承認モードを有効化**：確認中の呼び出しだけでなく、以降のすべてのツール呼び出しを承認します。
- **このセッションのすべての編集を許可**：ファイル編集時に表示されます。この許可はメモリ内だけに保持され、再起動後には残りません。

### コマンド単位の「常に許可」

より限定的な選択肢では、確認中の特定のコマンド、MCP ツール、または web-fetch ドメインだけを記憶できます。たとえば「`cargo test` を常に許可」です。これらの行はデフォルトで無効です。次の設定で有効にできます。

```toml
# ~/.grok/config.toml
[ui]
remember_tool_approvals = true
```

この機能を有効にすると、確認画面に次の項目が追加されます。

- **`Always allow: <command>`**：コマンドプレフィックスの許可を永続化します。
- 対応する「常に拒否」の行：同様の方法で拒否を永続化します。
- MCP ツールと web-fetch ドメインに対応する「常に許可」の行。

記憶されるプレフィックスは、コマンドの短い形式に限定されます。読み取り専用コマンドでは一覧に記載されたプレフィックスだけ（たとえば引数一覧全体ではなく `git status`）が保存され、それ以外のコマンドでは先頭の短いプレフィックスが保存されます。確定前に、何が記憶されるかが確認画面に正確に表示されます。[危険なコマンドの一覧](#dangerous-commands)にあるコマンドは、記憶されたプレフィックスを使わず再度確認します。

### プロジェクト単位の永続化

対話型の許可はホームディレクトリ配下にある Grok 独自の状態ディレクトリへ保存され、Grok を起動したディレクトリに限定して適用されます。あるプロジェクトで付与した許可が別のプロジェクトに適用されることはありません。許可はリポジトリには書き込まれず、手動編集を想定していません。

対話型の許可は、個人用かつマシン単位の状態です。コードレビューで確認し、チームメイトと共有できる許可リストには、代わりにプロジェクトの `.grok/config.toml` で宣言的なルールを使用してください。

---

## フックで Bash を特定のコマンドに制限する

`PreToolUse` フックを使うと、すべての権限モードで適用される許可リストを `Bash` ツールに強制できます。フックは権限システムより前に評価されます。フックの拒否は呼び出しを停止し、フックの許可は通常の権限チェックに処理を渡します（そのため、`deny` ルールは引き続き適用されます）。

> **注:** フックはフェイルオープンです。フックスクリプトがクラッシュする、タイムアウトする、または存在しない場合、ツール呼び出しはフックが許可したものとして続行され、失敗は UI に報告されます。セキュリティ境界として使用するフックは、自身のエラーを処理し、次の例のように連結コマンドを考慮する必要があります。[10-hooks.md](10-hooks.md) を参照してください。

### 例：`git` と `gh` だけを許可する

**`~/.grok/hooks/git-gh-only.json`**

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "git-gh-only.sh",
            "timeout": 5
          }
        ]
      }
    ]
  }
}
```

**`~/.grok/hooks/git-gh-only.sh`**

```bash
#!/bin/sh
# 連結コマンド内も含め、git と gh コマンドだけを許可する。

set -eu

deny() {
  echo '{"decision": "deny", "reason": "'"$1"'"}'
  exit 2
}

INPUT=$(cat)
CMD=$(echo "$INPUT" | jq -r '.toolInput.command // empty')

[ -n "$CMD" ] || deny "Empty command is not allowed"

# 連結をセグメント単位で確認できるよう、'&&' と '||' を ';' に正規化し、
# このスクリプトで検査できない構文を拒否する。
CMD=$(echo "$CMD" | sed 's/&&/;/g; s/||/;/g')
case "$CMD" in
  *'$('*|*'`'*|*'&'*|*'>'*|*'<'*) deny "Substitution, background, and redirection are not permitted" ;;
esac

# 区切り文字で分割し、各セグメントが git または gh で始まることを必須にする。
echo "$CMD" | tr ';|' '\n\n' | while IFS= read -r SEGMENT; do
  SEGMENT=$(echo "$SEGMENT" | sed 's/^[[:space:]]*//')
  [ -n "$SEGMENT" ] || continue
  case "$SEGMENT" in
    git\ *|git|gh\ *|gh) ;;
    *) deny "Only git and gh commands are permitted. Blocked segment: $SEGMENT" ;;
  esac
done
```

```bash
chmod +x ~/.grok/hooks/git-gh-only.sh
```

このフックは、連結された各セグメントが `git` または `gh` で始まらない限り、すべての `Bash` コマンドを拒否します。また、実行内容を検証できないため、コマンド置換、バックグラウンド実行、リダイレクトも無条件に拒否します。このフックはすべての権限モードで動作します。

フックのインストール、JSON 形式、プロジェクトフックの信頼モデル、その他のイベントについては、[10-hooks.md](10-hooks.md) を参照してください。このガイドには、補完的な「危険なパターンをブロックする」例もあります。

---

## 設定例

### git と gh だけを使うヘッドレス実行（CI と自動化）

```bash
grok -p "Implement the feature using only git and GitHub CLI" \
  --allow 'Read' \
  --allow 'Grep' \
  --allow 'Bash(git *)' \
  --allow 'Bash(gh *)'
```

上記の `git-gh-only` フックをインストールし、それ以外のすべての `Bash` コマンドを拒否します。すべてのツールをデフォルト拒否にするには、さらに `.claude/settings.json` に `{"permissions": {"defaultMode": "dontAsk"}}` を設定してください。

### 読み取り専用のコードレビュー

```toml
# .grok/config.toml
[permission]
rules = [
  { action = "allow", tool = "read" },
  { action = "allow", tool = "grep" },
  { action = "deny",  tool = "edit" },
  { action = "deny",  tool = "bash" },
]
```

### 対話型の開発

`default` モードに、よく実行するコマンド（`git`、`cargo test`、`rg` など）向けの範囲の狭い `Bash(...)` 許可ルールを組み合わせます。

---

## サンドボックスとの併用

権限は、モデルが要求できる操作を制御します。OS レベルのサンドボックス（[18-sandbox.md](18-sandbox.md) を参照）は、コマンドが承認された後でも、プロセスが実行できる操作を制御します。

信頼できないコードには、次の組み合わせを推奨します。

1. `dontAsk` と範囲の狭い許可ルール、または制限の厳しいフック
2. `--sandbox strict` またはカスタムプロファイル
3. プロジェクトの信頼確認と、すべての `SessionStart` フックのレビュー

---

## TUI での権限管理

- 権限に関する決定はトランスクリプトに表示されます。
- `/always-approve` コマンドは常時承認モードを切り替えます。その他のモードは `defaultMode` で設定します（[モードの設定](#setting-the-mode)を参照）。
- `[ui] remember_tool_approvals = true` を設定すると、権限確認にコマンド単位の「常に許可」オプションが表示され、現在のプロジェクトだけに永続化されます。[対話型の承認](#interactive-approvals-and-where-they-persist)を参照してください。
- フックとプラグインを管理するには、`/hooks` または `/plugins` を実行します（ほとんどのターミナルでは **Ctrl+L** でも Extensions モーダルが開きます。VS Code、Cursor、Windsurf、Zed では、`Ctrl+L` は代わりにターン途中の割り込みです）。[10-hooks.md](10-hooks.md) を参照してください。

---

## ベストプラクティス

1. **範囲の狭いパターンを優先する。** `Bash(git *)` が付与するアクセス権は、単独の `Bash` 許可ルールより限定的です。
2. **レイヤーを組み合わせる。** `dontAsk`、範囲の狭い許可ルール、制限の厳しいフック、サンドボックスは、それぞれ独立して制限します。
3. **見慣れない取得元のプロジェクト設定を確認する。** `.grok/config.toml` と `.claude/settings.json` にあるプロジェクトの権限ルールは、`allow` ルールを含め、別途信頼確認を行わずに適用されます。見慣れないチェックアウトで作業する前に、それらのルールとプロジェクトフックを確認してください（[10-hooks.md](10-hooks.md) のセキュリティに関する注意事項を参照）。
4. **ポリシーをテストする。** `defaultMode: "dontAsk"` を設定した状態（または `PreToolUse` フックをインストールした状態）で、代表的なコマンドを実行し、何がブロックされるか確認してください。
5. **読み取り専用コマンド一覧は利便性のための機能であり、セキュリティ境界ではないと考える。**

---

## 関連項目

- [10-hooks.md](10-hooks.md) — フック作成ガイド
- [14-headless-mode.md](14-headless-mode.md) — 権限関連を含むヘッドレス用フラグ
- [18-sandbox.md](18-sandbox.md) — OS レベルの分離プロファイル
- [05-configuration.md](05-configuration.md) — ネイティブの `config.toml` 構造
