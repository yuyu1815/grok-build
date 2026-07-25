<a id="agent-mode-acp-and-ide-integration"></a>

# エージェントモード（ACP）と IDE 連携

エージェントモードでは、Grok を ACP（Agent Client Protocol）サーバーとして実行し、IDE、エディター、カスタムツールと連携できます。1 つの応答を出力して終了するシングルプロンプトモード（`grok -p`）とは異なり、エージェントモードはプロセスを継続して実行し、構造化された JSON-RPC メッセージで通信します。

---

<a id="what-is-acp"></a>

## ACP とは

[Agent Client Protocol（ACP）](https://agentclientprotocol.com)は、AI エージェント通信用の標準規格です。クライアント（IDE、エディター、カスタムアプリ）が、構造化された JSON-RPC プロトコルを介して AI エージェントとやり取りする方法を定義します。ACP には次の機能があります。

- **セッション管理** -- 会話の作成、読み込み、再開
- **プロンプト送信** -- ユーザーメッセージの送信とストリーミング応答の受信
- **ツールの可視化** -- エージェントが使用中のツールをリアルタイムで確認
- **思考ストリーム** -- エージェントの推論過程を確認
- **権限処理** -- ツール実行を対話的に承認または拒否

---

<a id="stdio-transport"></a>

## stdio トランスポート

stdio は主要な連携モードです。エージェントは stdin と stdout を介して JSON-RPC メッセージを送受信します。

```bash
grok agent stdio
```

このモードを使用するクライアントには、次のものがあります。

- IDE 拡張機能（Zed、Neovim、Emacs など）
- カスタム自動化ツール
- ACP クライアントライブラリ

<a id="options"></a>

### オプション

以下は `grok agent` コマンドのオプションで、すべてのモードに適用されます。モード名の前に指定してください（例: `grok agent --model grok-build stdio`）。`stdio` サブコマンド自体にはオプションがありません。

| フラグ                     | 説明                                                         |
| -------------------------- | ------------------------------------------------------------ |
| `-m, --model <MODEL>`      | モデル ID を設定（例: `grok-build`）。                       |
| `--always-approve`         | すべてのツール実行を自動承認。（エイリアス: `--yolo`。）     |
| `--reauth`                 | エージェントの起動前に認証を実行。                           |
| `--agent-profile <PATH>`   | ファイルからエージェントプロファイルを読み込み。             |

---

<a id="server-mode"></a>

## サーバーモード

リモートクライアント向けの WebSocket サーバーとしてエージェントを実行します。

```bash
grok agent serve --bind 127.0.0.1:2419 --secret <token>
```

クライアントは WebSocket で接続し、シークレットトークンを使用して認証します。`--secret` を省略すると、エージェントはトークンを生成して起動時に出力します。`GROK_AGENT_SECRET` 環境変数で指定することもできます。エージェントは再接続後も維持されるため、クライアントが切断しても、後から進行中の作業を再開できます。

---

<a id="websocket-relay"></a>

## WebSocket リレー

ローカルネットワークではなくインターネット経由でエージェントに接続するには、WebSocket リレーサーバーを実行し、エージェントを接続します。

```bash
grok agent headless --grok-ws-url wss://your-relay.example.com/ws
```

エージェントはリレーへ外向きに接続し、Web クライアントも同じリレーへ接続します。ブラウザーからローカルプロセスを起動できない環境で Web UI を構築する場合に便利です。

---

<a id="acp-protocol-basics"></a>

## ACP プロトコルの基本

通信は JSON-RPC 2.0 形式に従います。一般的なセッションのライフサイクルは次のとおりです。

1. **初期化** -- クライアントが機能情報とともに `initialize` を送信
2. **セッション作成** -- クライアントが作業ディレクトリとともに `session/new` を送信
3. **プロンプト送信** -- クライアントがユーザーメッセージとともに `session/prompt` を送信
4. **更新の受信** -- エージェントがストリーミングコンテンツを含む `session/update` 通知を送信
5. **権限処理** -- エージェントがツール実行の承認を要求する場合がある

<a id="architecture"></a>

### アーキテクチャ

```
+------------------------------------------+
|           ACP Client                     |
|  (IDE, Editor, Custom Application)       |
+-------------------+----------------------+
                    | JSON-RPC over stdio
+-------------------v----------------------+
|           grok agent stdio               |
|                                          |
|  +---------+  +---------+  +---------+   |
|  | Session |  |  Tools  |  |   MCP   |   |
|  | Manager |  | Registry|  | Servers |   |
|  +---------+  +---------+  +---------+   |
+------------------------------------------+
```

---

<a id="streaming-updates"></a>

## ストリーミング更新

ACP は構造化されたイベントをストリーミングします。各 `session/update` 通知には、更新の種類を示す `sessionUpdate` フィールドが含まれます。

| `sessionUpdate` の値   | 説明                                                   |
| ---------------------- | ------------------------------------------------------ |
| `agent_message_chunk`  | エージェントの応答テキストの断片。                     |
| `agent_thought_chunk`  | エージェントの内部推論の断片。                         |
| `tool_call`            | 新しいツール呼び出し（タイトル、種類、状態、入力）。   |
| `tool_call_update`     | 実行中のツール呼び出しの状態または結果の更新。         |
| `plan`                 | エージェントの実行計画。                               |

各更新には種類が明記されるため、クライアントは推論、ツール呼び出し、応答テキストを別々のパネルに表示できます。

---

<a id="extension-methods"></a>

## 拡張メソッド

Grok は、基本 ACP プロトコルに加えて、SpaceXAI 固有の機能向けに `x.ai/` プレフィックスの拡張メソッドを定義しています。対象は次のとおりです。

| カテゴリー                 | プレフィックス       | 例                                               |
| -------------------------- | -------------------- | ------------------------------------------------ |
| **ファイルシステム**       | `x.ai/fs/*`          | `list`, `exists`, `read_file`, `write_file`      |
| **Git**                    | `x.ai/git/*`         | `status`, `stage`, `commit`, `diffs`, `discard`  |
| **Git Worktree**           | `x.ai/git/worktree/*`| `create`, `remove`, `apply`, `list`, `gc`        |
| **検索**                   | `x.ai/search/*`      | `fuzzy/open`, `fuzzy/change`, `content`          |
| **ターミナル**             | `x.ai/terminal/*`    | `create`, `kill`, `output`, `wait_for_exit`      |
| **セッション管理**         | `x.ai/session/*`     | `fork`, `resolve_local_for_worktree_resume`      |
| **会話と履歴**             | `x.ai/*`             | `prompt_history`, `rewind/*`, `compact_conversation` |
| **認証**                   | `x.ai/auth/*`        | `get_url`, `submit_code`                         |
| **フィードバックとテレメトリ** | `x.ai/*`         | `feedback`, `telemetry/*`                        |

この表は、各カテゴリーの代表的なメソッドを示しています。`x.ai/*` のメソッド群は SpaceXAI 固有で、リリースに伴って拡張される可能性があります。網羅的な一覧ではないため、利用可能なメソッドはエージェントの `initialize` 応答から確認してください。

<a id="notifications-agent-to-client"></a>

### 通知（エージェントからクライアント）

エージェントはリアルタイム更新のため、クライアントへプッシュ通知を送信します。

| 通知                       | 説明                                                         |
| -------------------------- | ------------------------------------------------------------ |
| `x.ai/search/fuzzy/status` | あいまい検索結果の更新                                       |
| `x.ai/git/worktree/status` | Worktree 作成の進行状況                                      |
| `x.ai/fs_notify`           | ファイルシステム変更通知                                     |
| `x.ai/fs/index`            | ファイルインデックス全体の更新                               |
| `x.ai/fs/index/delta`      | ファイルインデックスの差分更新                               |
| `x.ai/session_notification`| セッション固有の更新（差分レビュー、再試行状態、自動圧縮）   |
| `x.ai/session/update`      | セッション更新（ツール呼び出し、コンテンツ）                 |

---

<a id="session-_meta-options"></a>

## セッションの `_meta` オプション

`session/new` リクエストでは、次の任意の `_meta` フィールドを指定できます。

| フィールド             | 説明                                                   |
| ---------------------- | ------------------------------------------------------ |
| `rules`                | システムプロンプトに追加するルール。                   |
| `systemPromptOverride` | システムプロンプトを置き換える内容。                   |
| `agentProfile`         | 名前または JSON オブジェクトで指定するエージェントプロファイル。 |

---

<a id="acp-sdks"></a>

## ACP SDK

複数の言語向けに公式 SDK ライブラリが提供されています。

| 言語       | パッケージ                                                                               |
| ---------- | ---------------------------------------------------------------------------------------- |
| TypeScript | [`@agentclientprotocol/sdk`](https://www.npmjs.com/package/@agentclientprotocol/sdk)     |
| Rust       | [`agent-client-protocol`](https://crates.io/crates/agent-client-protocol)                |
| Python     | [`agent-client-protocol-python`](https://github.com/PsiACE/agent-client-protocol-python) |
| Go         | [`acp-go-sdk`](https://github.com/coder/acp-go-sdk)                                     |
| Kotlin     | [`acp`](https://github.com/agentclientprotocol/kotlin-sdk)                               |

---

<a id="compatible-clients"></a>

## 対応クライアント

| クライアント                                             | 対応状況     |
| -------------------------------------------------------- | ------------ |
| [Zed](https://zed.dev/docs/ai/external-agents)           | 対応済み     |
| [Neovim](https://neovim.io) (CodeCompanion, avante.nvim) | 対応済み     |
| [Emacs](https://github.com/xenodium/agent-shell)         | 対応済み     |
| [marimo notebook](https://github.com/marimo-team/marimo) | 対応済み     |
| JetBrains                                                | 対応予定     |

---

<a id="integration-example-a-typescript-acp-client"></a>

## 連携例: TypeScript ACP クライアント

```typescript
import { spawn, ChildProcess } from "child_process";
import * as readline from "readline";

class GrokACPChat {
  private proc!: ChildProcess;
  private sessionId!: string;
  private rl!: readline.Interface;

  constructor(private cwd = ".") {}

  async init() {
    this.proc = spawn("grok", ["agent", "stdio"]);
    this.rl = readline.createInterface({ input: this.proc.stdout! });

    // 初期化
    await this.request("initialize", {
      protocolVersion: 1,
      clientCapabilities: {
        fs: { readTextFile: true, writeTextFile: true },
        terminal: true,
      },
    });

    // セッションを作成
    const { sessionId } = await this.request("session/new", {
      cwd: this.cwd,
      mcpServers: [],
    });
    this.sessionId = sessionId;
    return this;
  }

  private async request(method: string, params: any): Promise<any> {
    return new Promise((resolve) => {
      const msg = JSON.stringify({ jsonrpc: "2.0", id: 1, method, params });
      this.proc.stdin!.write(msg + "\n");

      this.rl.once("line", (line) => {
        resolve(JSON.parse(line).result || {});
      });
    });
  }

  async *streamPrompt(text: string) {
    const msg = JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "session/prompt",
      params: {
        sessionId: this.sessionId,
        prompt: [{ type: "text", text }],
      },
    });
    this.proc.stdin!.write(msg + "\n");

    for await (const line of this.rl) {
      const data = JSON.parse(line);

      if (data.method === "session/update") {
        const update = data.params.update;
        yield update; // { sessionUpdate, content, title, ... }
      } else if (data.result) {
        break; // 最終応答
      }
    }
  }
}

// 使用例
const client = await new GrokACPChat(".").init();

for await (const update of client.streamPrompt("List the files in this project")) {
  switch (update.sessionUpdate) {
    case "agent_message_chunk":
      process.stdout.write(update.content?.text || "");
      break;
    case "agent_thought_chunk":
      console.log(`\n[Thinking: ${update.content?.text}]`);
      break;
    case "tool_call":
      console.log(`\n[Tool: ${update.title}]`);
      break;
  }
}
```

---

<a id="resources"></a>

## 関連資料

- [ACP 仕様](https://agentclientprotocol.com/protocol/prompt-turn)
- [プロトコルの概要](https://agentclientprotocol.com/overview/introduction)
