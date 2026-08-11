<a id="custom-models"></a>

# カスタムモデル

Grok は、代替プロバイダー、セルフホストモデル、組み込み設定の上書きに使用するカスタムモデルエンドポイントへ接続できます。このガイドでは、モデルの選択、エンドポイントの設定、サードパーティープロバイダーとの連携方法を説明します。

---

<a id="default-models"></a>

## デフォルトモデル

デフォルトでは、Grok は SpaceXAI がホストするモデルを使用し、新しいセッションは `grok-build` で開始されます。デフォルトモデルに設定は不要です。`grok login` または API キーで認証してから、セッションを開始してください。

利用可能なすべてのモデルを一覧表示します。

```bash
grok models
```

---

<a id="selecting-a-model"></a>

## モデルの選択

<a id="cli-flag"></a>

### CLI フラグ

```bash
grok -p "Hello" -m grok-build
```

<a id="slash-command"></a>

### スラッシュコマンド

TUI では、セッション中にモデルを切り替えられます。

```
/model grok-build
```

または、別名を使用します。

```
/m grok-build
```

<a id="model-picker-ctrlm"></a>

### モデルピッカー（Ctrl+M）

スクロールバックペインで `Ctrl+M` を押すと、モデルピッカーが開きます。組み込みとカスタムの両方を含む利用可能なすべてのモデルが表示され、1 回のキー入力で切り替えられます。プロンプトにフォーカスがある場合、`Ctrl+M` は複数行入力を切り替えます。プロンプトから離れずに切り替えるには `/model` を使用してください。

<a id="config-default"></a>

### 設定のデフォルト値

`~/.grok/config.toml` に永続的なデフォルトを設定します。

```toml
[models]
default = "grok-build"
```

---

<a id="supported-api-backends"></a>

## 対応する API バックエンド

Grok は 3 種類の API バックエンドに対応しています。モデルが使用するプロトコルを選択するには、`[model.*]` 設定で `api_backend` を指定します。

| 値 | API | デフォルト |
|----|-----|------------|
| `"chat_completions"` | OpenAI Chat Completions（`/v1/chat/completions`） | はい |
| `"responses"` | OpenAI Responses（`/v1/responses`） | |
| `"messages"` | Anthropic Messages（`/v1/messages`） | |

`api_backend` を省略すると、Grok は `chat_completions` を使用します。

Anthropic の `x-api-key` など、プロバイダー固有の認証ヘッダーやバージョンヘッダーを送信するには、後述の `extra_headers` フィールドを使用します。Grok はそれらのヘッダーを変更せず、エンドポイントへのすべてのリクエストで送信します。

---

<a id="configuring-custom-models"></a>

## カスタムモデルの設定

`~/.grok/config.toml` の `[model.<name>]` セクションに、カスタムモデルのエンドポイントを追加します。

```toml
[model.my-model]
model = "model-id"                        # API に送信するモデル識別子
base_url = "https://api.example.com/v1"   # OpenAI 互換エンドポイント
name = "Display Name"                     # モデルピッカーに表示
description = "Model description"          # 省略可能な説明
api_key = "sk-..."                        # このプロバイダーの API キー（省略可）
env_key = "XAI_API_KEY"                   # API キーを保持する環境変数（省略可、文字列または配列）
api_backend = "chat_completions"          # "chat_completions"、"responses"、または "messages"
temperature = 0.7                         # サンプリング温度
top_p = 0.95                              # 核サンプリングのパラメーター
max_completion_tokens = 8192              # 応答あたりの最大トークン数
context_window = 128000                   # コンテキストウィンドウの総トークン数
extra_headers = { "x-api-key" = "sk-..." } # 変更せず送信する追加リクエストヘッダー（省略可）
```

<a id="credential-resolution"></a>

### 認証情報の解決順序

Grok は次の順序で API キーを解決します。

1. モデル設定の `api_key` フィールド
2. `env_key` で指定した環境変数（単一の文字列または名前の配列）。設定済みの空でない最初の値が使用されます（例: SSH の `LC_*` 転送では `env_key = ["ANTHROPIC_AUTH_TOKEN", "LC_ANTHROPIC_AUTH_TOKEN"]`）
3. 独自の `api_key` / `env_key` がないモデルでは、サインイン済みセッショントークン（`grok login` で取得）
4. `XAI_API_KEY` 環境変数（グローバルフォールバック。後方互換性のため `GROK_CODE_XAI_API_KEY` も使用可能）

<a id="context-window"></a>

### コンテキストウィンドウ

`context_window` の値は、自動圧縮を開始するタイミングを Grok に伝えます。既知のモデルを上書きする場合、そのモデルのコンテキストウィンドウが継承されます。新しいモデルを定義して `context_window` を省略すると、Grok はデフォルトで 200,000 トークンを使用するため、プロバイダーに合わせて明示的に設定してください。

<a id="global-default-headers"></a>

### グローバルデフォルトヘッダー

カタログ内の*すべて*のモデル（組み込み、`/v1/models` から事前取得、カスタム）に同じヘッダーを適用するには、モデルごとに繰り返さず、グローバルな `[models]` セクションで一度だけ設定します。

```toml
[models]
extra_headers = { "X-Request-Tags" = "team=example,env=prod" }
```

これは、各モデルの推論リクエストのベースになります。モデル単位の `[model.<id>].extra_headers` エントリは、グローバルデフォルトを**キーごと**に上書きします（大文字と小文字は区別しません）。モデルで設定したキーが優先され、グローバルにしかないキーは引き続きそのモデルに継承されます。モデル単位のフィールドと同様に、これらはモデルの推論呼び出しにのみ付加され、画像生成や動画生成などの別サービスには送信されません。そのため、新しいモデルが追加されるたびに再宣言せず、コスト追跡などの属性タグを付けるのに便利です。

<a id="global-default-values"></a>

### グローバルデフォルト値

モデル単位の一般的な設定の一部も、*すべて*のモデルのデフォルトとして `[models]` で一度だけ設定できます。モデル単位の `[model.<id>]` の値が常に優先され、モデル（またはサーバーのモデル一覧）でフィールドが未設定の場合にのみグローバル値が補完されます。

```toml
[models]
temperature                 = 0.7
top_p                       = 0.95
max_completion_tokens       = 8192
max_retries                 = 8
inference_idle_timeout_secs = 600
stream_tool_calls           = true
```

これは環境全体に適用する、少数の固定設定です。特定のモデルを識別する設定（`model`、`base_url`、`api_key`、`context_window` など）は、この方法ではデフォルトにできません。また、専用の設定がある自動圧縮（`[session]`）、システムプロンプトのラベル（`[agent]`）、推論の強度（`[models].default_reasoning_effort`）は、従来どおりそれぞれの場所で設定します。

> **`stream_tool_calls` に関する注意:** これはサンプリングだけでなく、リクエストの*形式*にも影響します。一部のエンドポイント（BYOK プロバイダーなど）では未設定であることが求められます。グローバルな `stream_tool_calls = true` がそのようなモデルで問題を引き起こす場合は、そのモデルの `[model.<id>]` ブロックで `stream_tool_calls = false` を指定して無効にしてください。

---

<a id="overriding-built-in-models"></a>

## 組み込みモデルの上書き

組み込みモデルは、すべてを再定義せずに特定のフィールドだけを上書きできます。変更するフィールドだけを指定してください。

```toml
# デフォルトモデルの API キーだけを上書き
[model.grok-build]
api_key = "my-api-key"

# temperature を上書きし、カスタム API キーを追加
[model.grok-build]
temperature = 0.5
api_key = "sk-custom"
```

組み込みモデルを上書きすると、Grok は正しい `base_url` を含むデフォルト設定を基に、指定されたフィールドだけを適用します。未指定のフィールドはデフォルトから継承されます。

<a id="priority-order"></a>

### 優先順位

1. ユーザー設定（`[model.*]`）-- 最優先
2. リモートの `/v1/models` から事前取得したモデル
3. ハードコードされたデフォルト -- 優先度最低

---

<a id="provider-examples"></a>

## プロバイダー別の例

<a id="anthropic-claude"></a>

### Anthropic（Claude）

Anthropic Messages API を介して Claude モデルを直接使用します。

```toml
[model.claude-opus]
model = "claude-opus-4-6"
base_url = "https://api.anthropic.com/v1"
name = "Claude Opus 4.6"
api_backend = "messages"
context_window = 200000
extra_headers = { "x-api-key" = "sk-ant-...", "anthropic-version" = "2023-06-01" }
```

`messages` バックエンドは Anthropic Messages プロトコルを使用します。Anthropic は `Authorization: Bearer` ではなく `x-api-key` ヘッダーで認証するため、Grok が変更せず送信する `extra_headers` を介してキーを渡してください。

<a id="openai-chat-completions"></a>

### OpenAI（Chat Completions）

```toml
[model.gpt-4o]
model = "gpt-4o"
base_url = "https://api.openai.com/v1"
name = "GPT-4o"
env_key = "OPENAI_API_KEY"
```

`api_backend` のデフォルトは `"chat_completions"` のため、OpenAI では明示的に設定する必要はありません。

<a id="openai-responses-api"></a>

### OpenAI（Responses API）

プロバイダーが新しい Responses API に対応している場合は、次のように設定します。

```toml
[model.gpt-4o-responses]
model = "gpt-4o"
base_url = "https://api.openai.com/v1"
name = "GPT-4o (Responses)"
api_backend = "responses"
env_key = "OPENAI_API_KEY"
```

<a id="ollama-local-models"></a>

### Ollama（ローカルモデル）

[Ollama](https://ollama.ai) を使用してモデルをローカルで実行します。

```toml
[model.ollama-codellama]
model = "codellama"
base_url = "http://localhost:11434/v1"
name = "CodeLlama (Ollama)"
```

Ollama が実行中（`ollama serve`）で、モデルが取得済み（`ollama pull codellama`）であることを確認してください。

<a id="together-ai"></a>

### Together AI

```toml
[model.together-mixtral]
model = "mistralai/Mixtral-8x7B-Instruct-v0.1"
base_url = "https://api.together.xyz/v1"
name = "Mixtral 8x7B"
env_key = "TOGETHER_API_KEY"
```

<a id="local-openai-compatible-server"></a>

### ローカルの OpenAI 互換サーバー

OpenAI Chat Completions API または Responses API を実装する任意のサーバーを使用できます。

```toml
[model.local-llama]
model = "llama-3.1-70b"
base_url = "http://localhost:8080/v1"
name = "Local Llama"
temperature = 0.8
```

---

<a id="custom-models-endpoint"></a>

## カスタムモデルエンドポイント

デフォルトの代わりに、Grok をカスタムの OpenAI 互換 `/v1/models` エンドポイントへ接続します。モデルが企業ゲートウェイやセルフホストの推論サービスの背後にある場合に使用します。

<a id="environment-variables"></a>

### 環境変数

| 変数 | 必須 | 説明 |
|------|------|------|
| `GROK_MODELS_BASE_URL` | はい | 推論用のベース URL。Grok は `{base_url}/models` からモデル一覧を取得します。 |
| `XAI_API_KEY` | はい | `Authorization: Bearer` として送信する API キー。`GROK_CODE_XAI_API_KEY` も使用できます。 |
| `GROK_MODELS_LIST_URL` | いいえ | モデル一覧の URL が `{base_url}/models` と異なる場合に上書きします。 |

<a id="setup"></a>

### セットアップ

```bash
export GROK_MODELS_BASE_URL="https://api.acme.com/v1"
export XAI_API_KEY="xai-..."
grok
```

<a id="config-file-alternative"></a>

### 設定ファイルを使用する方法

```toml
[endpoints]
models_base_url = "https://api.acme.com/v1"

# 特定のモデルの API キーだけを上書き
[model.grok-build]
api_key = "my-api-key"
```

`[endpoints]` とモデルの部分的な上書きを併用すると、Grok はエンドポイント設定から `base_url` を継承するため、各 `[model.*]` セクションで指定する必要はありません。

<a id="auth-behavior"></a>

### 認証動作

`models_base_url` を設定すると、Grok はセッション認証ではなく API キー認証（`Authorization: Bearer`）を使用します。`grok login` は不要で、API キーだけで認証できます。

---

<a id="web-search-model"></a>

## Web 検索モデル

`web_search` ツールは別のモデルを使用します。次のように設定します。

```toml
[models]
web_search = "grok-4.20-multi-agent"
```

または、環境変数を使用します。

```bash
export GROK_WEB_SEARCH_MODEL="grok-4.20-multi-agent"
```

Web 検索でカスタムモデルを使用する場合、Grok が接続できるよう `[model.*]` エントリも必要です。サーバー側（「バックエンド」）の Web 検索は、モデルで `supports_backend_search = true` が設定されている場合にのみ実行されます（かつ、ビルドでバックエンド検索が有効な場合）。これは `api_backend` には依存しません。

```toml
[models]
web_search = "my-custom-model"

[model.my-custom-model]
model = "my-custom-model"
supports_backend_search = true
```

---

<a id="using-custom-models"></a>

## カスタムモデルの使用

```bash
# カスタムを含む利用可能なモデルを一覧表示
grok models

# TUI でスラッシュコマンドを使って選択
/model my-model

# ヘッドレスモードで使用
grok -p "Hello" -m my-model

# config.toml でデフォルトに設定:
[models]
default = "my-model"
```

---

<a id="enterprise-deployment"></a>

## エンタープライズ環境への導入

カスタムモデルを使用するエンタープライズ環境向けの完全な設定例です。

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

<a id="troubleshooting"></a>

## トラブルシューティング

<a id="model-not-found"></a>

### モデルが見つからない

```bash
# 利用可能なモデルを一覧表示
grok models

# config.toml の [model.*] セクションに誤字がないか確認
```

<a id="connection-errors"></a>

### 接続エラー

エンドポイントに到達できることを確認します。

```bash
curl -s https://api.example.com/v1/models \
  -H "Authorization: Bearer $XAI_API_KEY"
```

<a id="debug-logging"></a>

### デバッグログ

```bash
RUST_LOG=debug GROK_LOG_FILE=/tmp/grok.log grok
tail -f /tmp/grok.log
```

モデルの選択と API 呼び出しを追跡するには、`model` または `sampling` を含むログエントリを確認してください。
