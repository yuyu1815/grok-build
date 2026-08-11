# はじめに

Grok Build は SpaceXAI が提供するターミナルベースの AI コーディングアシスタントです。TUI（ターミナルユーザーインターフェース）として動作し、コードベースの把握、シェルコマンドの実行、ファイルの編集、ウェブ検索、タスク管理を行います。

フルスクリーン TUI で対話的に使用するほか、スクリプトや CI/CD 向けにヘッドレスで実行したり、Agent Client Protocol（ACP）を介してエディターに統合したりできます。

---

## インストール

最新の安定版をインストールします（macOS、Linux、または Git Bash を使用する Windows）。

```bash
curl -fsSL https://x.ai/cli/install.sh | bash
```

特定のバージョンをインストールするには、次を実行します。

```bash
curl -fsSL https://x.ai/cli/install.sh | bash -s 0.1.42
```

**Windows（PowerShell）** では、ネイティブの PowerShell インストーラーを使用します。

```powershell
irm https://x.ai/cli/install.ps1 | iex
```

特定のバージョンをインストールするには、次を実行します。

```powershell
$env:GROK_VERSION="0.1.42"; irm https://x.ai/cli/install.ps1 | iex
```

PowerShell インストーラーは `%USERPROFILE%\.grok\bin` をユーザーの PATH に自動で追加します。または、上記の bash スクリプトを使い、[Git for Windows](https://gitforwindows.org/)（Git Bash）か MSYS2 からインストールできます。WSL では Linux バイナリが自動的にインストールされます。

インストールを確認します。

```bash
grok --version
```

いつでも最新バージョンに更新できます。

```bash
grok update
```

---

## 初回起動

次のコマンドで Grok を起動します。

```bash
grok
```

初回起動時、Grok は grok.com で認証するためにブラウザーを開きます。サインインすると、認証情報が `~/.grok/auth.json` に保存され、セッションをまたいで維持されます。Grok は認証情報を自動的に更新し、更新できなくなった場合は再度サインインするよう求めます。

API キー認証を使用する場合（CI/CD やブラウザーのない環境など）は、代わりに環境変数 `XAI_API_KEY` を設定します。

```bash
export XAI_API_KEY="xai-..."
grok
```

OIDC、外部認証プロバイダー、デバイスコードフローを含むすべての認証方法については、[認証](02-authentication.md)を参照してください。

---

## 基本操作

認証が完了すると、Grok は主に 2 つの領域からなるフルスクリーン TUI を表示します。

- **スクロールバック** -- プロンプト、Grok の応答、ツール呼び出し、ファイル編集などを表示する会話履歴です。
- **プロンプト** -- メッセージを入力する画面下部の領域です。

メッセージを入力し、`Enter` を押して送信します。Grok は必要に応じてファイルを読み取り、コマンドを実行し、コードを編集します。各ツールの実行内容は、リアルタイムでスクロールバックに表示されます。

`Tab` を押すと、プロンプトとスクロールバックの間でフォーカスを移動できます。ターンの実行中は、`Ctrl+C` でキャンセルできます（未送信の入力がある場合は、先にその入力を消去します）。ターンの実行中に `Esc` を押しても何も起こりません。待機中に 800 ミリ秒以内に `Esc` を 2 回押すと、未送信の入力がある場合は消去し、プロンプトが空で会話メッセージがある場合は巻き戻しを開きます。[キーボードショートカット](03-keyboard-shortcuts.md#escape)を参照してください。スクロールバックにフォーカスがある状態では、矢印キーで項目を選択し、折りたたみ・展開できます。代わりに `j`/`k` で移動し、`h`/`l` で折りたたみ・展開するには、Vim モードを有効にします。

### ファイル参照

プロンプトで `@` を使うとファイルを添付できます。

```
@src/main.rs              # ファイルを添付
@src/main.rs:10-50        # 10～50 行目を添付
@src/                     # ディレクトリを参照
```

`@` 演算子を入力すると、あいまい検索対応のファイルピッカーが開きます。デフォルトでは `.gitignore` に従い、ドットファイルを非表示にします。隠しファイルを検索するには、先頭に `!` を付けます。

```
@!.github                 # 隠しファイルを検索
@!.env                    # .env ファイルを添付
```

### 権限

デフォルトでは、Grok はシェルコマンドの実行やファイルの編集前に許可を求めます。個別に承認するか、常時承認モードに切り替えられます。

- `Ctrl+O` を押して常時承認モードを切り替える
- 起動時に `--yolo` フラグを使用する: `grok --yolo`
- プロンプトに `/always-approve` と入力してモードを切り替える

---

## 主な概念

### セッション

すべての会話は **セッション** です。セッションは `~/.grok/sessions/` に自動保存され、後で再開できます。各セッションには、会話履歴全体、ツール呼び出し、ファイル編集、タスクの状態が記録されます。

- 新しいセッションを開始する: `Ctrl+N` または `/new`
- 以前のセッションを再開する: TUI では `/resume`、CLI では `--resume <ID>`
- 直近のセッションを続行する: `grok -c`

### スクロールバック

スクロールバックはメインの表示領域です。次の内容が表示されます。

- **ユーザープロンプト** -- 固定ヘッダーとして表示されるユーザーのメッセージ
- **エージェントメッセージ** -- Markdown の完全なレンダリングと構文ハイライトに対応した Grok の応答
- **思考ブロック** -- Grok の推論過程（折りたたみ可能）
- **ツール呼び出し** -- ファイル編集（インライン差分付き）、コマンド実行、検索結果など
- **タスクリスト** -- 進捗を追跡する TODO 項目

`Left`/`Right` 矢印キー（Vim モードでは `h`/`l` と `e`）で、選択した項目を折りたたみ・展開できます。Vim モードでは、`y` を押すと内容を、`Y` を押すとメタデータ（実行されたコマンドなど）をコピーできます。`Enter` を押すと、どのモードでもフルスクリーンビューアーで開きます。

### ツール

Grok には次の組み込みツールがあります。

| ツール | 説明 |
|------|-------------|
| `read_file` / `search_replace` | 行単位で正確にファイルを読み取り、編集する |
| `grep` | コードベース全体を正規表現で検索する（ripgrep を使用） |
| `list_dir` | ディレクトリの内容を一覧表示する |
| `run_terminal_command` | シェルコマンドを実行する |
| `web_search` / `web_fetch` | ウェブを検索し、URL の内容を取得する |
| `todo_write` | タスクリストを作成、管理する |
| `spawn_subagent` | 複数のサブエージェントセッションを並列で起動する |
| `memory_search` | セッションをまたいでメモリを検索する |

[MCP サーバー](05-configuration.md#mcp-servers)を使用すると、GitHub やデータベースなどとの連携用ツールを追加できます。

### スラッシュコマンド

プロンプトに `/` を入力するとコマンドを使用できます。プロンプトを一から書かずに、次の操作をすばやく実行できます。

```
/model grok-build                 # モデルを切り替える
/compact                          # 会話履歴を圧縮する
/always-approve                   # 常時承認モードを切り替える
/new                              # 新しいセッションを開始する
```

完全なリファレンスについては、[スラッシュコマンド](04-slash-commands.md)を参照してください。

---

## よく使う起動オプション

```bash
# 対話型 TUI を起動し、最初のターンとして初期プロンプトを送信する
grok "fix the failing auth test and run it"

# 新しい git worktree で初期プロンプトを送信する。プロンプトが worktree 名として解釈されないよう、
# --worktree=<name>（`=` 付き）を使用する。`grok -w "refactor module X"` では、
# "refactor module X" がプロンプトではなく worktree のラベルとして扱われる。
grok --worktree=feat "refactor module X"

# 現在の HEAD ではなく、特定のブランチ（例: main）を worktree のベースにする:
grok -w --ref main "implement feature from main"


# 特定のプロジェクトディレクトリで開始する
grok --cwd ~/projects/my-app

# プロジェクト固有のルールを追加する
grok --rules "Always use TypeScript. Prefer functional components."

# すべてのツール実行を自動承認する
grok --yolo

# 特定のモデルを使用する
grok -m grok-build

# 以前のセッションを再開する
grok --resume <session-id>

# 直近のセッションを続行する
grok -c

# 実験的なスクロールバックネイティブのレンダリングモード。設定は保持され、通常の `grok` でも
# --minimal/--fullscreen（または /minimal//fullscreen）で最後に選択したモードが再び開く。
grok --minimal

# 標準のフルスクリーン TUI に戻す（この設定も保持される）
grok --fullscreen

# ヘッドレスモード（スクリプト向け）
grok -p "Explain this codebase"
```

---

## ヘッドレスモード

スクリプト、CI/CD、自動化向けに Grok を非対話形式で実行します。

```bash
grok -p "Your prompt here"
```

出力形式は次のとおりです。

| 形式 | フラグ | 説明 |
|--------|------|-------------|
| `plain` | （デフォルト） | 人が読みやすいテキスト |
| `json` | `--output-format json` | `text`、`stopReason`、`sessionId`、`requestId` を含む単一の JSON オブジェクト |
| `streaming-json` | `--output-format streaming-json` | リアルタイム処理向けの NDJSON イベントストリーム |

CI/CD での使用例:

```bash
grok -p "Review changes for bugs" --output-format json --yolo | jq -r '.text'
```

---

## プロジェクトルール（AGENTS.md）

リポジトリに `AGENTS.md` ファイルを作成すると、プロジェクトごとの指示を追加できます。Grok はこれらのファイルを読み取り、会話の開始時にその内容をプロジェクト指示メッセージとして挿入します。

```
~/.grok/AGENTS.md           # グローバルルール（すべてのプロジェクトに適用）
<repo-root>/AGENTS.md       # リポジトリレベルのルール
<cwd>/AGENTS.md             # ディレクトリレベルのルール（最優先）
```

より深い階層にあるファイルが優先されます。互換性のため、Grok は `CLAUDE.md` ファイルも読み取ります。

---

## 次に読むもの

| ドキュメント | 学べる内容 |
|----------|-------------------|
| [認証](02-authentication.md) | ブラウザーログイン、API キー、OIDC、外部認証、デバイスコードフロー |
| [キーボードショートカット](03-keyboard-shortcuts.md) | すべてのキーバインドの完全なリファレンス |
| [スラッシュコマンド](04-slash-commands.md) | 使用可能なすべての `/` コマンド |
| [設定](05-configuration.md) | config.toml、pager.toml、環境変数 |
