# Grok CodeのProvider Login基盤 調査・実装計画

## 目的

今回は、Grok Codeをprovider中心のアプリへ変更し、Grok loginを起動条件から切り離す。

今回実装する範囲は以下とする。

- provider単位のlogin
- 起動時のlogin必須画面の削除
- 利用可能モデルがない状態の正常化
- chat送信時のprovider login案内
- providerごとの認証状態とモデル一覧
- `~/.grok/auth/<provider>.json`による認証保存
- Grok loginの`grok.json`への整理
- provider共通の認証・モデル管理境界

```text
Grok Code
  └─ Provider Login基盤
       └─ 現在はGrok providerを対象に実装
```

Credentialのcanonicalな保存場所は以下とする。

```text
~/.grok/auth/
  grok.json
  codex.json       # 将来拡張
  claude.json      # 将来拡張
  gemini.json      # 将来拡張
```

providerごとにファイルとschemaを分離する。今回の実装対象はGrokのprovider login基盤であり、Codexのlogin・chat・App Server対応は将来拡張として調査結果だけを残す。

---

## 今回の実装対象

- Grok providerのlogin状態管理
- `/login grok`のprovider単位login
- provider共通のlogin command設計
- providerなしでの起動
- 利用可能モデルなし状態の表示
- provider/modelがない場合のchat案内
- `~/.grok/auth/grok.json`への移行・保存
- 内部provider IDと表示名の分離
- 将来providerを追加できるregistry境界

## 将来拡張として調査する対象

- `/login codex`
- `~/.grok/auth/codex.json`
- 公式Codex login・storage・refresh
- 公式Codex chat・App Server
- Codex subagent
- Claude Code provider
- Gemini CLI provider

---

# Provider単位のloginと起動状態

## Providerごとのlogin

ログインはprovider単位で行う。

今回の対象はGrok providerとする。

```text
/login
/login grok
```

将来providerを追加した場合は、同じ形式でloginできるようにする。

```text
/login codex
/login claude
/login gemini
```

既存の構文との互換性が必要な場合は、今回のGrok loginで次の形式も扱えるようにする。

```text
/login provider grok
```

login成功後は、該当providerの認証状態と利用可能モデルを登録する。

## 起動時の状態

起動時に特定providerのloginを必須にしない。

```text
起動
  → loginなしでもUIを開く
  → provider/modelを読み込む
  → 利用可能モデルがなければ待機
```

`crates/codegen/xai-grok-shell/src/agent/app.rs:441-476`にある起動時のlogin必須経路を、providerの利用時に認証状態を確認する方式へ変更する。

## 利用可能モデルがない状態

providerが未loginで利用可能モデルがない状態は、エラーではなく通常の初期状態として扱う。

```text
利用可能なモデルがありません。
利用するproviderにloginしてください。

/login grok
```

chat送信時に利用可能モデルがなければ、provider loginの案内を表示する。

```text
利用可能なモデルがありません。
/login grok を実行してください。
```

選択中のproviderだけが未loginの場合は、provider固有の案内を表示する。

```text
選択中のproviderがloginされていません。
/login <provider> を実行してください。
```

## 状態モデル

```text
起動
  ├─ providerなし
  │    └─ login案内を表示
  │
  ├─ providerあり・モデルあり
  │    └─ chat可能
  │
  ├─ providerあり・未login
  │    └─ provider login案内
  │
  └─ providerあり・モデル取得失敗
       └─ provider固有エラー
```

chat可否はGrok loginの有無ではなく、最終的な利用可能provider/modelの有無で判断する。

## ProviderAuthの共通概念

```text
ProviderAuth
  ├─ provider_id
  ├─ display_name
  ├─ login()
  ├─ logout()
  ├─ status()
  └─ available_models()
```

## 内部IDと表示名

内部IDと表示名を分離する。今回の実装対象は`xai` / `Grok`とし、その他は将来provider用に予約する。

| 内部ID | 表示名 | 認証所有者 |
|---|---|---|
| `xai` | Grok | Grok AuthManager |
| `codex` | Codex | CodexProvider |
| `claude` | Claude | ClaudeProvider |
| `gemini` | Gemini | GeminiProvider |

既存コードでは内部IDとして`xai`を維持し、UIでは`Grok`と表示する。既存のauth、model、proxy、設定互換性を保ちながら、provider registryと表示名を整理する。

## 認証関連コマンド

今回の対象コマンド:

```text
/login
/login grok
/logout grok
/auth
/auth status
/providers
/models
```

将来provider追加後:

```text
/login <provider>
/logout <provider>
```

`/auth status`ではproviderごとのlogin状態と利用可能モデル数を表示する。

今回の表示例:

```text
Grok   logged in   5 models available
```

将来providerを追加した場合:

```text
Grok   logged in   5 models available
Codex  logged in   3 models available
Claude logged out
```

## Credentialの保存場所

providerごとに保存場所とファイルを分ける。今回はGrokの保存先だけを実装し、その他のファイル名は将来provider用に予約する。

```text
~/.grok/auth/
  grok.json       # 今回実装
  codex.json      # 将来拡張
  claude.json     # 将来拡張
  gemini.json     # 将来拡張
```

ファイルの保存場所は統一するが、中身のschemaはproviderごとに維持する。

- `grok.json`: GrokAuth形式
- `codex.json`: Codex認証形式
- `claude.json`: Claude固有形式
- `gemini.json`: Gemini固有形式

各providerのcredential fileはatomic write、owner-only permission、provider単位のlock、backup/recoveryを持つ。

---

# 将来拡張調査: 公式Codexのlogin・保存方式

## 1.1 Codex loginの入口

### 調査対象

- `[調査]` `.REF/codex/codex-rs/cli/src/login.rs`
- `[調査]` `.REF/codex/codex-rs/login/src/lib.rs`
- `[調査]` `.REF/codex/codex-rs/login/src/auth/manager.rs`

### 確認事項

- `codex login`の起動経路
- Browser loginとDevice loginの違い
- login後に保存される情報
- login済み状態の確認方法
- logout時に削除される情報
- Grok Codeからloginを起動する必要があるか

### 決めること

- ユーザーが事前に`codex login`する方式
- またはGrok Codeから`codex login`を起動する方式
- login状態をGrok CodeのUIでどこまで表示するか

---

## 1.2 Codex credential保存

### 調査対象

- `[調査]` `.REF/codex/codex-rs/login/src/auth/storage.rs`
- `[調査]` `.REF/codex/codex-rs/login/src/token_data.rs`
- `[調査]` `.REF/codex/codex-rs/login/src/auth/manager.rs`

### 確認事項

- `$CODEX_HOME/auth.json`
- OS keyring
- Secrets backend
- Ephemeral mode
- `AuthManager`による読み込み
- `AuthManager`のreload
- refresh tokenの保存・更新
- account IDの保存

### 方針

- `[決定]` canonicalな保存場所は`~/.grok/auth/<provider>.json`とする
- `[決定]` Codexは`~/.grok/auth/codex.json`を使用する
- `[決定]` Grokは`~/.grok/auth/grok.json`を使用する
- `[決定]` providerごとにcredential fileとschemaを分離する
- `[決定]` 公式Codexのstorage実装を参考にし、保存先をGrok Code向けに調整する
- `[必須]` tokenをprovider間でfallback・共有しない

---

## 1.3 Token refresh

### 調査対象

- `[調査]` `.REF/codex/codex-rs/login/src/auth/manager.rs`
- `[調査]` `.REF/codex/codex-rs/model-provider/src/auth.rs`
- `[調査]` `.REF/codex/codex-rs/core/src/client.rs`

### 確認事項

- Tokenの有効期限判定
- Proactive refresh
- 401時のrefresh
- Refresh token rotation
- Refresh失敗時の状態
- Refresh競合
- `AuthManager`の共有範囲
- Login/logout後のreload方法

### 決めること

- Codex App Serverを長寿命プロセスにする理由
- 毎回Codex CLIを起動しない理由
- login/logout後にApp Serverをreloadするか再起動するか

---

# 将来拡張調査: 公式Codexのchat実行経路

## 2.1 App Serverの起動方法

### 調査対象

- `[調査]` `.REF/codex/codex-rs/app-server/src/main.rs`
- `[調査]` `.REF/codex/codex-rs/app-server/src/lib.rs`
- `[調査]` `.REF/codex/codex-rs/app-server-transport/src/transport/`

### 確認事項

- App Serverの起動方法
- stdio transport
- WebSocket transport
- JSON-RPC初期化
- プロセス終了・再起動
- Error通知
- Protocol version

### 初期方針

- `[判断]` 最初はstdio transportを使う
- `[判断]` TCP/WebSocketは後回し
- `[判断]` Codex App Serverは長寿命child processにする

---

## 2.2 Chat / Session protocol

### 調査対象

- `[調査]` `.REF/codex/codex-rs/app-server-protocol/src/protocol/common.rs`
- `[調査]` `.REF/codex/codex-rs/app-server-protocol/schema/typescript/v2/`
- `[調査]` `ThreadStartParams`
- `[調査]` `TurnStartParams`
- `[調査]` Streaming notification定義

### 確認事項

- `initialize`
- `thread/start`
- `turn/start`
- Thread ID
- Turn ID
- Message event
- Tool event
- Usage event
- Completed event
- Error event
- Session resume

### 決めること

- Grok Codeのsubagent sessionとCodex threadの対応方法
- Codexのstream eventをGrok eventへ変換する方法
- Session resumeをどう扱うか

---

## 2.3 Tool ownership

### 調査対象

- `[調査]` `.REF/codex/codex-rs/app-server/`
- `[調査]` `.REF/codex/codex-rs/core/`
- `[調査]` `.REF/motosan-ai/sdks/rust/src/providers/codex_cli/`

### 確認事項

- ToolをCodex側が実行するのか
- Grok Code側が実行するのか
- Sandboxとapprovalの責任者
- MCPの責任者
- Tool結果の形式

### 初期方針

- `[判断]` Codexを独立したサブエージェントとして扱う
- `[判断]` Codex側のtool / sandbox / approvalを使う
- `[注意]` Grok Code側のtool dispatcherと二重実行しない

---

# Phase 1: 現在のGrok Code側を調査

## 1.1 Grok loginの範囲

### 調査対象

- `[調査]` `crates/codegen/xai-grok-shell/src/auth/model.rs`
- `[調査]` `crates/codegen/xai-grok-shell/src/auth/manager.rs`
- `[調査]` `crates/codegen/xai-grok-shell/src/auth/flow.rs`
- `[調査]` `crates/codegen/xai-grok-shell/src/auth/storage.rs`

### 確認事項

- Grok credentialの保存
- Grok OAuth / OIDC
- Token refresh
- Login / logout
- Grok user/team/org policy
- 外部auth command

### 方針

- `[判断]` Grok `AuthManager`はxAI provider専用に維持する
- `[判断]` Grok credentialは`~/.grok/auth/grok.json`へ保存する
- `[判断]` providerのlogin状態とGrok login状態を分けて扱える境界を作る
- `[判断]` 将来providerのcredentialをGrokAuthへ混ぜない

---

## 1.2 Grok tokenが推論へ渡る場所

### 調査対象

- `[調査]` `crates/codegen/xai-grok-shell/src/agent/config.rs:4256-4324`
- `[調査]` `crates/codegen/xai-grok-shell/src/agent/config.rs:4591-4692`
- `[調査]` `crates/codegen/xai-grok-shell/src/util/grok_auth_credentials.rs`
- `[調査]` `crates/codegen/xai-grok-auth/src/auth_provider.rs`
- `[調査]` `crates/codegen/xai-grok-auth/src/retry_middleware.rs`

### 確認事項

- Model credentialの解決
- Grok session tokenのfallback
- `XAI_API_KEY`のfallback
- xAI専用ヘッダー
- 401 refresh / retry

### 必須条件

- `[必須]` `xai`モデルだけがGrok `AuthManager`を使う
- `[必須]` 未loginのproviderがGrok tokenへfallbackしない境界を作る
- `[必須]` providerごとの401・refresh状態を分離できる構造にする
- `[必須]` xAI専用ヘッダーを将来providerへ送らない構造にする

---

## 1.3 Model routing

### 調査対象

- `[調査]` `crates/codegen/xai-grok-shell/src/agent/config.rs`
- `[調査]` `crates/codegen/xai-grok-shell/src/agent/models.rs`
- `[調査]` `crates/codegen/xai-grok-sampler/src/config.rs`
- `[調査]` `crates/codegen/xai-grok-sampler/src/client.rs`

### 確認事項

- Model IDとproviderの現在の関係
- Model picker
- Sampler接続
- Endpoint決定
- API backend決定
- Session keyの渡し方

### 決めること

- Modelにprovider識別子を持たせる
- providerごとの利用可能モデルを管理する
- Sampler経由と将来provider runtime経由を分けられる境界を作る
- 将来providerのsubagent対応を追加できる構造にする

---

# 将来拡張の参考調査: motosan-ai・CLIProxyAPI

## 4.1 motosan-ai

### 調査対象

- `[調査]` `.REF/motosan-ai/sdks/rust/src/providers/mod.rs`
- `[調査]` `.REF/motosan-ai/sdks/rust/src/client.rs`
- `[調査]` `.REF/motosan-ai/sdks/rust/src/providers/codex_cli/`
- `[調査]` `.REF/motosan-ai/sdks/rust/src/providers/chatgpt_codex.rs`
- `[調査]` `.REF/motosan-ai/docs/cli-runtime-integration-requirements.md`

### 確認事項

- Provider abstraction
- `codex exec --json`
- NDJSON event変換
- Session resume
- CLI providerのtool ownership
- 他provider追加方法

### 方針

- `[推奨]` Codex CLI adapterの参考にする
- `[推奨]` MVPのプロトタイプに利用する

---

## 4.2 CLIProxyAPI

### 調査対象

- `[調査]` `.REF/CLIProxyAPI/sdk/auth/`
- `[調査]` `.REF/CLIProxyAPI/internal/auth/codex/`
- `[調査]` `.REF/CLIProxyAPI/internal/runtime/executor/`
- `[調査]` `.REF/CLIProxyAPI/README_JA.md`

### 確認事項

- Provider別auth manager
- Codex OAuth
- Account管理
- Request / response変換
- 複数provider routing
- Proxy server構造

### 方針

- `[参考]` Provider別認証設計
- `[参考]` Refresh競合対策
- `[参考]` Codex request変換

---

# Phase 2: Provider Login基盤の設計・実装

## 現在の採用構成

```text
Grok Code
  └─ Provider Login基盤
       └─ GrokProvider
            └─ ~/.grok/auth/grok.json
```

### 決定事項

- `[決定]` 起動時にGrok loginを必須にしない
- `[決定]` `/login <provider>`でprovider単位のloginを行う
- `[決定]` 利用可能モデルがない状態でもUIを起動する
- `[決定]` chat送信時に利用可能model/providerを確認する
- `[決定]` 利用可能モデルがなければlogin案内を表示する
- `[決定]` Grok credentialのcanonicalな保存場所は`~/.grok/auth/grok.json`とする
- `[決定]` provider共通の認証・モデル管理境界を作る
- `[決定]` 内部provider IDと表示名を分離する
- `[決定]` Codexは将来providerとしてregistryに登録できる形にする

## 現在のMVP

```text
起動
  ↓
provider/modelの状態確認
  ↓
利用可能モデルがあればchat
  ↓
利用可能モデルがなければlogin案内
```

### MVPの目印

- `[MVP]` loginなしでGrok Codeを起動する
- `[MVP]` `/login grok`をprovider単位のloginとして扱う
- `[MVP]` `/login provider grok`形式との互換性を確認する
- `[MVP]` Grok credentialを`~/.grok/auth/grok.json`へ保存する
- `[MVP]` 起動時のlogin必須画面を削除する
- `[MVP]` 利用可能モデルなし状態を表示する
- `[MVP]` chat送信時にlogin案内を表示する
- `[MVP]` providerごとの認証状態を表示する
- `[MVP]` providerごとの利用可能モデル数を表示する
- `[MVP]` `/auth status`を表示する
- `[MVP]` login後にprovider/model一覧を再読み込みする
- `[MVP]` logout後にprovider/model一覧を更新する
- `[MVP]` 未login状態とモデル取得失敗を区別する

---

# Phase 3: Provider共通境界の確認

## Provider adapter

### 調査・設計の目印

- `[設計]` Providerごとの起動方法
- `[設計]` Providerごとのlogin状態
- `[設計]` Providerごとのstream event
- `[設計]` Providerごとのsession管理
- `[設計]` Providerごとのtool ownership
- `[設計]` Providerごとのcapability
- `[設計]` Providerごとのエラー分類
- `[設計]` Providerごとの終了・再起動処理

### 将来追加するprovider

- `[将来]` Codex CLI
- `[将来]` Claude CLI
- `[将来]` Gemini CLI
- `[将来]` その他CLI provider
- `[将来]` API key型provider

---

# Phase 4: 最終確認

## Login状態

- `[確認]` loginなしでGrok Codeを起動できる
- `[確認]` `/login grok`でGrok loginを開始できる
- `[確認]` `/login provider grok`形式を必要に応じて扱える
- `[確認]` `/logout grok`でGrok credentialを削除できる
- `[確認]` `/auth status`でGrokのlogin状態を表示できる

## Credential保存

- `[確認]` Grok credentialが`~/.grok/auth/grok.json`に保存される
- `[確認]` provider単位の保存ディレクトリが作成される
- `[確認]` 保存がatomic writeで行われる
- `[確認]` 保存ファイルのpermissionが適切である
- `[確認]` login/logout後にcredential状態が再読み込みされる

## Model状態

- `[確認]` loginなしでもUIが起動する
- `[確認]` 利用可能モデルなし状態を正常に表示できる
- `[確認]` 利用可能モデルがある場合はchatできる
- `[確認]` 利用可能モデルがない場合はlogin案内を表示する
- `[確認]` モデル取得失敗と未loginを区別して表示する
- `[確認]` providerごとの利用可能モデル数を表示できる

## Provider境界

- `[確認]` 内部provider IDと表示名が分離されている
- `[確認]` Grok loginがprovider共通のlogin状態として扱える
- `[確認]` xAI専用認証情報が将来provider用の経路へ混ざらない
- `[確認]` Codexなど将来providerをregistryへ追加できる境界がある

---

# 今回の調査で確定すること

1. 認証情報のcanonicalな保存場所
   - `~/.grok/auth/<provider>.json`
   - 今回は`~/.grok/auth/grok.json`を実装する
   - `codex.json`などは将来provider用に予約する

2. 今回の実装対象
   - Provider単位のlogin基盤
   - 起動時のlogin必須処理の解除
   - 利用可能モデルなし状態
   - chat送信時のlogin案内
   - Grok credentialの保存場所整理
   - Provider/model状態管理
   - Provider共通registry境界

3. Grok Codeの責任
   - Provider選択
   - Providerごとのlogin
   - Providerごとのcredential保存
   - Providerごとの認証状態
   - Providerごとの利用可能モデル
   - Login/logout後のmodel再読み込み
   - 未login・モデルなし・モデル取得失敗の状態表示

4. 将来providerの扱い
   - Codexは`codex.json`とCodexProviderを将来追加する
   - Claudeは`claude.json`とClaudeProviderを将来追加する
   - Geminiは`gemini.json`とGeminiProviderを将来追加する
   - 公式provider実装、motosan-ai、CLIProxyAPIは将来拡張の調査資料として残す

5. Provider共通境界
   - Providerごとにlogin・storage・runtime・sessionを分離する
   - 内部provider IDと表示名を分離する
   - Providerごとに認証状態とモデル一覧を管理する

## 最終方針

**今回はCodexを実装せず、Grok CodeのProvider Login基盤だけを整備する。**

起動時にGrok loginを必須にせず、`/login <provider>`で必要なproviderをloginできる構造にする。利用可能なモデルがない場合は、chat送信時にlogin案内を表示する。

認証情報は`~/.grok/auth/<provider>.json`で管理し、今回のGrok用保存先は`~/.grok/auth/grok.json`とする。Codex、Claude、Geminiのlogin・chat・runtimeは将来providerとして同じ境界へ追加できるようにする。
