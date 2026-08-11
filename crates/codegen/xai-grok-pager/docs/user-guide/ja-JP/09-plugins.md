<a id="plugins"></a>

# プラグイン

プラグインは、スキル、スラッシュコマンド、エージェント、フック、MCP サーバー設定、LSP サーバー設定を、1 つのインストール可能な単位にまとめたものです。

---

<a id="what-a-plugin-contains"></a>

## プラグインの構成要素

プラグインは、次のコンポーネントを任意に組み合わせて格納するディレクトリです。

- **スキル** -- SKILL.md ファイルを格納する `skills/` ディレクトリ
- **スラッシュコマンド** -- コマンドファイルを格納する `commands/` ディレクトリ
- **エージェント** -- エージェント定義を格納する `agents/` ディレクトリ
- **フック** -- ライフサイクルフックを定義する `hooks/hooks.json` ファイル。プラグインフックには `GROK_PLUGIN_ROOT` と `GROK_PLUGIN_DATA` も渡されます（フックに渡されるすべての環境変数については、[フックガイド](10-hooks.md)を参照してください）。
- **MCP サーバー** -- サーバー設定を格納する `.mcp.json` ファイル
- **LSP サーバー** -- 言語サーバー設定を格納する `.lsp.json` ファイル

プラグインに `plugin.json` マニフェストが含まれている場合は、マニフェストでパスを上書きしたり、メタデータを追加したりできます。それ以外の場合、コンポーネントは規約に従ったディレクトリから読み込まれます。マニフェストは省略可能です。マニフェストがなくても、Grok は上記のコンポーネントを標準ディレクトリから検出します。

たとえば、`team-tools` プラグインには、デプロイスキル、コードレビューエージェント、pre-commit フック、Linear MCP サーバーを含めることができます。これらを 1 回の操作でまとめてインストールできます。

<a id="environment-variables-in-plugin-hooks"></a>

## プラグインフックの環境変数

プラグインフックには、すべてのフックに設定される標準の環境変数に加え、次の 2 つの環境変数が渡されます。

| 変数 | 説明 |
|------|------|
| `GROK_PLUGIN_ROOT` | インストールされたプラグインディレクトリの絶対パス。 |
| `GROK_PLUGIN_DATA` | プラグインの状態、キャッシュ、ログを保存する、書き込み可能なデータディレクトリの絶対パス。 |

Grok はこれらの値を設定し、フック JSON の `env` マップで同じキーに宣言した値を上書きします。（互換性のため、Grok は別名の `CLAUDE_PLUGIN_ROOT` と `CLAUDE_PLUGIN_DATA` も設定します。）フックに渡されるすべての環境変数については、[フックガイド](10-hooks.md)を参照してください。

---

<a id="plugin-locations"></a>

## プラグインの場所

Grok は次の場所から、優先順位に従ってプラグインを検出します。

| 場所 | スコープ | 信頼 |
|------|----------|------|
| `_meta.pluginDirs`（`session/new` / `session/load`） | セッション -- そのセッションでのみ読み込まれる | 自動的に信頼される |
| `--plugin-dir`（CLI フラグ、`grok agent`） | プロセス -- そのエージェントプロセスでのみ読み込まれる | 自動的に信頼される |
| `.grok/plugins/` | プロジェクト -- バージョン管理を通じてチームと共有される | 信頼が必要 |
| `~/.grok/plugins/` | ユーザー -- すべてのプロジェクトで使う個人用プラグイン | 自動的に信頼される |
| `[plugins].paths`（設定） | `config.toml` に追加したカスタムディレクトリ | 場所による |

互換性のため、Grok は対応する `.claude/plugins/` も読み込みます。同じ名前のプラグインが複数ある場合は、優先順位が高い場所のプラグインが使用されます。

Agent SDK は、セッション単位のプラグインを `GrokOptions.plugins` から読み込みます。この値は `session/new` と `session/load` で `_meta.pluginDirs` として渡されます。呼び出し元がディレクトリを制御するため、これらのプラグインは常に信頼され、確認なしでフックと MCP サーバーが有効になります。また、セッション終了後には保持されません。`--plugin-dir` フラグは、CLI から直接使用する場合のプロセス全体に対する同等の指定です（繰り返し指定可能: `grok agent --no-leader --plugin-dir A --plugin-dir B stdio`）。これは専用のエージェントプロセスにのみ適用され、leader mode では無視されます（共有 leader が独自にプラグインを検出します）。

---

<a id="manage-plugins-in-the-tui"></a>

## TUI でプラグインを管理する

<a id="open-the-modal"></a>

### モーダルを開く

| 操作 | 開くタブ |
|------|----------|
| `Ctrl+L`（任意のペインから。**VS Code 系以外**） | Plugins タブ |
| `/plugins`（任意のターミナル。**VS Code 系では必須**） | Plugins タブ |

モーダルには、**Hooks**、**Plugins**、**Marketplace**、**Skills**、**MCP Servers** の 5 つのタブがあります。`Tab`（次へ）または `Shift+Tab`（前へ）でタブを切り替えます。`/hooks`、`/marketplace`、`/skills`、`/mcps` の各コマンドを実行すると、対応するタブでモーダルが開きます。

<a id="plugins-tab"></a>

### Plugins タブ

`Enter` を押すとプラグインの行が展開され、詳細が表示されます。

- **名前**と**バージョン**
- **スコープ** -- `cli`、`project`、`user`、`custom path`、またはマーケットプレイスの取得元名
- **スキル** -- 名前または件数
- **エージェント** -- 名前または件数
- **フック** -- 件数
- **MCP サーバー** -- 件数（プラグインが信頼されていない場合は `blocked`）
- **説明**と**パス**

Plugins タブでは、次のキーを使用します。

| キー | 操作 |
|------|------|
| `r` | すべてのプラグインを再読み込みする |
| `a` | `owner/repo`、URL、またはローカルパスからプラグインを追加する |
| `Space` | 選択したプラグインを有効または無効にする |
| `x` | 選択したプラグインをアンインストールする |
| `f` | 状態（すべて、有効、無効）で絞り込む |
| `Enter` | プラグインの詳細を展開または折りたたむ |
| `/` | 名前でプラグインを検索する |

<a id="marketplace-tab"></a>

### Marketplace タブ

設定済みのマーケットプレイスの取得元からプラグインを探してインストールできます。

Marketplace タブでは、次のキーを使用します。

| キー | 操作 |
|------|------|
| `i` | 選択したプラグインをインストールする |
| `d` | 選択したプラグインをアンインストールする |
| `a` | マーケットプレイスの取得元を追加する |
| `x` | 選択した取得元とそのプラグインを削除する |
| `r` | マーケットプレイスの取得元を更新する |
| `u` | 選択したマーケットプレイスプラグインを更新する |
| `Enter` | 取得元またはプラグインを展開または折りたたむ |
| `/` | 名前でプラグインを検索する |

一覧の行に表示されるコンポーネントの要約と、展開ビューに表示されるカテゴリ別のコンポーネント詳細は、`plugin-index.json` カタログを公開しているマーケットプレイスでのみ表示されます。

---

<a id="cli-commands"></a>

## CLI コマンド

対話型セッションを開始せずにプラグインを管理できます。

<a id="plugin-commands"></a>

### プラグインコマンド

```bash
grok plugin list [--json] [--available]   # インストール済みプラグインを一覧表示（--available には --json が必要）
grok plugin install <source> --trust      # Git URL、GitHub 短縮形式（user/repo）、またはローカルパス
grok plugin uninstall <name> [--confirm] [--keep-data]   # 別名: rm、remove
grok plugin update [<name>]               # 名前を省略すると、すべてのプラグインを更新
grok plugin enable <name>
grok plugin disable <name>
grok plugin details <name>                # プラグインのコンポーネント一覧を表示
grok plugin validate [<path>]             # plugin.json を検証（デフォルト: 現在のディレクトリ）
grok plugin tag [<path>] [--push] [--force] [--dry-run]   # マニフェストのバージョンからリリースタグを作成
```

`--trust` を付けずに `grok plugin install <source>` を実行すると、Grok は取得元を表示し、インストールによってプラグインのフック、MCP サーバー、スキルが有効になることを警告して、インストールせずに停止します。インストールするには `--trust` を追加してください。

`<source>` 引数には、次の形式を指定できます。

- `user/repo` -- GitHub 短縮形式
- `user/repo@v1.0` -- ref を固定
- `user/repo#subdir` -- リポジトリ内のサブディレクトリ
- `https://github.com/user/repo.git` -- 完全な URL
- `git@github.com:user/repo.git` -- SSH
- `./local-dir` または `/absolute/path` -- ローカルディレクトリ

<a id="marketplace-commands"></a>

### マーケットプレイスコマンド

```bash
grok plugin marketplace list [--json]
grok plugin marketplace add <url>         # Git URL、GitHub 短縮形式（user/repo）、またはローカルパス
grok plugin marketplace remove <url>      # 設定済み取得元の Git URL またはローカルパス
grok plugin marketplace update [<name>]   # 名前を省略すると、すべての取得元を更新
```

<a id="example-set-up-a-team-marketplace"></a>

### 例: チーム用マーケットプレイスを設定する

```bash
grok plugin marketplace add my-org/team-plugins
grok plugin marketplace list
grok plugin install my-org/team-plugins --trust
grok plugin list
grok plugin update
```

---

<a id="slash-commands"></a>

## スラッシュコマンド

対話型セッションでは、次のコマンドで特定のタブを開きます。引数は指定できません。プラグインの管理には、モーダルまたは `grok plugin` CLI を使用してください。

| コマンド | 開くタブ |
|----------|----------|
| `/plugins` | Plugins タブ |
| `/hooks` | Hooks タブ |
| `/marketplace` | Marketplace タブ |
| `/skills` | Skills タブ |
| `/mcps` | MCP Servers タブ |

---

<a id="configuration"></a>

## 設定

`~/.grok/config.toml` で、プラグインディレクトリとプラグインごとの状態を設定します。

```toml
[plugins]
paths = ["~/my-plugins/custom-tools"]        # 追加のプラグインディレクトリ
disabled = ["user/a1b2c3d4/noisy-plugin"]    # 読み込みをスキップするプラグイン ID または名前
enabled = ["project/9f8e7d6c/team-tools"]    # 強制的に有効化するプラグイン ID または名前
```

プラグインを `disabled` に指定すると、検出はされますが、そのコンポーネントは読み込まれません。プラグインを有効にするには `enabled` に指定します。プラグインは、CLI で上書きするか明示的な設定パスで有効にしない限り、デフォルトで無効です。有効にするにはここへ追加してください。各エントリには、プレーンなプラグイン名（`grok plugin list` に表示される名前）または `<scope>/<hash>/<name>` 形式の完全なプラグイン ID を指定します。

<a id="hide-the-plugins-ui"></a>

### プラグイン UI を非表示にする

フックとプラグインの UI（`/hooks` コマンド、`/plugins` コマンド、スクロールバックの注釈）を非表示にするには、`~/.grok/pager.toml` に次の設定を追加します。

```toml
disable_plugins = true
```

---

<a id="marketplace-sources"></a>

## マーケットプレイスの取得元

git またはローカルのマーケットプレイス取得元を追加すると、プラグインを検出してインストールできます。

<a id="in-configtoml"></a>

### config.toml で設定する

各取得元には `name` と、`git` URL（任意で `branch` も指定可能）またはローカルの `path` のいずれかが必要です。

```toml
[[marketplace.sources]]
name = "My Team Plugins"
git = "https://github.com/my-org/plugins.git"

[[marketplace.sources]]
name = "Local Dev"
path = "~/dev/my-plugins"
```

<a id="in-settingsjson"></a>

### settings.json で設定する

名前をキーとして、`extraKnownMarketplaces` の下に取得元を追加します。各エントリの `source` には、`git`（`url` を指定）、`github`（`repo` を指定）、`local`（`path` を指定）のいずれかを指定します。

```json
{
  "extraKnownMarketplaces": {
    "my-marketplace": {
      "source": { "source": "git", "url": "git@github.com:my-org/plugins.git" }
    }
  }
}
```

このファイルは `~/.grok/settings.json` または `~/.claude/settings.json` に配置します。

---

<a id="trust-model"></a>

## 信頼モデル

プラグインを有効にすると、そのスキル、スラッシュコマンド、エージェントが読み込まれます。信頼は有効化とは別に管理され、プラグインのコードを実行できるかどうかを制御します。有効なプラグインでも、信頼するまではフック、MCP サーバー、LSP サーバーが無効のままです。これにより、信頼されていないリポジトリがマシン上でコードを実行するのを防ぎます。

Grok は `~/.grok/plugins/` のプラグインを自動的に信頼します。`.grok/plugins/` のプロジェクトプラグインには、明示的な信頼が必要です。プラグインを信頼するには、`--trust` を付けてインストールします。

```bash
grok plugin install <source> --trust
```

---

<a id="inspect-plugins"></a>

## プラグインを調査する

検出されたすべてのプラグインとその提供内容を確認するには、`grok inspect` を実行します。

```bash
grok inspect          # プラグインと、そのスキル、エージェント、フック、MCP サーバーを表示
grok inspect --json   # 機械可読 JSON を出力
```

プラグインが提供するコンポーネントは、各セクション（Skills、Agents、MCP Servers など）に `plugin: <name>` ラベル付きで表示されるため、各コンポーネントの取得元を確認できます。

---

<a id="general-keyboard-shortcuts"></a>

## 共通キーボードショートカット

次のキーは、モーダルのすべてのタブで使用できます。

| キー | 操作 |
|------|------|
| `Tab` | 次のタブ |
| `Shift+Tab` | 前のタブ |
| `j` / 下矢印 | 選択を下へ移動 |
| `k` / 上矢印 | 選択を上へ移動 |
| `Enter` | 選択した項目を展開または折りたたむ |
| `/` | 現在のタブを名前で検索する |
| `Esc` | 検索をクリアする、またはモーダルを閉じる |

プラグインのアンインストールなど、一部の操作では確認を求められます。確定するには `y`、キャンセルするには `Esc` を押します。
