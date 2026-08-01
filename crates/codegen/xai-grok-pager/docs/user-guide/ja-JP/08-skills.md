<a id="skills"></a>

# スキル

スキルは、タスク固有の指示で Grok を拡張する、再利用可能なプロンプトパッケージです。セッションごとに説明し直す代わりに、繰り返し使う手順を一度だけ定義できます。

---

<a id="what-are-skills"></a>

## スキルとは？

スキルは、`SKILL.md` ファイルを含むディレクトリです。その Markdown 本文で、特定の種類のタスクを処理する方法（手順、規約、ツールの使用パターン）を Grok に指示します。

AGENTS.md に書くには限定的すぎる一方、毎回入力し直すには長すぎる反復可能な手順には、スキルを使用します。Grok は、現在のタスクに該当する場合にのみスキルを有効化します。

---

<a id="skill-locations"></a>

## スキルの場所

Grok は、次のディレクトリから優先順位に従ってスキルを検出します。

| 場所 | スコープ | 優先度 | 注記 |
|----------|-------|----------|-------|
| `./.grok/skills/`, `./.grok/commands/` | ローカル（CWD） | 最高 | 現在のディレクトリのスキル／従来形式のコマンド Markdown |
| `<repo_root>/.grok/skills/`, `…/commands/` | リポジトリ | 中 | リポジトリ全体で共有 |
| `~/.grok/skills/`, `~/.grok/commands/` | ユーザー | 最低 | すべてのプロジェクトで使う個人用スキル |
| `~/.claude/skills/`, `~/.claude/commands/` | ユーザー | 最低 | Claude Code 互換（設定可能） |
| `./.claude/skills/`, `./.claude/commands/` | ローカル／リポジトリ | 高 | プロジェクトの Claude スキルと従来形式のカスタムスラッシュコマンド |
| `~/.cursor/skills/` | ユーザー | 最低 | Cursor 互換（設定可能） |
| `./.cursor/skills/` | ローカル／リポジトリ | 高 | プロジェクトの Cursor スキル（Cursor 互換スキルが有効な場合） |

Grok は名前でスキルの重複を排除し、優先度の高い場所が低い場所を上書きします。また、各階層で（`.grok/` と併せて）`.agents/skills/` と `commands/` をスキャンし、作業ディレクトリからリポジトリルートまでのすべてのディレクトリをたどります。

`commands/` ディレクトリ直下の `*.md` ファイルは、ユーザーが呼び出せるスラッシュコマンドになります（ファイル名の拡張子を除いた部分がコマンド名）。これは Claude Code の従来形式のカスタムコマンド配置と同じです。

スキルとコマンドの検出では、`.gitignore` は使用されません。既知のスキルルート（`.grok/`、`.agents/`、`.claude/`、`.cursor/`）配下のパスは、ディスク上に存在すれば常に読み込まれます。チームでは、`.claude/**` をローカル専用設定として無視しつつ、`/frontend` のようなプロジェクトコマンドが動作することを期待する場合がよくあります。スキルを非表示にするには、リポジトリの無視ルールではなく、設定の `[skills] ignore` を使用してください。

Grok はデフォルトで Claude と Cursor のスキルディレクトリをスキャンします。特定ベンダーのスキャンを停止するには、`~/.grok/config.toml` の `[compat.cursor]` または `[compat.claude]` で、その `skills` セルを `false` に設定するか、環境変数 `GROK_CURSOR_SKILLS_ENABLED` または `GROK_CLAUDE_SKILLS_ENABLED` を `false` に設定します。詳細については、[設定](05-configuration.md#harness-compatibility)を参照してください。これらの設定にかかわらず、Grok はベンダー提供の既知のデフォルトスキル（Cursor の `shell`、`canvas`、`statusline` など）を常に除外します。

<a id="additional-skill-directories"></a>

### 追加のスキルディレクトリ

`~/.grok/config.toml` の `[skills]` を使用して、ディレクトリの追加、パスの除外、個別スキルの無効化ができます。

```toml
[skills]
paths = ["~/my-team-skills"]          # スキャンする追加ディレクトリ
ignore = ["~/my-team-skills/wip"]     # 除外するパス（完全に非表示）
disabled = ["wip-skill"]              # 一覧には残すが無効にするスキル名
```

`paths` の各エントリには、`SKILL.md` ファイル、または Grok が再帰的にたどるディレクトリを指定します。`ignore` はスキルを完全に非表示にし、`disabled` は一覧に残したまま、システムプロンプトと呼び出しの対象から除外します。`paths` と `ignore` にはファイルシステムパスを指定し、`~` 展開を利用できます。`disabled` にはスキル名を指定します。

---

<a id="creating-a-skill"></a>

## スキルを作成する

<a id="directory-structure"></a>

### ディレクトリ構成

各スキルは、`SKILL.md` ファイルを持つ個別のディレクトリに配置します。

```
~/.grok/skills/
  commit/
    SKILL.md
  review-pr/
    SKILL.md
  deploy/
    SKILL.md
```

<a id="skillmd-format"></a>

### SKILL.md の形式

スキルファイルは、YAML frontmatter と、それに続く Markdown の指示で構成されます。

```markdown
---
name: commit
description: conventional commit 規約に従って、適切な形式の git commit を作成する。ユーザーが変更の commit を求めた場合、または /commit を依頼した場合に使用する。
---

# Git Commit スキル

ステージされた変更を確認し、明確で規約に沿ったメッセージの commit を作成する。

## 手順

1. `git diff --staged` を実行して変更を確認する
2. 変更内容と変更理由を要約する
3. conventional commits 形式に従って commit メッセージを作成する
4. そのメッセージで `git commit -m "..."` を実行する
```

<a id="core-frontmatter-fields"></a>

### 主要な frontmatter フィールド

| フィールド | 説明 |
|-------|-------------|
| `name` | スキル識別子。小文字、数字、ハイフンを使用し、最大 64 文字にします。Grok はスペースとアンダースコアをハイフンに正規化します。`name` を省略した場合、スキルのディレクトリ名が使用されます。 |
| `description` | スキルの機能と使用する状況。Grok はこれを読み、スキルを呼び出すかどうかを判断します。省略した場合は、本文の最初の段落が使用されます。 |

具体的な `description` を記述してください。これは、Grok がスキルを自動的に呼び出すタイミングを決定します。トリガーとなるフレーズとユースケースを明記します。

<a id="optional-frontmatter-fields"></a>

### 任意の frontmatter フィールド

複数単語の frontmatter キーには kebab-case を使用します（`model` のような単一単語のキーは、そのまま記述します）。

| フィールド | 説明 |
|-------|-------------|
| `when-to-use` | 自動呼び出しのトリガーとなるフレーズ。`description` とは別に保持されます。 |
| `allowed-tools` | スキルが使用するツール。YAML リスト、またはカンマ区切りかスペース区切りの文字列で指定します。 |
| `argument-hint` | スラッシュコマンドのオートコンプリートに表示されるヒントテキスト（例: `commit message`）。 |
| `user-invocable` | スキルをスラッシュコマンドとして実行できるかどうか。デフォルトは `true` です。スラッシュコマンドから非表示にするには `false` に設定します。（モデルによるスキルの呼び出しを停止するには、代わりに `disable-model-invocation` を設定します。） |
| `disable-model-invocation` | `true` の場合、ユーザーのスラッシュコマンドでのみスキルが実行され、モデルは自動的に呼び出せません。デフォルトは `false` です。 |
| `model` | スキル実行時のモデル上書き。 |
| `effort` | 推論 effort の上書き。 |
| `license` | ライセンス識別子（例: `Apache-2.0`）。 |
| `compatibility` | 環境要件（例: `Requires git, docker, jq`）。 |
| `metadata` | 任意の文字列キーと値のペア。Grok は表示用に `metadata.author` と `metadata.short-description` を使用します。 |

---

<a id="creating-skills-with-create-skill"></a>

## /create-skill でスキルを作成する

`/create-skill` コマンドは、新しいスキルの作成を対話形式で案内します。Grok が要望を確認し、ファイルの下書きを作成してディスクへ書き込みます。

<a id="how-it-works"></a>

### 仕組み

`/create-skill` を実行すると、Grok は次の処理を行います。

1. **要件を収集する。** Grok はスキル名、保存先のスコープ、記録したいワークフローの説明を確認します。名前には小文字、数字、ハイフンを使用し、2～64 文字で、先頭と末尾を英字または数字にします。

2. **description の下書きを作成する。** Grok は、スキルの機能、トリガーとなるフレーズ、スラッシュコマンド名を記載した `description` を作成します。続行する前に、下書きを承認または編集できます。

3. **スキルディレクトリを作成する。** Grok は `<scope>/.grok/skills/<name>/` ディレクトリを作成し、スキルで必要な場合は `scripts/` または `references/` サブディレクトリも作成します。

4. **SKILL.md を書き込む。** Grok は frontmatter（`name` と `description`）、指示の Markdown 本文、補助ファイルを書き込みます。

5. **検証して確認する。** Grok はファイルを読み直し、正しく書き込まれたことを確認して、スキルの実行方法を案内します。

<a id="choosing-a-scope"></a>

### スコープを選択する

Grok は、スキルの保存先を確認します。

- **プロジェクト**（`<repo_root>/.grok/skills/<name>/`）-- このリポジトリ内でのみ利用でき、バージョン管理を通じてチームメンバーと共有できます。Grok は、git リポジトリ内ではこのスコープを推奨します。
- **ユーザー**（`~/.grok/skills/<name>/`）-- すべてのプロジェクトで利用できます。

Grok はディスク上のファイル変更時にスキルを再読み込みするため、新しいスキルは数秒以内にスラッシュメニューへ表示されます。

---

<a id="using-skills"></a>

## スキルを使用する

<a id="run-a-skill-by-name"></a>

### 名前でスキルを実行する

各スキルは、スキル名と同じ名前のスラッシュコマンドになります。名前を入力して実行します。

```
/commit              # 「commit」スキルを実行
/review-pr           # 「review-pr」スキルを実行
```

スキルを実行すると、その指示が会話に読み込まれ、モデルは指示に従って処理します。引数を渡すには、名前の後に入力します。

```
/commit fix the build
```

スキルを参照するには、`/` を入力してスラッシュコマンドメニューを開きます。Grok はすべての組み込みコマンドとスキルを一覧表示し、入力に応じて絞り込みます。代わりにコマンドラインからスキルを一覧表示するには、`grok inspect` を実行します（[スキルの詳細を表示する](#viewing-skill-details)を参照）。

<a id="qualified-names"></a>

### 修飾名

スキル名が別のスキルや組み込みコマンドと競合する場合、Grok はスキルのスコープ（`local:`、`repo:`、`user:`、またはプラグイン名）を接頭辞とする修飾名を提示します。特定のスキルを選ぶには、修飾形式を使用します。

```
/local:commit        # ./.grok/skills/ の「commit」スキル
/user:commit         # ~/.grok/skills/ の「commit」スキル
```

<a id="automatic-invocation"></a>

### 自動呼び出し

Grok は、関連するタスクを認識すると自動的にスキルを呼び出せます。Grok はプロンプトをスキルの `description` および `when-to-use` フィールドと照合するため、どちらにもトリガーとなる状況を記述してください。

たとえば、スキルの description に「ユーザーが変更の commit を求めた場合に使用する」と記載されている場合、「変更を commit して」と依頼すると、そのスキルが自動的に呼び出される可能性があります。明示的なスラッシュコマンドを必須とし、自動呼び出しを防ぐには、frontmatter に `disable-model-invocation: true` を設定します。

---

<a id="viewing-skill-details"></a>

## スキルの詳細を表示する

`grok inspect` を実行すると、Grok が検出したすべてのスキルを、その他の設定と併せて確認できます。

```bash
grok inspect          # 人が読みやすい要約
grok inspect --json   # 機械可読レポート
```

人が読みやすい出力では、Skills セクションに各スキルの名前と取得元が表示されます。取得元は `project`、`user`、`bundled`、`config`（`[skills].paths` のエントリ）、`server`（管理対象ワークスペースのスキルストアから同期されたスキル）、または `plugin: <name>` です。Grok は、`[skills].disabled` で無効化されたスキル、または無効化されたベンダー連携由来のスキルに `[disabled]` タグを付けます。

このレポートには、実際のセッションと同様に `[skills]` 設定が適用されます。`paths` のスキルは一覧に表示され、`ignore` 接頭辞配下のスキルは非表示になり、`disabled` に指定されたスキルは一覧に残りますが `[disabled]` タグが付きます。

`--json` レポートには、各スキルの完全な詳細として、`name`、`description`、`source`（SKILL.md ファイルへのパスを含む）、`userInvocable` フラグが含まれます。

---

<a id="bundled-and-plugin-skills"></a>

## 同梱スキルとプラグインスキル

Grok には組み込みスキルが同梱されており、起動時に `~/.grok/skills/` へ展開されます。これには `/create-skill`、`/help`、`/check-work` などがあります。同梱スキルはユーザースキルと同様に動作し、優先度の高い場所（ローカルまたはリポジトリ）に同名のスキルがある場合は、同梱スキルを上書きします。`grok inspect` は、展開されたコピーに `bundled` と表示するため、自分で作成したスキルと区別できます。（同名のプラグインスキルは上書きせず、修飾形式の `plugin:name` で引き続き利用できます。）

スキルはプラグインから提供される場合もあります。スキルを含むプラグインをインストールすると、ユーザースキルやプロジェクトスキルと並んで表示されます。`grok inspect` は、プラグイン提供の各スキルの取得元を `plugin: <name>` と表示します。

スキルを提供するプラグインのインストールについて詳しくは、[プラグインガイド](09-plugins.md)を参照してください。

---

<a id="best-practices"></a>

## ベストプラクティス

1. **具体的な description を記述する。** description は自動呼び出しを左右します。「git commit を作成する」では曖昧すぎます。「conventional commit 規約に従って、適切な形式の git commit を作成する。ユーザーが変更の commit を求めた場合、または /commit を依頼した場合に使用する。」のように記述すると効果的です。

2. **具体的な手順を含める。** Grok が従うべき明確で順序立った手順を示すと、スキルは最も効果的に機能します。

3. **ツールを名前で参照する。** スキルが特定のツール（`run_terminal_command` や `search_replace` など）に依存する場合は、モデルが使用すべきツールを把握できるよう、名前を明記します。

4. **スキルの焦点を絞る。** 1 つのワークフローにつき 1 つのスキルを作成します。単一の「deploy-and-rollback」スキルより、「deploy」スキルと「rollback」スキルに分ける方が効果的です。

5. **プロジェクトスキルをバージョン管理する。** チーム全体が利用できるように、`.grok/skills/` をリポジトリへ commit します。`~/.grok/skills/` のユーザースキルは個人用であり、共有されません。

6. **実行してテストする。** 自動呼び出しに頼る前に、`/name` を呼び出してスキルが動作することを確認します。
