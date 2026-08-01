<a id="slash-commands"></a>

# スラッシュコマンド

プロンプトに `/` と入力するとコマンドを利用できます。各コマンドは即座にアクションを実行し、入力中に自動補完されます。

スラッシュコマンドには 2 つの提供元があります。

- **Shell builtins** -- エージェントバックエンド（xai-grok-shell）が処理
- **Pager builtins** -- TUI フロントエンド（xai-grok-pager）が処理

どちらも自動補完メニューに表示されます。SKILL.md ファイルからインストールしたスキルもスラッシュコマンドとして表示されます。

---

<a id="session-management"></a>

## セッション管理

### `/new`

現在の会話を消去して、新しいセッションを開始します。

```
/new
```

エイリアス: `/clear`

### `/resume`

セッション選択画面を開き、ディスクから以前のセッションを読み込みます。

```
/resume
```

### `/compact [context]`

会話履歴を圧縮して、コンテキストウィンドウの空きを確保します。保持する内容を任意で指定できます。

```
/compact
/compact keep the auth implementation details
```

コンテキストウィンドウがいっぱいになると、Grok は使用率 85% で自動的に圧縮します（config.toml の `[session] auto_compact_threshold_percent` で設定可能）。

### `/context`

コンテキストウィンドウの使用状況とセッション統計を表示します。内訳はカテゴリ別（システムプロンプト、メッセージ、推論／オーバーヘッド、空き）に示され、ツール定義、スキル一覧、MCP サーバーの通知についても、推定トークンコストとともに参考情報として表示されます。

```
/context
```

### `/session-info`

モデル、ターン数、コンテキスト使用量などのセッション情報を表示します。

```
/session-info
```

### `/fork`

現在までの履歴を保持し、現在のセッションから新しいエージェントへ分岐します。

```
/fork
```

### `/rewind`

会話を以前のターンまで巻き戻し、それ以降の内容をすべて破棄します。

```
/rewind
```

### `/copy`

最新の応答をクリップボードへコピーします。数値を渡すと、最新から N 番目の応答をコピーします。

```
/copy
/copy 2
```

### `/export`

現在の会話をファイルまたはクリップボードへエクスポートします。

```
/export
```

### `/quit`

アプリケーションを終了します。

```
/quit
```

エイリアス: `/exit`

### `/home`

現在のセッションを終了し、ウェルカム画面へ戻ります。

```
/home
```

エイリアス: `/welcome`

### `/rename`

現在のセッション名を変更します。

```
/rename new session title
```

エイリアス: `/title`

---

<a id="model-and-mode"></a>

## モデルとモード

### `/model <name>`

別のモデルへ切り替えます。モデル ID または表示名を指定できます（大文字と小文字は区別しません）。推論モデルでは、2 番目の引数として effort レベルも指定できます。

```
/model grok-build
/model Grok Build
/model Reasoning X high
```

エイリアス: `/m`

### `/effort <level>`

モデルを選び直さずに、**現在の**モデルの推論 effort を設定します。レベルは `low`、`medium`、`high`、`xhigh` です。アクティブなモデルが推論 effort に対応している場合のみ機能します。

```
/effort high
/effort low
```

### `/always-approve` と `/auto`

どちらも権限モードを切り替える**トグル**として動作します。両方とも補完メニューに残り、有効なモードをもう一度実行すると無効になります。

| コマンド | 無効な場合 | すでに有効な場合 |
|---|---|---|
| `/always-approve` | すべての権限確認を省略 | 確認ありに戻す |
| `/auto` | 分類器が安全なツールを承認（危険なツールでは確認が表示される場合あり） | 確認ありに戻す |

一方のモードが有効なときにもう一方のコマンドを実行すると、モードが**切り替わります**（たとえば always-approve が有効なときに `/auto` を実行すると auto に切り替わります）。

`/auto` は、自動権限モード機能が有効な場合にのみ表示されます。`Shift+Tab`（順番に切り替え）、`Ctrl+O`、または `/settings` でもモードを変更できます。

```
/always-approve
/auto
```

### `/multiline`

複数行入力モードを切り替えます。有効にすると、`Enter` で改行を挿入し、`Shift+Enter`（または `Alt+Enter`）でメッセージを送信します。ターンの途中では、空の入力欄で修飾キーなしの `Enter` を押すと、キューの先頭にあるフォローアップを引き続き即時送信します。

```
/multiline
```

エイリアス: `/ml`

### `/history`

プロンプト履歴検索を開きます。このセッションのプロンプトを新しい順にあいまい検索できます。入力して絞り込み、`Enter` または `Tab` を押すと、一致したプロンプトが入力欄へ戻ります。

すばやく呼び出すには、空のプロンプトで `↑` を押します。最新のプロンプトが入力欄に入った状態でパネルが開き、`↑`／`↓` で履歴を移動できます（各項目が入力欄に入ります）。最新の項目で `↓` を押すとパネルが閉じ、入力すると呼び出したプロンプトをその場で編集できます。

```
/history
```

### `/compact-mode`

コンパクト表示モードを切り替えます。パディングと視覚的な間隔を減らし、表示密度を高めます。

```
/compact-mode
```

### `/vim-mode`

Vim 形式のスクロールバック用キーバインド（j/k、h/l、g/G、y/Y、…）を切り替えます。無効な場合（デフォルト）、スクロールバックで修飾キーなしの文字キーや `Shift+letter` を押すと、プロンプトにフォーカスしてその文字を入力します。設定は `config.toml` の `[ui].vim_mode` に保存されます。

```
/vim-mode
```

### `/minimal` と `/fullscreen`

現在のセッションを別の描画モードで開き直します。`/minimal`（fullscreen で表示）は、実験的なスクロールバックネイティブモードへ切り替えます。`/fullscreen`（minimal で表示、エイリアスは `/full`）は、標準の代替画面 TUI へ戻します。どちらも同じ会話で pager を再起動し、選択は**保持**されます。選択内容は `config.toml` の `[ui].screen_mode` に保存されるため、以後、通常どおり `grok` を起動すると最後に使ったモードで開きます。CLI フラグの `--minimal`／`--fullscreen` も、起動時に同じ切り替えを行います。

```
/minimal
/fullscreen
```

### `/plan`

プランモードに入ります。

```
/plan [description]
```

### `/view-plan`

現在保存されているプランのプレビューを開きます。エイリアス: `/show-plan`、`/plan-view`。

```
/view-plan
```

---

<a id="memory"></a>

## メモリ

`/flush`、`/dream`、`/memory` コマンドを使うには、`--experimental-memory` または `GROK_MEMORY=1` が必要です。`/remember` は常に利用できます。

### `/memory`

保存済みメモリを閲覧、表示、管理します。`on` または `off` を渡すと、メモリを有効または無効にできます。

```
/memory
/memory off
```

エイリアス: `/mem`

### `/flush`

現在のセッションの知識をすぐにメモリへ保存します。セッションで最も重要な内容を LLM が要約します。

```
/flush
```

圧縮前に重要なコンテキストを保持したい場合や、セッション中の任意の時点で使います。

### `/dream`

メモリの統合を実行し、セッションログを整理されたトピックへまとめます。

```
/dream
```

### `/remember`

自動要約を待たずに、メモをすぐにメモリへ保存します。

```
/remember the staging deploy uses the eu-west cluster
```

---

<a id="hooks-and-plugins"></a>

## フックとプラグイン

`/hooks`、`/plugins`、`/marketplace`、`/skills` コマンドは、同じ拡張機能モーダルをそれぞれ別のタブで開きます。

### `/hooks`

拡張機能モーダルの Hooks タブを開きます。モーダルでは、読み込まれたフックの表示、カスタムフックの追加と削除、個別の有効化と無効化ができます。モーダルでプロジェクトの信頼が付与されることはありません。信頼モデルについては [10-hooks.md](10-hooks.md) を参照してください。

```
/hooks
```

**注:** shell は個別の `/hooks-list`、`/hooks-trust`、`/hooks-add`、`/hooks-remove`、`/hooks-untrust` コマンドを公開します。TUI pager では、これらが `/hooks` モーダルに統合されています。

### `/plugins`

拡張機能モーダルの Plugins タブを開きます。モーダルでは、インストール済みプラグインの表示、マーケットプレイスからの新規インストール、信頼の管理ができます。

```
/plugins
```

shell はサブコマンド（`/plugins list`、`/plugins install <source>`、`/plugins uninstall <name>`、`/plugins update`）にも対応しています。TUI の `/plugins` モーダルでは、同じ機能を視覚的なインターフェースで利用できます。

### `/marketplace`

拡張機能モーダルの Marketplace タブを開き、プラグインを閲覧、インストールします。

```
/marketplace
```

### `/skills`

拡張機能モーダルの Skills タブを開き、インストール済みスキルを表示します。

```
/skills
```

---

<a id="media-generation"></a>

## メディア生成

### `/imagine <description>`

テキストの説明から画像を生成します。

```
/imagine a golden sunset over a calm ocean with silhouetted palm trees
```

### `/imagine-video <description>`

画像またはテキストの説明から動画を生成します。ショットを計画し、素材画像を生成して、`image_to_video` でアニメーション化します。

```
/imagine-video a cat playing piano in a jazz club
```

---

<a id="scheduling"></a>

## スケジュール

### `/loop [interval] <prompt>`

一定間隔でプロンプトを繰り返し実行します。間隔は `30m`、`1 hour`、`every 2 days` の形式で指定します。省略すると、Grok が入力を求めます。

```
/loop 30m check deploy status
/loop check deploy status every hour
```

間隔の形式: `Ns`（秒、最小 60）、`Nm`（分）、`Nh`（時間）、`Nd`（日）。60 秒未満の間隔は、最小値の 60 秒に引き上げられます。

定期タスクは 7 日後に自動で期限切れになります。`scheduler_delete` でキャンセルできます（ジョブ ID はループ作成時に提供されます）。

---

<a id="other"></a>

## その他

### `/goal`

自律的な目標を設定、管理、確認します。Grok は複数のターンにわたって目標達成に取り組み、進捗を報告します。

```
/goal Migrate the auth module to the new API
/goal status
```

引数: `<objective>`、`status`、`pause`、`resume`、`clear`。**利用条件:** goal 機能が有効で、セッションのツールセットに `update_goal` ツールがある場合にのみ表示されます。

### `/theme`

TUI のカラーテーマを切り替えます。

```
/theme
```

エイリアス: `/t`

### `/feedback [message]`

問題を報告したり、フィードバックを送信したりします。

```
/feedback Something isn't working correctly
```

### `/btw`

現在のタスクを中断せずに、エージェントへ補足を伝えます。

```
/btw also check the error handling
```

### `/mcps`

MCP サーバー管理モーダルを開きます。

```
/mcps
```

### `/terminal-setup`

ターミナル機能の検出結果とセットアップ情報を表示します。これには、カラーレベル、利用可能なテーマ、クリップボード経路、よくある問題（truecolor、tmux clipboard、keyboard protocol）の修正手順が含まれます。

```
/terminal-setup
```

エイリアス: `/terminal-check`、`/terminal-info`

### `/release-notes`

現在のバージョンのリリースノートを表示します。

```
/release-notes
```

エイリアス: `/changelog`

### `/docs`

TUI 内の How-to Guides を閲覧したり、オンラインの Build docs を開いたり、タイトルを指定してガイドへ移動したりします。

```
/docs
/docs web
/docs Getting Started
```

- 引数なしの `/docs`（または `/docs how-to`）は How-to Guides の選択画面を開きます
- `/docs web` はブラウザーで https://docs.x.ai/build/overview を開きます
- `/docs <title>` は指定したガイドを開きます（タイトルの大文字と小文字は区別しません）

エイリアス: `/howto`、`/guides`

### `/import-claude`

Claude 設定のインポートモーダルを開き、`~/.claude` の設定（権限、環境変数、MCP サーバー、フック、パス）を取り込みます。

```
/import-claude
```

---

<a id="agents-and-personas"></a>

## エージェントとペルソナ

### `/config-agents`

エージェントモーダルを開き、エージェント定義の表示と管理、デフォルトエージェントの設定、アクティブなエージェントの切り替えを行います。

```
/config-agents
```

エイリアス: `/agents`

### `/personas`

ペルソナを管理し、作成、編集、削除します。サブエージェントはペルソナを適用して、その動作を調整できます。

```
/personas
```

---

<a id="account-and-billing"></a>

## アカウントと請求

### `/login`

セッションを離れずに、アカウントへログインまたは再認証します。

```
/login
```

### `/logout`

ログアウトしてログイン画面へ戻ります。

```
/logout
```

### `/usage`

クレジット使用量を確認したり、請求を管理したりします。

```
/usage
```

### `/privacy`

プライバシーとデータ保持の状態を表示または切り替えます。

```
/privacy
```

---

<a id="configuration-and-ui"></a>

## 設定と UI

### `/settings`

設定モーダルを開き、設定を対話形式で表示、変更します。

```
/settings
```

エイリアス: `/config`、`/preferences`、`/prefs`

### `/timestamps`

メッセージのタイムスタンプ表示を切り替えます。

```
/timestamps
```

---

<a id="skills-as-slash-commands"></a>

## スラッシュコマンドとしてのスキル

SKILL.md の frontmatter で `user-invocable: true` が設定された有効なスキルは、すべてスラッシュコマンドとして表示されます（`/skills` で無効にしたスキルは表示されません）。たとえば `~/.grok/skills/commit/SKILL.md` にスキルがある場合、次のように呼び出せます。

```
/commit fix typo in README
```

プラグインのスキルもスラッシュコマンドとして表示されます。複数のスコープで同名のスキルがある場合は、修飾形式を使います。

```
/local:commit      # プロジェクトスコープのスキル
/user:commit       # ユーザースコープのスキル
```

組み込みスラッシュコマンドは、同名のスキルより常に優先されます。スキルに "compact" という名前を付けた場合、`/compact` を入力すると組み込みの compact コマンドが実行されますが、`/local:compact` ではスキルが呼び出されます。

---

<a id="autocomplete"></a>

## 自動補完

スラッシュコマンドメニューはあいまい検索に対応しています。`/` の後に入力すると、利用可能なコマンドが絞り込まれます。メニューには次の項目が表示されます。

- コマンド名
- 説明
- 引数のヒント（コマンドが引数を受け付ける場合）
- 提供元（builtin、スキルのスコープ、プラグイン名）

自動補完メニューでコマンドを選択するには、`Tab` または `Enter` を押します。
