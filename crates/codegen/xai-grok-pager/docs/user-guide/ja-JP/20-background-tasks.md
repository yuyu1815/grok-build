<a id="background-tasks-and-monitoring"></a>

# バックグラウンドタスクと監視

Grok は、会話を妨げずに長時間実行プロセスを動かせます。このドキュメントでは、バックグラウンドコマンド、`/loop` コマンド、`monitor` ツール、スケジューラーについて説明します。

---

<a id="background-commands"></a>

## バックグラウンドコマンド

コマンドをバックグラウンドで実行するには、`run_terminal_command` ツールで `background: true` を設定します。タスク ID がすぐに返され、`get_command_or_subagent_output` で出力を取得できます。

<a id="how-it-works"></a>

### 仕組み

1. エージェントが `background: true` を指定して `run_terminal_command` を呼び出します。
2. コマンドがバックグラウンドで起動します。
3. エージェントは、後で参照するための `task_id` を受け取ります。
4. コマンドが完了すると、会話に通知が表示されます。

<a id="getting-output"></a>

### 出力の取得

バックグラウンドコマンドやサブエージェントの状態を確認するには、`get_command_or_subagent_output` ツールを使用します。

- `get_command_or_subagent_output(task_id)` — 待機せずに現在の出力と状態を取得
- `get_command_or_subagent_output(task_id, timeout_ms=30000)` — 完了まで指定したミリ秒数を上限として待機

<a id="waiting-for-multiple-tasks"></a>

### 複数タスクの待機

複数のタスクをまとめて待つには、`wait_commands_or_subagents` を使用します。

- `task_ids` — 待機するタスク ID のリスト（最大 20 件）
- `mode` — `wait_any` は最初のタスクが完了した時点で返り、`wait_all` はすべてのタスクが完了するまで待機
- `timeout_ms` — 最大待機時間（ミリ秒、デフォルト: 30 秒）

このツールは、指定したすべてのタスクの状態と出力を返します。

<a id="killing-background-tasks"></a>

### バックグラウンドタスクの終了

実行中のバックグラウンドタスクやサブエージェントを終了するには、`kill_command_or_subagent(task_id)` を使用します。シェルプロセスには SIGTERM、続いて SIGKILL を送信し、サブエージェントには Cancel と Shutdown を送信します。タスクを終了した場合、またはすでに終了していた場合は成功を返します。

<a id="common-use-cases"></a>

### 一般的な用途

- **開発サーバー**: 開発サーバーを起動し、コーディングを続ける
- **テストスイート**: 修正作業中にテストをバックグラウンドで実行する
- **ビルド処理**: ビルドを開始し、後で結果を確認する
- **長時間のコンパイル**: コンパイルを開始し、ほかの作業を続ける

---

<a id="send-a-running-task-to-the-background"></a>

## 実行中のタスクをバックグラウンドへ移す

対話型 TUI では、`Ctrl+G` を押すと、実行中のフォアグラウンドコマンドをバックグラウンドへ移せます。次の場合に使用してください。

- コマンドが予想以上に時間を要している。
- コマンドの実行中に、エージェントへ別の質問をしたい。
- プロセスの開始後に、長時間実行されると分かった。

タスクはそのまま実行され、完了すると通知が届きます。

---

<a id="the-loop-command"></a>

## /loop コマンド

`/loop` は、指定した間隔でプロンプトを繰り返し実行します。タスクのポーリング、定期確認、継続的な監視に役立ちます。

<a id="syntax"></a>

### 構文

```
/loop [interval] <prompt>
```

間隔には次の形式を使用できます。

| 形式 | 例 | 説明 |
| ---- | -- | ---- |
| `Ns` | `60s` | N 秒ごと（最小 60 秒） |
| `Nm` | `5m` | N 分ごと |
| `Nh` | `2h` | N 時間ごと |
| `Nd` | `1d` | N 日ごと |

<a id="examples"></a>

### 例

```
/loop 5m テストスイートが成功するか確認し、失敗があれば報告して
/loop 2h 前回の確認以降の新しいコミットを要約して
/loop 60s localhost:3000 の開発サーバーが応答するか確認して
```

<a id="behavior"></a>

### 動作

- 作成時にプロンプトをすぐ実行し、その後は指定した間隔で繰り返します
- 実行のたびに新しいエージェントターンが作成されます
- 定期タスクは 7 日後に自動で期限切れになります
- 同時に有効にできるスケジュール済みタスクは最大 50 件です

---

<a id="the-monitor-tool"></a>

## monitor ツール

`monitor` ツールは、長時間実行スクリプトのイベントをストリーミングします。出力の各行が会話内の通知になります。`monitor` は `/loop` のストリーミング版です。定期確認には `/loop`、リアルタイムのイベントストリームには `monitor` を使用してください。

<a id="how-it-works-1"></a>

### 仕組み

1. シェルコマンド（`command`）と、各通知に表示する短い `description` を指定します。
2. Grok がコマンドの stdout と stderr を 1 つの出力ファイルに統合します。
3. そのファイルに追加された各行が、会話へ通知されます。
4. コマンドが終了するか、ユーザーが停止するまでモニターが実行されます。

<a id="script-guidelines"></a>

### スクリプトのガイドライン

- **パイプでは必ず `grep --line-buffered` を使用してください。** 使用しないと、パイプのバッファリングによってイベントが数分遅れます。
- **ポーリングループでは一時的な失敗を処理してください**（`curl ... || true`）。1 回のリクエスト失敗でモニターを停止させないでください。
- **対象を絞るフィルターを使用してください。** すべての行がメッセージになるため、生ログをそのままパイプに流さないでください。
- **ポーリング間隔を監視対象に合わせてください。** リモート API ではレート制限を守るため 30 秒以上、ローカルの確認では 0.5～1 秒を目安にします。
- **stdout と stderr の両方がイベントになります。** イベントにしたくない出力は、たとえば `2>/dev/null` を付けてリダイレクトするか、フィルターで除外してください。

<a id="examples-1"></a>

### 例

```bash
# ログファイル内のエラーを監視
tail -f /var/log/app.log | grep --line-buffered "ERROR"

# ディレクトリ内のファイル変更を監視
inotifywait -m --format '%e %f' /watched/dir

# GitHub の新しい PR コメントをポーリング
last=$(date -u +%Y-%m-%dT%H:%M:%SZ)
while true; do
  now=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  gh api "repos/owner/repo/issues/123/comments?since=$last" \
    --jq '.[] | "\(.user.login): \(.body)"'
  last=$now; sleep 30
done
```

<a id="persistent-monitors"></a>

### 永続モニター

セッション中ずっと実行するモニターには、`persistent: true` を設定します。

- PR の監視
- ログの追跡
- CI 状態の監視

永続モニターを停止するには、`kill_command_or_subagent(task_id)` を使用します。

<a id="volume-control"></a>

### 通知量の制御

モニターが生成するイベントが多すぎる場合、Grok はモニターを自動停止します。その場合は、フィルター条件を厳しくして再起動してください。`grep --line-buffered`、`awk`、または必要なイベントだけを出力するラッパースクリプトを推奨します。

---

<a id="the-scheduler"></a>

## スケジューラー

スケジューラーは、定期タスクを作成するための低レベル API です。`/loop` はスケジューラーを使いやすくしたラッパーです。

<a id="scheduler_create"></a>

### scheduler_create

スケジュール済みタスクを作成します。

| パラメーター | 説明 |
| ------------ | ---- |
| `interval` | 実行間隔: `"5m"`、`"2h"`、`"1d"`、`"60s"` |
| `prompt` | 各実行時に処理するプロンプトのテキスト |
| `fire_immediately` | 指定間隔での実行に加え、作成時にも実行（デフォルト: `false`） |
| `recurring` | 繰り返し実行（デフォルト: `true`）、または 1 回だけ実行（`false`） |
| `durable` | セッションをまたいで維持（デフォルト: `false`） |

<a id="scheduler_list"></a>

### scheduler_list

有効なすべてのスケジュール済みタスクについて、ID、プロンプト、実行間隔、次回実行時刻を一覧表示します。

<a id="scheduler_delete"></a>

### scheduler_delete

ID を指定してスケジュール済みタスクをキャンセルします。タスクが見つかり、削除された場合は成功を返します。

---

<a id="the-tasks-pane"></a>

## タスクペイン

対話型 TUI では、`Ctrl+B` を押すとタスクペインの表示を切り替えられます。このペインでは、次の項目を 1 つの画面で確認できます。

- 実行中のサブエージェントと進捗
- 有効なバックグラウンドタスクと状態
- モニターと `/loop` のタスク（それぞれにリアルタイムの行数バッジを表示）
- 各項目のタスク ID

代わりにプロンプトキューの表示を切り替えるには、`Ctrl+;` を押します。

---

<a id="use-cases-and-patterns"></a>

## ユースケースとパターン

<a id="dev-server--coding"></a>

### 開発サーバー + コーディング

開発サーバーをバックグラウンドで起動し、コーディングを続けます。

```
開発サーバーを `npm run dev` でバックグラウンド起動してから、ログインフォームを実装して。
```

エージェントは `background: true` で開発サーバーを実行し、コードの記述を続けます。サーバーが起動すると通知が表示されます。

<a id="continuous-test-monitoring"></a>

### 継続的なテスト監視

```
/loop 5m テストスイートを実行し、前回以降に発生した新しい失敗だけを報告して
```

エージェントは 5 分ごとにテストを実行し、新しい失敗だけを報告します。

<a id="log-monitoring"></a>

### ログ監視

特定のイベントを監視するには、`monitor` を使用します。

```
アプリケーションログの ERROR と WARN の項目を監視して。次を使用:
tail -f /var/log/app.log | grep --line-buffered -E "ERROR|WARN"
```

各エラーや警告が会話内の通知として表示されます。

<a id="ci-pipeline-watching"></a>

### CI パイプラインの監視

```
/loop 2m この PR の GitHub Actions 実行状態を確認し、完了したら報告して。
```

---

<a id="best-practices"></a>

## ベストプラクティス

- **1 回限りの長時間コマンドには `background` を使用する**（ビルド、テストスイート、サーバー起動）
- **定期確認には `/loop` を使用する**（CI 状態、テスト実行、ヘルスチェック）
- **リアルタイムのイベントストリームには `monitor` を使用する**（ログの追跡、ファイル監視）
- **遅延実行する 1 回限りのタスクには `recurring: false` を指定した `scheduler_create` を使用する**
- **モニターのフィルター条件を厳しくする** — 生ログのストリームではなく `grep --line-buffered` を推奨
- **通常のコマンドでポーリング用の sleep ループを使用しない** — 代わりに `timeout_ms` を指定した `get_command_or_subagent_output` を使用
- **適切なポーリング間隔を設定する** — レート制限を避けるためリモート API には 30 秒以上を設定し、ローカルの確認にはより短い間隔を設定
