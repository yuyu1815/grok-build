# Grok Provider Login基盤 詳細設計

## 0. 文書の位置付け

本書は `CODEX_PROVIDER_INTEGRATION_PLAN.md` とは別に、**Codexを実装対象から外し、Grok/xAI providerだけを対象**として、既存コードの依存関係・呼び出し関係・追加ファイル・追加関数・変更対象・実装順序を具体化する詳細設計である。

本書は設計・調査結果であり、実装そのものは含まない。

### 調査対象外

以下は今回の設計・実装対象外とする。

- Codex login
- Codex credential storage
- Codex App Server
- Codex chat/runtime
- Codex subagent
- `codex.json`
- Claude/Geminiのprovider実装
- providerごとの外部CLI runtime
- provider間のtoken共有・fallback

将来providerを追加できる境界は設計するが、今回登録する実providerはGrok/xAIだけとする。

---

## 1. 先に確定する設計判断

### 1.1 providerの正規ID

既存コード・計画書の互換性を考慮し、内部IDと表示名を次のように固定する。

| 種別 | 値 |
|---|---|
| canonical internal provider ID | `xai` |
| UI display name | `Grok` |
| login command alias | `grok` |
| accepted internal alias | `xai` |
| canonical credential file | `~/.grok/auth/grok.json` |

`/login grok` は入力エイリアスであり、内部では `ProviderId::Xai` に正規化する。

`xai` と `grok` を呼び出し箇所ごとに個別比較しない。provider parserを一箇所に置き、以後は `ProviderId` を渡す。

### 1.2 現状コードとの差異

計画書では credential の保存先を `~/.grok/auth/grok.json` としているが、現状実装の主保存先は次の通りである。

- `auth/manager.rs` の `AuthManager::new` は原則 `grok_home.join("auth.json")`
- `auth/storage.rs` も `auth.json` を前提にする
- `GROK_AUTH_PATH` による既存上書き経路がある
- auth file lock、API key保存、migration、テストも `auth.json` 前提の箇所がある

したがって、保存先変更は単純なファイル名変更ではなく、**path解決・lock・atomic write・legacy migration・reload・削除を一つのstorage境界で変更する**。

### 1.3 起動時login必須経路の解釈

計画書が示す `agent/app.rs:441-476` は、調査結果上は主に headless 経路の session credential 必須処理である。

- stdio/interactiveの起動生成経路は、同じ箇所で直接loginを開始していない
- headlessは既存仕様として session 必須の意味を持つ
- interactive UIのlogin強制表示は `xai-grok-pager` 側の `AuthState` など別経路で存在する可能性がある

よって、実装では次のpolicyを分離する。

| 起動モード | 初期方針 |
|---|---|
| interactive/TUI | loginなしでUIを起動する |
| stdio/ACP | credentialなしでinitializeできるようにする |
| headless | 既存のsession必須仕様を維持する。緩和する場合は別決定とする |

headlessのcredential必須を、interactiveのlogin必須解除と同じ変更で削除しない。

### 1.4 provider abstractionの大きさ

Codexを保留するため、動的な `Box<dyn Provider>` 階層やproviderごとのruntime traitを先行実装しない。

今回追加する境界は次の最小構成とする。

- `ProviderId`
- `ProviderDescriptor`
- `ProviderRegistry`（Grokのみを返す固定registry）
- `GrokProvider` facade
- provider共通の状態・chat gate・表示用summary
- Grok固有処理は既存 `AuthManager`、`ModelsManager`、`auth::flow` に委譲

新規ファイルはまず `crates/codegen/xai-grok-shell/src/provider.rs` 一つにまとめる。provider固有runtime・認証transport・モデルfetchをこのファイルへ移植しない。

---

## 2. 既存アーキテクチャの全体像

```text
xai-grok-shell
  ├─ agent/app.rs
  │    ├─ AuthManager生成
  │    ├─ ModelsManager生成
  │    ├─ watcher / ConfigUpdate処理
  │    └─ MvpAgent生成
  │
  ├─ auth/
  │    ├─ model.rs                 GrokAuth/AuthMode/AuthStore
  │    ├─ manager.rs               credentialのmemory/disk/refresh中心
  │    ├─ flow.rs                  login/logout/refresh入口
  │    ├─ storage.rs               auth.jsonのread/write/lock/backup
  │    ├─ credential_provider.rs   HTTP用Grok credential adapter
  │    └─ error.rs                 auth/refreshエラー分類
  │
  ├─ agent/config.rs
  │    ├─ ModelEntryConfig
  │    ├─ ModelInfo / ModelEntry
  │    ├─ credential解決
  │    └─ SamplerConfigへの変換
  │
  ├─ agent/models.rs
  │    ├─ ModelsManager
  │    ├─ /v1/models fetch
  │    ├─ models_cache.json
  │    ├─ current model
  │    └─ x.ai/models/update通知
  │
  ├─ agent/mvp_agent/
  │    ├─ acp_agent.rs             initialize/model state/chat gate
  │    └─ agent_ops.rs             model選択/credential/sampling準備
  │
  ├─ session/
  │    ├─ slash_commands.rs        slash parser/action
  │    ├─ acp_session_impl/
  │    │    ├─ slash_exec.rs       slash実行dispatcher
  │    │    ├─ sampler_turn.rs      turnごとのSamplerConfig再構築
  │    │    └─ model_switch.rs      session model切替
  │    └─ acp_session.rs            SessionActor/auth/model state
  │
  ├─ extensions/
  │    ├─ auth.rs                   ACP login/logout
  │    └─ session_admin.rs          model/config reload
  │
  └─ xai-grok-auth / xai-grok-sampler / xai-grok-sampling-types
       ├─ HTTP credential injection/retry
       ├─ sampler実行
       └─ backend/auth scheme型
```

### 2.1 重要な責務分離

| 責務 | source of truth |
|---|---|
| Grok credentialのwire schema | `auth/model.rs` の `GrokAuth` |
| credentialのmemory/disk/refresh | `auth/manager.rs` の `AuthManager` |
| login/logout flow | `auth/flow.rs` |
| credential fileの安全な保存 | `auth/storage.rs` |
| HTTP requestへのGrok credential注入 | `auth/credential_provider.rs` + `xai-grok-auth` |
| model catalog | `agent/models.rs` の `ModelsManager` |
| model definition/routing | `agent/config.rs` の `ModelEntryConfig/ModelInfo/ModelEntry` |
| chat用credential解決 | `agent/config.rs` + `agent/mvp_agent/agent_ops.rs` |
| sampler/backend実行 | `xai-grok-sampler` |
| session model state | `MvpAgent` / `SessionActor` |
| provider表示・共通状態 | 新規 `provider.rs` |

新しいprovider facadeは既存source of truthを複製しない。

---

## 3. 新規ファイル

## 3.1 `crates/codegen/xai-grok-shell/src/provider.rs`

### 役割

- provider IDの正規化
- provider表示情報
- Grok providerの固定registry
- provider auth statusの表示用変換
- provider model summary
- provider単位のlogin/logout/reload facade
- chat可否の状態判定
- providerエラー分類

### 置くべき型

```rust
pub enum ProviderId {
    Xai,
    Unknown,
}

pub struct ProviderDescriptor {
    pub id: ProviderId,
    pub display_name: &'static str,
    pub login_aliases: &'static [&'static str],
}

pub struct ProviderRegistry;

pub struct GrokProviderStatus {
    pub provider_id: ProviderId,
    pub display_name: &'static str,
    pub authenticated: bool,
    pub auth_status: crate::cli_models::AuthStatus,
    pub available_model_count: usize,
    pub model_state: ProviderModelState,
}

pub enum ProviderModelState {
    Unknown,
    Available,
    Empty,
    FetchFailed,
}

pub enum ProviderChatGate {
    Ready,
    NoProvider,
    NotAuthenticated,
    NoAvailableModels,
    ModelFetchFailed,
}
```

実際の `AuthStatus` の細かい分類は既存 `cli_models.rs:9-54` を再利用する。新しいstatus型でtokenやcredential本体を保持しない。

### 置くべき関数・メソッド

```rust
pub fn parse_provider_id(input: &str) -> Result<ProviderId, ProviderError>;

impl ProviderRegistry {
    pub fn all() -> &'static [ProviderDescriptor];
    pub fn descriptor(id: ProviderId) -> &'static ProviderDescriptor;
    pub fn grok(/* shared handles */) -> GrokProvider;
}

impl GrokProvider {
    pub fn id(&self) -> ProviderId;
    pub fn display_name(&self) -> &'static str;
    pub fn auth_status(&self) -> crate::cli_models::AuthStatus;
    pub fn status(&self) -> GrokProviderStatus;
    pub fn available_models(&self) -> Vec<ProviderModelSummary>;
    pub fn available_model_count(&self) -> usize;
    pub fn chat_gate(&self, model_id: Option<&str>) -> ProviderChatGate;
    pub async fn login(&self, request: ProviderLoginRequest)
        -> Result<GrokProviderStatus, ProviderError>;
    pub fn logout(&self, scope: Option<&str>)
        -> Result<GrokProviderStatus, ProviderError>;
    pub async fn on_auth_changed(&self)
        -> Result<GrokProviderStatus, ProviderError>;
    pub fn reload_configured_models(&self, config: Config)
        -> Result<(), ProviderError>;
    pub fn reload_model_cache(&self)
        -> Result<(), ProviderError>;
}
```

上記は設計上の責務を示すAPIであり、既存の所有権・非同期境界に合わせて引数型を確定する。

### `GrokProvider`が所有してはいけないもの

- credential tokenのコピー
- `GrokAuth`の独自cache
- `ModelsManager`とは別のmodel catalog
- `ShellAuthCredentialProvider`の別インスタンスを用途ごとに生成する仕組み
- session actorへの直接参照
- UI出力用のwriter

`GrokProvider`は既存共有 `Arc<AuthManager>` と既存 `ModelsManager` を参照し、状態更新を既存managerへ委譲する。

### `lib.rs`への追加

`crates/codegen/xai-grok-shell/src/lib.rs` のmodule宣言へ `mod provider;` または必要な公開範囲に応じた `pub(crate) mod provider;` を追加する。

今回のprovider APIはshell内部利用が中心のため、crate外へ広く公開しない。不要な公開APIを増やさない。

---

## 4. 既存ファイルごとの変更設計

## 4.1 `auth/storage.rs`

### 現在の責務

- `read_auth_json`
- 空storeとしての読み込み
- corrupt file backup
- atomic write
- owner-only permission
- lock
- API key保存・削除

主な位置:

- `auth/storage.rs:50-85` 読み込み
- `auth/storage.rs:88-169` corrupt backup
- `auth/storage.rs:172-325` atomic write/permission/rollback
- `auth/storage.rs:327-376` API key read/write/clear

### 追加・変更する関数

```rust
pub fn provider_auth_dir(grok_home: &Path) -> PathBuf;
pub fn provider_auth_path(grok_home: &Path, provider: ProviderId) -> PathBuf;
pub fn provider_auth_lock_path(path: &Path) -> PathBuf;
pub fn legacy_grok_auth_path(grok_home: &Path) -> PathBuf;
pub fn read_provider_auth(provider: ProviderId, grok_home: &Path)
    -> io::Result<AuthStore>;
pub fn migrate_legacy_grok_auth_if_needed(
    grok_home: &Path,
    canonical_path: &Path,
) -> io::Result<MigrationResult>;
```

既存の汎用JSON read/write処理は再利用し、provider pathの決定だけを追加する。

### 保存仕様

```text
~/.grok/
  auth/
    grok.json
    grok.json.lock
    grok.json.corrupt.<timestamp>
```

- 親ディレクトリ `~/.grok/auth/` を必要時に作成
- temp file → flush/fsync → renameのatomic write
- owner-only permissionを維持
- provider単位でlock pathを分ける
- corrupt fileは既存と同じbackup/recovery方針
- `auth.json`はGrokのlegacy sourceとしてのみ扱う
- 他providerのファイルをGrokの読み込み対象にしない
- provider間token fallbackを実装しない

### migration仕様

1. canonical `auth/grok.json` が存在すれば、それだけを読む
2. canonicalが存在せず、legacy `auth.json` が存在すればlegacyを読む
3. login状態をUI上はGrokの状態として扱う
4. 次の正常な保存時にcanonicalへatomic writeする
5. canonical保存成功後、legacyを即時削除するかは既存互換性を確認して決定する
6. migration失敗時はcredentialを破棄せず、legacyを保持する
7. `GROK_AUTH_PATH` が指定された場合は既存互換として最優先する

初回実装では、legacyを無条件削除しない。canonical書き込み成功を確認してから扱う。

## 4.2 `auth/manager.rs`

### 現在の責務

- `AuthManager::new` によるauth store/path初期化
- memory credentialの保持
- `current_or_expired`
- `get_valid_token`
- `refresh_chain`
- `update` / `save_without_enrichment`
- `hot_swap` / `clear_in_memory`
- `clear` / `remove_scope`
- refresh lockとcross-process lock

主な位置:

- `auth/manager.rs:261-374` 構築・読み込み
- `auth/manager.rs:437-520` clear/reload
- `auth/manager.rs:523-570` disk state再読み込み
- `auth/manager.rs:779-895` 保存
- `auth/manager.rs:945-955` hot swap
- `auth/manager.rs:1181-1209` refresher設定
- `auth/manager.rs:1240-1471` auth dispatch/refresh

### 変更方針

`AuthManager`はGrok専用のまま維持する。provider共通managerへ改名・一般化しない。

変更は次の二点に限定する。

1. constructorへcanonical provider pathを渡せるようにする
2. storageのpath/lock/migration処理を `auth/storage.rs`へ委譲する

### 追加候補

```rust
pub fn new_for_provider(
    config: &GrokComConfig,
    provider: ProviderId,
) -> Self;

pub fn auth_storage_path(&self) -> &Path;
pub fn provider_id(&self) -> ProviderId;
pub fn reload_provider_auth(&self) -> io::Result<()>;
```

既存の `new` は内部または互換入口として残し、最終的なpathはstorage helperから得る。

### 変更してはいけない境界

- `GrokAuth` schemaをprovider共通schemaへ変換しない
- `refresh_chain`をprovider facadeへ移さない
- `AuthCredentialProvider`へlogin/logout/model APIを追加しない
- xAIのteam pin、OIDC issuer、external auth commandをprovider共通型に混ぜない

## 4.3 `auth/flow.rs`

### 現在の呼び出し関係

```text
run_auth_flow
  -> run_auth_flow_interactive
      -> run_auth_flow_inner
          -> cached credential判定
          -> refresh
          -> external auth/devbox
          -> OIDC/OAuth2/device/loopback
          -> AuthManager::update/save
```

主な位置:

- `auth/flow.rs:400-666` auth flow
- `auth/flow.rs:668-712` non-interactive auth
- `auth/flow.rs:866-942` CLI login
- `auth/flow.rs:944-1031` logout

### 追加候補

```rust
pub async fn run_provider_login(
    provider: ProviderId,
    manager: &AuthManager,
    config: &GrokComConfig,
    request: ProviderLoginRequest,
) -> anyhow::Result<()>;

pub fn run_provider_logout(
    provider: ProviderId,
    manager: &AuthManager,
    scope: Option<&str>,
) -> io::Result<LogoutResult>;
```

内部で `provider != Xai` を拒否する。provider facadeはCLI文字列を直接渡さず、正規化済み `ProviderId` を渡す。

### loginの注意

`run_cli_login` はCLI表示・stderr出力を含むため、provider facadeから常に直接呼ぶのではなく、slash commandの実行環境に適した既存interactive flowへ委譲する。

provider層に次を持ち込まない。

- CLI専用print
- TUI専用render
- session actorの状態更新

login完了後のmodel reloadは、flow終了後にprovider facadeまたはcommand executorが明示的に行う。

### logoutの呼び出し関係

```text
GrokProvider::logout
  -> auth::perform_logout
      -> telemetry identityの消去
      -> AuthManager::clear/remove_scope
      -> storage write/delete
  -> ModelsManager::on_auth_changed
  -> auth method/model state update
  -> x.ai/models/update
```

`perform_logout`の結果を無視しない。credential削除成功とmodel reload成功を別々に分類して表示する。

## 4.4 `auth/credential_provider.rs` と `xai-grok-auth`

### 現在の呼び出し関係

```text
ShellAuthCredentialProvider
  -> AuthManager::current_or_expired
  -> GrokAuthCredentials::apply
  -> xai_grok_auth::HttpAuth

AuthRetryMiddleware
  -> AuthCredentialProvider::refresh_after_unauthorized
  -> ShellAuthCredentialProvider
  -> AuthManager::try_recover_unauthorized
  -> refresh_chain
```

対象:

- `auth/credential_provider.rs:14-89`
- `auth/credential_provider.rs:121-166`
- `xai-grok-auth/src/auth_provider.rs:10-118`
- `xai-grok-auth/src/retry_middleware.rs:11-79`

### 変更方針

`xai-grok-auth::AuthCredentialProvider`はHTTP credential injection/retry専用のまま維持する。

追加しないメソッド:

- `provider_id`
- `display_name`
- `login`
- `logout`
- `available_models`
- `reload_models`
- UI向けerror分類

`GrokAuthCredentials` と `X-XAI-Token-Auth` はGrok provider内部に閉じ込める。将来providerへ流用できる共通型として扱わない。

## 4.5 `agent/config.rs`

### 現在のモデル構造

- `ModelEntryConfig`: `agent/config.rs:3413-3530`
- `ModelInfo`: `agent/config.rs:3691-3831`
- `ModelEntry`: `agent/config.rs:3864-3908`
- `resolve_credentials`: `agent/config.rs:4256-4325`
- `sampling_config_for_model`: `agent/config.rs:4591-4640`
- `inject_url_derived_headers`: `agent/config.rs:4641-4672`
- `resolve_model_to_sampling_config`: `agent/config.rs:4673-4693`

### model/provider ID

`ModelInfo`または内部model metadataに `ProviderId` を持たせる。built-in/remote Grok catalogは明示的に `ProviderId::Xai`、ユーザー定義custom modelと未知のfallbackは安全側の `ProviderId::Unknown` とする。

この変更の目的は、session tokenが暗黙にGrok tokenとみなされる経路を明示化することである。

ACPのmodel IDとは別に次の三つを保持する。

| 値 | 用途 |
|---|---|
| catalog key | ACP `ModelId`、persisted sessionの識別 |
| `ModelInfo.model` | APIへ送るrouting slug |
| `ProviderId` | credential/auth/backendの所有者 |
| `ModelInfo.name` | UI表示名 |

### 追加候補

```rust
pub fn provider_id(&self) -> ProviderId;
pub fn resolve_provider_credentials(
    provider: ProviderId,
    model: &ModelEntry,
    session_key: Option<&str>,
) -> Result<ResolvedCredentials, CredentialResolutionError>;
pub fn model_requires_provider_login(&self) -> bool;
```

`resolve_credentials`は互換入口として残してもよいが、内部でprovider-aware関数へ委譲する。

### 必須条件

- `ProviderId::Xai` のmodelだけが `AuthManager` のGrok tokenを使う
- model固有BYOK credentialの優先順位を維持する
- provider IDが一致しなければGrok session tokenへfallbackしない
- `X-XAI-Token-Auth`、`x-authenticateresponse`、Grok client headersはGrok/xAI endpointだけに追加する
- `xai`以外のmodelへGrokの `AuthManagerBearerResolver` を渡さない

### backend routing

既存 `ApiBackend` はそのまま利用する。

- `xai-grok-sampling-types/src/types.rs:1010-1030`
- `ChatCompletions`
- `Responses`
- `Messages`

provider abstractionのためにsampler crateへxAI判定を追加しない。

## 4.6 `agent/models.rs`

### 現在の責務

- `ModelsManager`構築: `models.rs:97-193`
- catalog生成: `models.rs:218-268`, `1832-1897`
- auth別fetch: `models.rs:19-57`
- available models: `models.rs:372-387`
- auth変更: `models.rs:584-639`
- model update通知: `models.rs:641-664`
- config reload: `models.rs:274-352`
- disk cache reload: `models.rs:666-764`
- sampling fallback: `models.rs:929-963`
- remote fetch: `models.rs:990-1132`, `1970-1981`
- model selection/fallback: `models.rs:1139-1184`, `1671-1774`

### 追加候補

```rust
pub fn has_available_models(&self, provider: ProviderId) -> bool;
pub fn available_model_count(&self, provider: ProviderId) -> usize;
pub fn available_for_provider(
    &self,
    provider: ProviderId,
) -> IndexMap<acp::ModelId, acp::ModelInfo>;
pub async fn on_provider_auth_changed(
    &self,
    provider: ProviderId,
) -> Result<(), ModelReloadError>;
pub fn clear_provider_catalog(&self, provider: ProviderId);
```

### source of truth

provider facadeは `ModelsManager::available()` のcatalogを参照する。独自の `HashMap` やモデル数cacheを追加しない。

### 未login・空catalogの扱い

現状 `ModelsManager::sampling_config` はcatalog空時にbundled defaultへfallbackする。これは既存互換のため残す余地があるが、ユーザーchatの入口では先にprovider gateを実施し、bundled fallbackを実在モデルとして扱わない。

```text
chat送信
  -> provider/chat gate
  -> catalogが空なら明示的に停止
  -> catalogがある場合のみ sampling_config
```

`ModelEntry::fallback`を通る前に `NoAvailableModels` を返す経路を追加する。

### model fetch失敗と未loginの区別

次の状態を `ModelsManager` または provider facadeが区別できるようにする。

- 未認証でfetchしていない
- 認証済みだがcatalogが空
- remote fetchが失敗した
- cacheを使用中
- bundled defaultだけが存在する
- allowlistが全modelを除外した

既存の `allowlist_excludes_all` と `has_fetched_real_catalog` を再利用し、単なる `available().is_empty()` だけで全状態を表現しない。

### reload後の通知

```text
AuthManager::update/hot_swap/clear
  -> ModelsManager::on_auth_changed
  -> auth方式再判定
  -> cache invalidate
  -> model fetch/clear
  -> current model再選択
  -> x.ai/models/update
```

login/logout commandはこの既存経路を一つだけ呼ぶ。command側でcatalogを直接書き換えない。

## 4.7 `agent/app.rs`

### 変更対象

- `agent/app.rs:1191-1273` shared `AuthManager`、`ModelsManager`、`MvpAgent`生成
- `agent/app.rs:1484-1531` auth hot-reload/clear
- `agent/app.rs:1571-1610` model/config cache reload
- `agent/app.rs:441-497` headless auth gate

### 方針

startupで次を行う。

1. `AuthManager`はcredentialなしでも生成
2. `ModelsManager`は空catalogでも生成
3. `MvpAgent::initialize`は空model stateを返せるようにする
4. UI/ACPはlogin案内状態を表示
5. chat時にprovider gateで停止

headlessの `run_auth_flow`、`try_ensure_fresh_auth`、session必須条件は別policyとして保持する。

### provider registryの生成位置

`AuthManager`と`ModelsManager`が共有状態として生成された後、`MvpAgent`へ渡すprovider contextを作る。

```text
create_auth_manager
  -> create_models_manager
  -> ProviderRegistry::grok(shared auth/models/config)
  -> MvpAgent::with_models(...)
```

provider facadeが別のAuthManagerを生成してはいけない。

## 4.8 `agent/mvp_agent/acp_agent.rs`

### 現在の関係

- `initialize`: `acp_agent.rs:314-437`
- model state初期化: `acp_agent.rs:389-393`
- authenticate: `acp_agent.rs:439-464`
- session load時のmodel fallback/latched unavailable state: `acp_agent.rs:1780-1832`
- prompt時のunavailable model gate: `acp_agent.rs:2000-2083`
- set session model: `acp_agent.rs:3118-3143`

### 変更候補

```rust
pub fn provider_status(&self, provider: ProviderId) -> ProviderStatus;
pub fn model_state_without_login_gate(&self) -> SessionModelState;
pub fn chat_gate_for_session(&self, session_id: &SessionId)
    -> ProviderChatGate;
```

initializeではloginがないことをエラーにしない。`modelState`は空でも正常な初期状態として返す。

prompt実行前に次の順で判定する。

1. sessionのmodel IDをcatalog keyへ解決
2. modelのproviderを取得
3. provider statusを取得
4. provider auth stateを確認
5. provider available modelを確認
6. 失敗ならprovider-specific messageを返す
7. Readyの場合のみ通常のsamplingへ進む

既存の「persisted modelがaccountから消えた」latched stateと、今回の「未login」は同じ状態にしない。

## 4.9 `agent/mvp_agent/agent_ops.rs`

### 現在の関係

- `resolve_model_id`: `agent_ops.rs:1056-1082`
- `prepare_sampling_config_for_model`: `agent_ops.rs:1084-1166`
- login/auth後のrefresh関連: `agent_ops.rs:1568-1576`
- `MvpAgent::model_state`: `agent_ops.rs:2237-2275`

### 追加候補

```rust
pub fn resolve_provider_for_model(&self, model_id: &str) -> ProviderId;
pub fn check_chat_readiness(
    &self,
    session_id: &SessionId,
    model_id: &str,
) -> Result<(), ProviderChatError>;
```

`prepare_sampling_config_for_model`内でtokenがないことを単なるwarningとしてsamplerへ進めない。chat gateを前段に置く。

### 正常なchat経路

```text
MvpAgent::prompt
  -> check_chat_readiness
      -> resolve_model_id
      -> resolve ProviderId::Xai
      -> GrokProvider::status
      -> ModelsManager::available_for_provider
  -> prepare_sampling_config_for_model
      -> resolve_provider_credentials
      -> sampling_config_for_model
  -> SessionActor::reconstruct_full_config
  -> SamplerActor::update_config
  -> SamplingClient::conversation_collect
```

## 4.10 `session/slash_commands.rs`

### 現状

調査時点では、既存のbuiltin command一覧・`BuiltinAction`・argument判定に以下が存在しない。

- `/login`
- `/logout`
- `/auth`
- `/providers`
- `/models`

主な位置:

- `session/slash_commands.rs:47-259` command一覧
- `session/slash_commands.rs:608-663` `BuiltinAction`
- `session/slash_commands.rs:665-728` name/argument判定

### 追加するaction

```rust
Login { provider: Option<String> },
Logout { provider: Option<String> },
AuthStatus,
Providers,
Models,
```

### parser仕様

受理する構文:

```text
/login
/login grok
/login xai
/login provider grok
/logout grok
/auth
/auth status
/providers
/models
```

`/login` のprovider省略時は `xai/Grok` を既定値とするか、provider選択案内を表示するかを実装時に固定する。今回providerが一つだけなので、既定値は `xai` とする設計が最小である。

unknown providerはparserでは文字列として受け取り、executorで `ProviderRegistry` に問い合わせてエラーにする。これにより将来provider追加時のparser変更を最小化する。

## 4.11 `session/acp_session_impl/slash_exec.rs`

### 現在の責務

`execute_builtin_slash_command` がactionを実行する。

### 追加するdispatcher関数

```rust
async fn execute_login(
    session: &mut SessionActor,
    provider: Option<String>,
) -> SlashCommandResult;

async fn execute_logout(
    session: &mut SessionActor,
    provider: Option<String>,
) -> SlashCommandResult;

fn execute_auth_status(session: &SessionActor) -> SlashCommandResult;
fn execute_providers(session: &SessionActor) -> SlashCommandResult;
fn execute_models(session: &SessionActor) -> SlashCommandResult;
```

### login成功後の処理順

```text
parse ProviderId
  -> ProviderRegistry::grok
  -> GrokProvider::login
  -> AuthManager state確定
  -> ModelsManager::on_auth_changed
  -> model catalog再取得
  -> auth_method_id再評価
  -> current model再選択
  -> x.ai/models/update
  -> statusを表示
```

login command自身が `AuthManager::new` を呼ばない。

### logout成功後の処理順

```text
parse ProviderId
  -> GrokProvider::logout
  -> AuthManager::clear/remove_scope
  -> ModelsManager::on_auth_changed
  -> provider model state更新
  -> current modelを未選択またはlogin-requiredへ
  -> statusを表示
```

## 4.12 `session/acp_session.rs`

`SessionActor`は既に auth method ID、`AuthManager`、session model状態を保持する。

主な位置:

- `session/acp_session.rs:563-592`

login/logout後に必要な処理:

- shared auth method IDの更新
- sessionの次turnで新credentialを読む
- session modelがproviderのcatalogに存在するか再評価
- logout後に古いsession tokenをsamplerへ渡さない

session actorからproviderの内部credential schemaを直接参照しない。`GrokProviderStatus` と `ProviderChatGate` のみを利用する。

## 4.13 `extensions/auth.rs`

### 現在の責務

- ACP auth method処理
- ACP logout
- `perform_logout`
- logout後の `ModelsManager::on_auth_changed`

主な位置:

- `extensions/auth.rs:119-142`

### 変更方針

既存のACP logoutとslash logoutが別々にcleanupを実装しない。

```text
ACP logout
  -> GrokProvider::logout

slash /logout grok
  -> GrokProvider::logout
```

上記へ寄せ、`ModelsManager::on_auth_changed` を二重に呼ばない。

## 4.14 `extensions/session_admin.rs`

### 現在のreload経路

- `x.ai/internal/reload_models`
- `x.ai/internal/reload_models_cache`
- config reload: `session_admin.rs:544-595`
- cache reload: `session_admin.rs:597-614`

### 変更方針

既存internal requestは維持し、必要ならprovider facadeへ委譲する。

```text
ConfigUpdate::ModelsChanged
  -> x.ai/internal/reload_models
  -> handle_reload_models
  -> GrokProvider::reload_configured_models
  -> ModelsManager::apply_config

ConfigUpdate::ModelsCacheChanged
  -> x.ai/internal/reload_models_cache
  -> handle_reload_models_cache
  -> GrokProvider::reload_model_cache
  -> ModelsManager::reload_from_disk_cache
```

provider facadeはconfig watcher自体を所有しない。

## 4.15 `xai-grok-pager`

認証必須画面がpager/TUIの表示状態である場合、次の箇所を確認・変更する。

- `crates/codegen/xai-grok-pager/src/app/app_view.rs:334-366` `AuthState/AuthMode`
- `app_view.rs:941-951` view state
- `app_view.rs:1095` ready判定
- `app_view.rs:2651` render context
- `app_view.rs:2822` 以降のauth state表示
- `crates/codegen/xai-grok-pager/src/actions/defaults.rs:797-807` model picker action

このcrateへGrok credential処理を移植しない。pagerはshellのprovider statusを受け取り、表示を切り替えるだけにする。

---

## 5. 詳細な依存関係

## 5.1 crate依存

```text
xai-grok-shell
  ├─ xai-grok-auth
  │    ├─ AuthCredentialProvider
  │    ├─ CredentialSnapshot
  │    └─ AuthRetryMiddleware
  │
  ├─ xai-grok-sampler
  │    ├─ SamplerConfig
  │    ├─ BearerResolver
  │    └─ SamplingClient
  │
  ├─ xai-grok-sampling-types
  │    ├─ ApiBackend
  │    └─ AuthScheme
  │
  ├─ xai-grok-models
  │    └─ bundled default model catalog
  │
  └─ xai-file-utils
       └─ storage client / proxy client
```

### 5.1.1 新規provider.rsの依存方向

```text
provider.rs
  -> auth::AuthManager / auth::flow
  -> cli_models::AuthStatus
  -> agent::models::ModelsManager
  -> agent::config::ModelEntry/ProviderId関連

auth::* / ModelsManager / sampler
  -X-> provider.rs
```

provider.rsを下位層から参照させない。これにより循環依存を避ける。

## 5.2 認証呼び出しグラフ

```text
startup
  -> Config::create_auth_manager
  -> AuthManager::new/new_for_provider
  -> storage::read_provider_auth
  -> AuthStore/GrokAuth

login
  -> slash_exec::execute_login
  -> ProviderRegistry::grok
  -> GrokProvider::login
  -> auth::run_auth_flow(_interactive)
  -> OIDC/device/loopback/external/devbox
  -> AuthManager::update/save_without_enrichment
  -> storage::atomic_write
  -> GrokProvider::on_auth_changed
  -> ModelsManager::on_auth_changed

logout
  -> slash_exec::execute_logout
  -> GrokProvider::logout
  -> auth::perform_logout
  -> AuthManager::clear/remove_scope
  -> storage::write/delete
  -> ModelsManager::on_auth_changed

401
  -> xai_grok_auth::AuthRetryMiddleware
  -> AuthCredentialProvider::refresh_after_unauthorized
  -> ShellAuthCredentialProvider
  -> AuthManager::try_recover_unauthorized
  -> AuthManager::refresh_chain
  -> TokenRefresher
  -> AuthManager::update
  -> storage::atomic_write
```

## 5.3 model呼び出しグラフ

```text
ModelsManager::new/from_config
  -> ModelFetchAuth::resolve
  -> prefetch/cache/remote fetch
  -> parse_remote_model_value
  -> resolve_model_catalog
  -> IndexMap<catalog_key, ModelEntry>

ModelsManager::available
  -> provider::GrokProvider::available_models
  -> MvpAgent::model_state
  -> ACP InitializeResponse.meta.modelState

auth changed
  -> ModelsManager::on_auth_changed
  -> model auth再判定
  -> cache invalidate/fetch
  -> reselect_default_model/reselect_current_model_if_missing
  -> notify_models_updated
  -> x.ai/models/update
```

## 5.4 chat呼び出しグラフ

```text
prompt/turn
  -> MvpAgent::check_chat_readiness
  -> resolve_model_id
  -> catalog key/model routing slug解決
  -> ModelEntry.provider_id
  -> GrokProvider::chat_gate
  -> ModelsManager::available_for_provider

Ready
  -> prepare_sampling_config_for_model
  -> resolve_provider_credentials
  -> sampling_config_for_model
  -> SessionActor::reconstruct_full_config
  -> SamplerConfig
  -> SamplerActor::update_config
  -> SamplingClient::conversation_collect
  -> ApiBackend dispatch
```

## 5.5 model IDの呼び出し関係

```text
remote response.id / response.model
  -> parse_remote_model_value
  -> ModelEntryConfig.id/model
  -> ModelInfo
  -> ModelsManager catalog key
  -> ACP ModelId
  -> resolve_catalog_key
  -> sampling_config.model
  -> HTTP request routing slug
```

`ACP ModelId`をそのままAPIのrouting modelとして使わない。既存のcatalog key/routing slug分離を維持する。

---

## 6. provider状態モデル

### 6.1 状態

```text
ProviderRegistry
  └─ Xai/Grok
       ├─ NotAuthenticated
       │    ├─ model fetch未実行
       │    └─ model catalog empty
       │
       ├─ Authenticated
       │    ├─ models available
       │    ├─ models empty
       │    └─ model fetch failed
       │
       └─ Authenticated + stale cache
```

### 6.2 chat gate

| 状態 | chat結果 |
|---|---|
| provider未登録 | `利用可能なproviderがありません。` |
| Grok未login | `選択中のproviderがloginされていません。/login grok` |
| login済み・modelなし | `利用可能なモデルがありません。/login grok` またはmodel取得案内 |
| model fetch失敗 | provider固有のfetch error |
| modelあり・credentialあり | 通常のchat |
| allowlist全除外 | 既存allowlistエラー |
| persisted model消失 | 既存のmodel unavailable/fallback状態 |

未loginとmodel fetch failureを同じ `NoModels` に潰さない。

### 6.3 表示用status

`/auth status` はtokenやpathを表示しない。

```text
Grok  logged in   5 models available
Grok  logged out
Grok  logged in   model list unavailable
```

`available_model_count` は、current auth methodでユーザーが選択可能なmodelだけを数える。raw catalog件数をそのまま表示しない。

---

## 7. 認証情報の安全境界

### 7.1 許可する経路

```text
ProviderId::Xai
  -> GrokAuth/AuthManager
  -> GrokAuthCredentials
  -> XAI-specific auth headers
  -> xAI endpoint
```

### 7.2 禁止する経路

```text
ProviderId::Other
  -X-> AuthManager::current_or_expired
  -X-> GrokAuthCredentials
  -X-> X-XAI-Token-Auth
  -X-> xAI session token fallback
```

### 7.3 具体的な防止策

- `resolve_provider_credentials` はprovider IDを必須引数にする
- model metadataにprovider IDを持たせる
- `AuthManager`はGrok provider adapterからのみ取得する
- samplerへ `AuthManagerBearerResolver` を渡すのは `ProviderId::Xai` の場合だけ
- `inject_url_derived_headers` はxAI endpoint判定を維持する
- auth fileのpathをproviderごとに分離する
- provider未login時に他provider credentialへfallbackしない
- `/auth status`やログにはtoken値を出さない

---

## 8. 変更しないファイル・型

次は今回のprovider login基盤のために一般化・移植しない。

### `xai-grok-auth`

- `AuthCredentialProvider`
- `AuthRetryMiddleware`
- `CredentialSnapshot`

HTTP auth/retryの低レイヤとして維持する。

### `xai-grok-sampler`

- `SamplerConfig`
- `BearerResolver`
- `HeaderInjector`
- `SamplingClient`

provider IDをsamplerへ持ち込まない。shellが正しいconfig/headerを構築する。

### `GrokAuth`

- `auth/model.rs`のGrok固有schemaを維持
- `AuthMode`をprovider共通のlogin modeへ昇格しない
- refresh token、OIDC issuer、team/org情報を共通credentialへ混ぜない

### Codex関連

Codexの型、ファイル、storage名、runtime、App Serverは追加しない。

---

## 9. 実装順序

## Phase 1: provider境界だけを追加

対象:

- 新規 `src/provider.rs`
- `src/lib.rs`

内容:

- `ProviderId::Xai`
- `ProviderDescriptor`
- alias parser
- fixed `ProviderRegistry`
- `GrokProvider` facadeの骨格
- status/model summaryの戻り値

この段階では既存挙動を変えない。

## Phase 2: storage pathとmigration

対象:

- `auth/storage.rs`
- `auth/manager.rs`
- `auth/manager/lock.rs`（必要箇所）
- auth関連テスト

内容:

- `auth/grok.json`
- provider lock
- atomic write
- permission
- corrupt backup
- legacy `auth.json` read/migration
- `GROK_AUTH_PATH`互換

## Phase 3: status/model facade

対象:

- `cli_models.rs`
- `agent/models.rs`
- `agent/config.rs`
- `provider.rs`

内容:

- status変換
- provider別model count
- model/provider ID
- model catalog state
- auth変更後reload委譲

## Phase 4: login/logout command

対象:

- `auth/flow.rs`
- `extensions/auth.rs`
- `session/slash_commands.rs`
- `session/acp_session_impl/slash_exec.rs`
- `session/acp_session.rs`

内容:

- `/login`
- `/login grok`
- `/login provider grok`
- `/logout grok`
- login/logout後のauth method/model reload

## Phase 5: loginなし起動

対象:

- `agent/app.rs`
- `agent/mvp_agent/acp_agent.rs`
- interactive UIのauth gate
- 必要なら `xai-grok-pager`

内容:

- interactive/stdioでcredentialなしinitialize
- empty model state
- startup login screenをlogin案内状態へ変更
- headless policyを壊さない

## Phase 6: chat gate

対象:

- `agent/mvp_agent/acp_agent.rs`
- `agent/mvp_agent/agent_ops.rs`
- `agent/config.rs`
- `agent/models.rs`

内容:

- chat直前のprovider/auth/model判定
- no-model時のfallback抑止
- 未login・モデルなし・fetch失敗の表示分離
- provider token fallback防止

## Phase 7: read-only commandと通知

対象:

- `session/acp_session_impl/slash_exec.rs`
- `agent/models.rs`
- `provider.rs`

内容:

- `/auth`
- `/auth status`
- `/providers`
- `/models`
- login/logout/config reload後の通知

---

## 10. テスト設計

## 10.1 storage

追加・更新テスト:

- canonical `auth/grok.json` が優先される
- legacy `auth.json`から読み込める
- migration成功後もcredentialが保持される
- migration失敗時にlegacyが壊れない
- `grok.json.lock` が使われる
- atomic write中断時に旧ファイルが残る
- owner-only permissionが維持される
- corrupt file backupがprovider path単位になる
- `GROK_AUTH_PATH`互換が維持される
- provider間pathへアクセスしない

## 10.2 provider parser/registry

- `grok` → `ProviderId::Xai`
- `xai` → `ProviderId::Xai`
- 大文字小文字の方針を固定してテスト
- `provider grok`形式をparse
- unknown providerで未登録エラー
- registryはGrokだけを返す
- Codex descriptorが存在しない

## 10.3 auth status

- API key
- logged-in session
- model credentials
- deployment key
- not authenticated
- expired credential
- refresh失敗
- token値が表示出力へ混入しない

既存 `cli_models::AuthStatus::resolve` の優先順位を壊さない。

## 10.4 model state

- credentialなしで空model stateを返す
- login後にmodel catalogが取得される
- logout後にcatalogが更新される
- fetch失敗と未loginが別状態になる
- bundled fallbackがavailable modelとしてstatusに数えられない
- allowlist全除外が既存エラーを返す
- persisted model消失の既存fallback/latched behaviorを壊さない
- ACP `x.ai/models/update`が一度だけ発行される

## 10.5 chat gate

- providerなし
- provider未login
- login済み・modelなし
- login済み・fetch failure
- modelあり
- model固有BYOK
- xAI modelだけがGrok tokenを受け取る
- provider ID不一致時にGrok tokenへfallbackしない
- `X-XAI-Token-Auth`がGrok以外へ付かない

## 10.6 command

```text
/login
/login grok
/login xai
/login provider grok
/logout grok
/auth
/auth status
/providers
/models
```

- login成功後にstatus/model countが更新される
- logout成功後にstatus/model countが更新される
- unknown providerが明示的に失敗する
- login中のcancel/errorがsessionを壊さない
- 二重loginでrefresh/storage lockが破綻しない

## 10.7 起動モード

- interactive: loginなしで起動
- stdio/ACP: empty model stateでinitialize
- headless: 既存session必須仕様の確認
- UI auth state: login画面ではなく案内状態
- startup時にlogin flowが自動開始されない

---

## 11. 受け入れ条件

### 起動

- credentialがない状態でinteractive UIが起動する
- initializeがauth errorで停止しない
- model一覧が空でもUIが表示される
- headlessの既存契約を不用意に変更しない

### login/logout

- `/login` または `/login grok` でGrok loginが開始される
- `/login provider grok`を互換構文として処理できる
- `/logout grok`でGrok credentialが削除される
- login/logout後にauth statusとmodel listが更新される

### storage

- canonical pathが `~/.grok/auth/grok.json`
- atomic write
- owner-only permission
- provider単位lock
- legacy `auth.json`の読み込み/migration
- refresh token競合を既存のlock方針で保護

### model/chat

- loginなしでchatするとlogin案内が出る
- 利用可能modelがない場合に正常な案内が出る
- model fetch failureと未loginが区別される
- 利用可能modelがあれば通常chatできる
- modelのcatalog key/display name/routing slugが混同されない

### provider境界

- 内部ID `xai` と表示名 `Grok`が分離される
- Grok tokenがprovider外へfallbackしない
- xAI専用headerがprovider外へ漏れない
- `AuthCredentialProvider`とsamplerにprovider UI責務を追加しない
- Codex実装が混入しない

---

## 12. リスクと対処

### 12.1 `auth.json`から`auth/grok.json`への移行

最大の変更点。pathだけでなくlock、watcher、API key、refresh、backup、テストが連動する。

対処:

- storage helperを先に作る
- canonical優先・legacy fallbackを明示する
- migration成功前にlegacyを削除しない
- pathを直接組み立てる箇所を全検索して潰す

### 12.2 bundled default fallback

catalogが空でもsampling側がbundled defaultを作るため、「モデルなし」をchat gateより後で判定するとlogin案内が出ない可能性がある。

対処:

- chat gateをsampling config生成より前に置く
- status/model countではbundled fallbackを実在modelとして数えない
- 既存fallbackはauxiliary/互換経路として必要性を確認する

### 12.3 headlessとinteractiveの混同

`agent/app.rs:441-497`を一括削除するとheadlessの認証契約を壊す可能性がある。

対処:

- 起動modeを明示的に分ける
- interactive/stdioのlogin gateだけを変更
- headlessを変更する場合は別テストと別決定を要求する

### 12.4 sessionに古いcredentialが残る

login/logout後もsession actorやsamplerが古いconfigを保持すると、UI上はlogout済みでもリクエストが送られる可能性がある。

対処:

- turnごとに `reconstruct_full_config` を通す
- logout後に `ModelsManager::on_auth_changed` とauth method stateを更新
- `AuthManagerBearerResolver` はGrok provider時だけ有効にする
- logoutテストで次turnが認証案内になることを確認する

### 12.5 provider abstractionの過剰設計

Codex保留なのに動的provider traitやruntime abstractionを導入すると、既存責務が分散する。

対処:

- Grok固定registry
- `GrokProvider` facade
- 既存managerへの委譲
- provider共通型はstatus/model/chat gateに限定

---

## 13. 実装時の最終ファイル一覧

### 新規

- `crates/codegen/xai-grok-shell/src/provider.rs`
- `GROK_PROVIDER_LOGIN_DETAILED_DESIGN.md`（本書）

### 必須変更候補

- `crates/codegen/xai-grok-shell/src/lib.rs`
- `crates/codegen/xai-grok-shell/src/auth/storage.rs`
- `crates/codegen/xai-grok-shell/src/auth/manager.rs`
- `crates/codegen/xai-grok-shell/src/auth/manager/lock.rs`
- `crates/codegen/xai-grok-shell/src/auth/flow.rs`
- `crates/codegen/xai-grok-shell/src/auth/mod.rs`
- `crates/codegen/xai-grok-shell/src/cli_models.rs`
- `crates/codegen/xai-grok-shell/src/agent/app.rs`
- `crates/codegen/xai-grok-shell/src/agent/config.rs`
- `crates/codegen/xai-grok-shell/src/agent/models.rs`
- `crates/codegen/xai-grok-shell/src/agent/mvp_agent/acp_agent.rs`
- `crates/codegen/xai-grok-shell/src/agent/mvp_agent/agent_ops.rs`
- `crates/codegen/xai-grok-shell/src/session/slash_commands.rs`
- `crates/codegen/xai-grok-shell/src/session/acp_session.rs`
- `crates/codegen/xai-grok-shell/src/session/acp_session_impl/slash_exec.rs`
- `crates/codegen/xai-grok-shell/src/session/acp_session_impl/sampler_turn.rs`
- `crates/codegen/xai-grok-shell/src/extensions/auth.rs`
- `crates/codegen/xai-grok-shell/src/extensions/session_admin.rs`

### 条件付き変更

- `crates/codegen/xai-grok-pager/src/app/app_view.rs`
- `crates/codegen/xai-grok-pager/src/actions/defaults.rs`
- `crates/codegen/xai-grok-shell/src/config/reloader.rs`
- `crates/codegen/xai-grok-shell/src/config/watcher.rs`
- `crates/codegen/xai-grok-models/default_models.json`

### 今回変更しない

- `xai-grok-auth`のprovider trait一般化
- `xai-grok-sampler`へのprovider registry追加
- Codex関連ファイル
- Claude/Gemini実装

---

## 14. 最終設計結論

今回の実装は、Grok固有の既存認証・モデル・sampler実装を置き換えるのではなく、次の薄い境界を上位に追加する。

```text
Grok Code
  └─ ProviderRegistry
       └─ ProviderId::Xai / display name: Grok
            └─ GrokProvider facade
                 ├─ AuthManager
                 ├─ auth::flow
                 ├─ provider auth storage
                 ├─ ModelsManager
                 └─ provider-aware chat gate
```

重要な原則は以下である。

1. `xai`は内部ID、`Grok`は表示名、`grok`はlogin入力aliasとして分離する
2. `GrokAuth/AuthManager`はGrok専用のまま維持する
3. credentialのcanonical pathは `~/.grok/auth/grok.json` とする
4. 既存 `auth.json` はlegacy migration対象としてのみ扱う
5. `ModelsManager`をmodel catalogの唯一のsource of truthにする
6. loginなし起動とchat可否を分離する
7. chat可否はGrok loginの有無ではなく、provider/auth/modelの最終状態で判定する
8. 未login・モデルなし・fetch失敗・allowlist全除外を分離する
9. xAI専用headerとGrok tokenをprovider外へ漏らさない
10. `AuthCredentialProvider`とsamplerをprovider UI層へ一般化しない
11. headless auth gateをinteractive login gateと同じ変更で削除しない
12. Codexの型・storage・runtime・commandは今回追加しない

この構成であれば、今回の範囲をGrok provider login基盤に限定しながら、将来providerを追加できる最小限のregistry境界と、認証・モデル・chatの責務分離を確保できる。

---

## 15. 依存関係の再調査結果

依存関係調査を再実行し、crateの実際のCargo.toml、module宣言、use宣言、既存テストを確認した。以下を設計上の確定事項として追加する。

### 15.1 Cargo依存

対象crateのworkspace登録:

- workspace `Cargo.toml:20` — `xai-grok-auth`
- workspace `Cargo.toml:39` — `xai-grok-sampler`
- workspace `Cargo.toml:40` — `xai-grok-sampling-types`
- workspace `Cargo.toml:44` — `xai-grok-shell`

既存の直接依存:

```text
xai-grok-shell
  ├─> xai-grok-auth       (middleware feature有効)
  ├─> xai-grok-sampler
  └─> xai-grok-sampling-types

xai-grok-sampler
  └─> xai-grok-sampling-types

xai-grok-sampling-types
  ├─> xai-grok-compaction
  └─> xai-grok-tools
```

確認したCargo.toml:

- `crates/codegen/xai-grok-auth/Cargo.toml:8-22`
- `crates/codegen/xai-grok-sampler/Cargo.toml:8-26`
- `crates/codegen/xai-grok-sampling-types/Cargo.toml:8-17`
- `crates/codegen/xai-grok-shell/Cargo.toml:59-67`, `:133-150`

`provider.rs`を `xai-grok-shell` 内に追加するだけなら、新規crateや新規外部依存は不要である。既存の以下を再利用できる。

- `std::sync::Arc`
- workspace既存の`async-trait`
- workspace既存の`reqwest`
- 既存の`tokio`
- 既存の`xai-grok-auth`
- shell内の`AuthManager`
- shell内の`ModelsManager`

providerを独立crateへ切り出す場合のみ、`xai-grok-auth`、`reqwest`、`async-trait`、必要に応じて`tokio`をそのcrateへ追加する必要がある。今回は独立crate化しない。

### 15.2 実際の参照方向

`xai-grok-shell`から`xai-grok-auth`への代表的な接続:

- `auth/credential_provider.rs:5-7` — `AuthCredentialProvider`、`CredentialSnapshot`、`HttpAuth`
- `auth/credential_provider.rs:42-88` — `ShellAuthCredentialProvider`
- `auth/credential_provider.rs:121-165` — `StorageClient`へのcredential provider注入
- `remote/client.rs:154`, `:319`
- `agent/feedback_client.rs:413-433`
- `session/acp_session_impl/session_setup.rs:364`
- `session/acp_session_impl/spawn.rs:735`

`xai-grok-shell`から`xai-grok-sampler`への代表的な接続:

- `sampling/mod.rs:12-29` — sampler型のre-export
- `agent/config.rs:12`, `:4591-4611` — `SamplerConfig`構築
- `session/acp_session.rs:71`, `:583`, `:1014` — sampler state保持
- `session/acp_session_impl/sampler_turn.rs:235-251`, `:516-539`
- `session/acp_session_impl/spawn.rs:1038-1045`

`xai-grok-shell`から`xai-grok-sampling-types`への代表的な接続:

- `agent/config.rs:13`
- `agent/models.rs:17`
- `agent/session_config.rs:3`
- `session/two_pass.rs:8`
- `session/storage/mod.rs:15`
- `session/storage/jsonl/mod.rs:425-491`

調査した範囲では、`xai-grok-auth`、`xai-grok-sampler`、`xai-grok-sampling-types`から`xai-grok-shell`への実コード依存は存在しない。したがって、provider具体実装をshell側に置く場合の依存方向は次のまま維持できる。

```text
xai-grok-shell
  ├─> xai-grok-auth
  ├─> xai-grok-sampler
  └─> xai-grok-sampling-types
```

### 15.3 循環依存を避ける配置

`xai-grok-auth`は、shellが実装するcredential providerのためのdependency-inversion seamである。`xai-grok-auth/src/lib.rs:1-13`のtrait/middleware層へ`AuthManager`や`GrokProvider`を追加してはいけない。

禁止する依存:

```text
xai-grok-shell
  -> xai-grok-auth
       -> xai-grok-shell
```

同様に、`xai-grok-sampler`へshellの`AuthManager`を持ち込まない。samplerは`BearerResolver`、`HeaderInjector`、`AuthScheme`など既存抽象だけを扱い、providerの具体認証解決はshell側で行う。

`provider.rs`の安全な依存方向:

```text
provider.rs
  -> crate::auth::AuthManager
  -> crate::auth::flow
  -> crate::agent::models::ModelsManager
  -> crate::agent::config
  -> xai_grok_authの既存trait/middleware
  -> xai_grok_samplerの既存抽象
```

下位crateから`provider.rs`を参照させない。

### 15.4 auth/agent/sessionの実際の構成

認証側:

```text
auth::storage
  -> auth::model

auth::manager
  -> auth::storage
  -> auth::model
  -> auth::refresh
  -> auth::recovery

auth::flow
  -> auth::manager
  -> auth::model
  -> auth::oidc
  -> agent::config
  -> config
  -> remote
```

確認箇所:

- `auth/mod.rs:1-44`
- `auth/storage.rs:5`, `:50`, `:158`, `:193`, `:256-315`
- `auth/manager.rs:13-36`, `:262`, `:300`
- `auth/flow.rs:8-10`, `:313`, `:299-302`

agent側:

```text
agent::config
  -> auth::AuthManager / GrokComConfig
  -> xai_grok_sampler::SamplerConfig

agent::models
  -> agent::config
  -> auth::AuthManager
  -> remote
  -> sampling
  -> xai_grok_sampling_types

agent::app
  -> auth::{AuthManager, AuthMode, GrokAuth, run_auth_flow}
  -> ModelsManager
  -> MvpAgent
```

確認箇所:

- `agent/config.rs:2-12`, `:1808-1810`, `:4591`
- `agent/models.rs:12-17`, `:97-122`, `:151-175`, `:218-247`
- `agent/app.rs:23`, `:455-522`, `:539`, `:190-202`

session側:

```text
SessionActor
  -> AuthManager
  -> ModelsManager
  -> SamplerConfig/SamplerHandle
  -> slash_exec
```

確認箇所:

- `session/acp_session.rs:360-364`, `:579-592`, `:704`, `:1014`
- `session/acp_session.rs:49`, `:1137-1179`
- `session/acp_session_impl/slash_exec.rs:5-13`

`session/slash_commands.rs`は現状、auth実行層に依存しないparser/moduleである。login commandを追加する場合も、parserは`BuiltinAction`を返すだけにし、実行は`slash_exec`またはsession/agentの既存共有stateを通す。

### 15.5 テスト配置の確定候補

優先順位は次の通り。

1. `auth/credential_provider.rs:408`付近
   - `AuthManager`からのbearer取得
   - token-auth header判定
   - snapshot
   - unauthorized recovery委譲

2. `auth/manager_tests.rs:20`以降
   - disk/memory token
   - hot swap
   - expiry
   - scope
   - provider facadeがmanagerの状態を正しく見ること

3. `auth/storage.rs:378-512`
   - atomic write
   - rollback
   - permission
   - provider path migration

4. `auth/flow.rs:1033-1490`
   - login flow成功後のmanager更新
   - login cancel/error
   - cached credential compatibility

5. `session/slash_commands.rs:1102`以降
   - `/login`などのadvertise/resolve
   - `BuiltinAction`への変換
   - availability gate

6. `session/acp_session_impl/slash_exec.rs`またはsession-level test
   - 実際のlogin/logout dispatcher
   - login/logout後のmodel/auth state更新

7. `xai-grok-sampler/tests/`
   - samplerの既存`BearerResolver`、`HeaderInjector`、wire behaviorのみ
   - shellの`AuthManager`をsampler testへ持ち込まない

既存候補:

- `tests/test_sampling_client.rs`
- `session/acp_session_tests/auth_error_no_retry_tests.rs`
- `session/acp_session_impl/sampler_turn.rs`内のsampling/auth tests
- `agent/config.rs`内のsampling config tests

依存関係調査の結果、provider実装の最初のテストは`xai-grok-shell`内のauth/provider境界に置き、sampler crateにはprovider具体実装のテストを追加しない方針とする。
