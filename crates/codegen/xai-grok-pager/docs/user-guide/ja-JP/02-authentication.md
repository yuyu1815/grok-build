<a id="authentication"></a>

# 認証

Grok は、対話型のブラウザーログイン、エンタープライズシングルサインオン（SSO）、ヘッドレス CI/CD ランナーなど、複数の認証方法に対応しています。

---

<a id="browser-login-default"></a>

## ブラウザーログイン（デフォルト）

初回起動時、Grok は grok.com で認証するためにブラウザーを開きます。

```bash
grok
```

Grok は認証情報を `~/.grok/auth.json` に保存し、セッションをまたいで再利用します。アクセストークンはバックグラウンドで自動的に更新されます。トークンを更新できない場合、Grok は再度サインインするよう求めます。サーバーから有効期限が指定されていない認証情報には、30 日間の有効期間が適用されます。

<a id="re-authenticate"></a>

### 再認証

アカウントを切り替える場合や認証の問題を解決する場合は、次を実行します。

```bash
grok login
```

`grok login` を実行するとサインインフローが再開され、キャッシュ済みセッションが置き換えられます。デフォルトではブラウザーが開き、`auth.x.ai` の SpaceXAI OAuth を通じてサインインします。別のフローを選択するにはフラグを指定します。

| フラグ | 説明 |
|------|-------------|
| `--oauth` | `auth.x.ai` の SpaceXAI OAuth を通じてサインインします。これがデフォルトのため、フラグは省略できます。 |
| `--device-auth`（別名 `--device-code`） | ヘッドレス環境やリモート環境向けのデバイスコードフローでサインインします。 |

サインアウトするには `grok logout` を実行します。フラグはなく、キャッシュ済みの認証情報が消去されます。

---

<a id="api-key"></a>

## API キー

CI/CD、自動化、ブラウザーにアクセスできない環境では、[console.x.ai](https://console.x.ai) で取得した API キーを使用します。

```bash
export XAI_API_KEY="xai-..."
grok
```

Grok は、有効なセッショントークンがない場合のフォールバックとして API キーを使用します。すでに対話形式でサインインしている場合は、保存済みのセッショントークンが優先されます。API キーへフォールバックするには、`grok logout` を実行するか `~/.grok/auth.json` を削除します。

---

<a id="oidc-customer-sso"></a>

## OIDC（顧客 SSO）

grok.com の代わりに、Okta、Azure AD、Auth0 などの独自の Identity Provider（IdP）を通じて開発者を認証します。

<a id="1-register-a-public-client-in-your-idp"></a>

### 1. IdP にパブリッククライアントを登録する

- グラントタイプ: Authorization Code with PKCE（Proof Key for Code Exchange）
- リダイレクト URI: `http://127.0.0.1/callback` -- ループバックアドレスです。Grok はサインイン時にランダムなポートをバインドし、ほとんどの IdP は [RFC 8252](https://tools.ietf.org/html/rfc8252) に従ってループバックリダイレクトをポート非依存として扱います。
- クライアントシークレットは不要です。PKCE がその役割を担います。

<a id="2-configure-the-cli"></a>

### 2. CLI を設定する

設定ファイルを使用する場合:

```toml
# ~/.grok/config.toml
[grok_com_config.oidc]
issuer = "https://acme.okta.com"
client_id = "0oa1b2c3d4e5f6g7h8i9"
```

または、環境変数を使用します。

```bash
export GROK_OIDC_ISSUER="https://acme.okta.com"
export GROK_OIDC_CLIENT_ID="0oa1b2c3d4e5f6g7h8i9"
```

API エンドポイントを上書きして、独自のプロキシを指定することもできます。

```bash
export GROK_CLI_CHAT_PROXY_BASE_URL="https://grok-proxy.acme.com/v1"
```

<a id="3-run-grok"></a>

### 3. `grok` を実行する

CLI は `{issuer}/.well-known/openid-configuration` からエンドポイントを検出し、IdP のログインページを開いて、トークンを `~/.grok/auth.json` に保存します。トークンは、保存された `refresh_token` を使って自動的かつ非表示に更新されます。

<a id="optional-fields"></a>

### オプションフィールド

| フィールド | デフォルト | 備考 |
|-------|---------|-------|
| `scopes` | `["openid", "profile", "email", "offline_access", "api:access"]` | `offline_access` により、ユーザー操作なしのトークン更新が有効になります |
| `audience` | None | 一部の IdP（Auth0 など）で必要です |

---

<a id="external-auth-provider"></a>

## 外部認証プロバイダー

ブラウザーベースのログインができない場合（サンドボックス化された VM、CI ランナー、エアギャップネットワークなど）は、認証を外部のバイナリまたはスクリプトに委任します。

<a id="how-it-works"></a>

### 仕組み

```
+--------------+     sh -c     +------------------------+
|     Grok     |-------------->|  your auth binary      |
|              |               |                        |
|  reads       |<-- stdout ----|  prints token          |
|  auth.json   |               |                        |
|              |   (stderr)    |  prints status/URLs    |--> surfaced to user
+--------------+               +------------------------+
```

1. Grok は `sh -c "<command>"` を介してコマンドを実行します
2. バイナリは必要な認証フロー（SSO、デバイスコード、証明書交換）を実行します
3. **stderr** には、ログイン URL やステータスメッセージなど、人が読める出力を書き込みます。Grok は stderr を読み取ってユーザーに表示します。TUI では、最初の `https://` URL がクリック可能なサインインリンクになります。
4. **stdout** は Grok によって取得され、アクセストークンとして保存されます
5. 終了コード 0 は成功です。0 以外の場合、Grok は対話型ログインへフォールバックします

<a id="the-stdout-stderr-contract"></a>

### stdout / stderr の規約

| ストリーム | 出力する内容 | 閲覧者 |
|--------|---------------|-------------|
| **stdout** | トークンのみ -- ほかには何も出力しないでください | Grok（解析され、auth.json に保存されます） |
| **stderr** | ログイン URL、ステータスメッセージ、エラー | ユーザー（Grok が stderr を読み取り、TUI ではサインイン URL をクリック可能なリンクとして表示します） |

**トークン以外は stdout に何も出力しないでください。** 進捗メッセージやデバッグ出力も禁止です。Grok は stdout を読み取り、前後の空白を除去して、結果をトークンとして解析します。

<a id="stdout-token-format"></a>

### stdout のトークン形式

**単純な文字列** -- 生のトークンのみ:

```
eyJhbGciOiJSUzI1NiIs...
```

**JSON** -- オプションのリフレッシュトークン、有効期限、発行者を含む形式:

```json
{"access_token": "eyJhbGciOi...", "refresh_token": "ref-tok", "expires_in": 3600, "issuer": "https://idp.example.com"}
```

トークンに有効期限があり、期限前に Grok からバイナリを自動的に再実行させたい場合は JSON を使用します。

JSON フィールド:

| フィールド | 必須 | 意味 |
|-------|----------|---------|
| `access_token` | はい | Grok が xAI API に送信する Bearer トークン |
| `refresh_token` | いいえ | 参照用に保存されます。Grok は OAuth のリフレッシュグラントではなく、バイナリを再実行して更新します |
| `expires_in` | いいえ | トークンの有効期間（秒）。期限前の先行更新を有効にします |
| `issuer` | いいえ | トークンの発行者を識別します |

<a id="configuration"></a>

### 設定

設定ファイルを使用する場合:

```toml
# ~/.grok/config.toml
[auth]
auth_provider_command = "/usr/local/bin/my-auth-provider"
auth_provider_label = "Acme Corp"   # 任意 -- TUI のログインボタンをカスタマイズ
auth_token_ttl = 3600               # 任意 -- トークンの有効期間（秒）
```

または、環境変数を使用します。

```bash
export GROK_AUTH_PROVIDER_COMMAND="/usr/local/bin/my-auth-provider"
export GROK_AUTH_PROVIDER_LABEL="Acme Corp"
export GROK_AUTH_TOKEN_TTL=3600
```

<a id="token-refresh"></a>

### トークンの更新

Grok が期限切れトークンを更新する必要がある場合、環境変数に `GROK_AUTH_EXPIRED=1` を設定してバイナリを再実行します。実行のたびに保存済みの認証情報がすべて置き換えられるため、更新時を含むすべての呼び出しで、`issuer` など同じ JSON フィールドを出力してください。バイナリはこの変数を利用して、より高速な非対話型の更新処理を実行できます。

```bash
#!/bin/sh
if [ "$GROK_AUTH_EXPIRED" = "1" ]; then
    echo "Refreshing token..." >&2
    TOKEN=$(my-company-auth --refresh --silent)
else
    echo "Authenticating via Acme Corp SSO..." >&2
    TOKEN=$(my-company-auth --login --interactive)
fi

if [ -z "$TOKEN" ]; then
    echo "Authentication failed" >&2
    exit 1
fi

echo "{\"access_token\": \"$TOKEN\", \"expires_in\": 3600}"
```

<a id="environment-variables"></a>

### 環境変数

| 変数 | 説明 |
|----------|-------------|
| `GROK_AUTH_PROVIDER_COMMAND` | 認証バイナリへのパス |
| `GROK_AUTH_PROVIDER_LABEL` | TUI のログイン画面に表示する名前（例: "Acme Corp"） |
| `GROK_AUTH_TOKEN_TTL` | トークンの有効期間（秒、`expires_in` のない単純な文字列形式のトークン用） |
| `GROK_AUTH_EXPIRED` | トークン更新のためにバイナリを再実行するとき、Grok が `1` に設定します |
| `GROK_AUTH_EARLY_INVALIDATION_SECS` | 期限前に先行更新する秒数（デフォルト: 300） |

---

<a id="device-code-flow"></a>

## デバイスコードフロー

ローカルでブラウザーを使用できないヘッドレス環境（SSH セッション、Docker コンテナ、リモート VM）では、次を実行します。

```bash
grok login --device-auth    # または: grok login --device-code
```

ターミナルに URL とコードが表示されます。任意のデバイスで URL を開き、コードを入力して認証を完了してください。Grok はログインが確認されるまでポーリングします。

デバイスコードフローを[外部認証プロバイダー](#external-auth-provider)経由で実装し、完全に制御することもできます。

---

<a id="automatic-credential-refresh"></a>

## 認証情報の自動更新

Grok は期限切れの認証情報を自動的に更新します。

- **期限前:** 認証プロバイダーが `expires_in`（JSON 出力）を返した場合、または `auth_token_ttl` を設定した場合、Grok は期限の約 5 分前に認証バイナリを再実行します。
- **認証エラー時:** サーバーが 401 Unauthorized を返した場合、Grok は認証情報を更新してリクエストを再試行します。
- **OIDC:** `refresh_token` が利用可能な場合、Grok はブラウザーを再度開かずに IdP 経由で更新します。

更新の猶予時間を調整するには、次のように設定します。

```bash
# 期限の 5 分前に更新（デフォルト）
export GROK_AUTH_EARLY_INVALIDATION_SECS=300

# 先行更新の猶予時間を無効化: 期限到達時または 401 発生時に更新（0 に設定）
export GROK_AUTH_EARLY_INVALIDATION_SECS=0
```

---

<a id="hot-reload"></a>

## ホットリロード

Grok は `~/.grok/auth.json` の変更を自動的に反映します。外部から認証情報を更新した場合（新しいトークンを書き込むスクリプトを使用した場合など）、Grok は再起動せず、次の API 呼び出しから新しい認証情報を使用します。

---

<a id="auth-precedence"></a>

## 認証の優先順位

Grok はリクエストごとに、優先度の高いものから次の順序で認証情報を解決します。

1. **モデルごとの `api_key` または `env_key`** -- `config.toml` の `[model.<name>]` で設定します。存在する場合は常に最優先されます。
2. **有効なセッショントークン** -- ブラウザー、OIDC/OAuth2、外部プロバイダーのログインで取得され、`~/.grok/auth.json` に保存されます。
3. **`XAI_API_KEY`** -- 有効なセッショントークンがない場合のフォールバックです。

複数のログインフローが設定されている場合、Grok は優先度の高いものから次の順序で、最初に利用可能なソースを使ってセッショントークンを設定します。

1. **外部認証プロバイダー**（`auth_provider_command`）
2. **エンタープライズ OIDC** -- `config.toml` の `[grok_com_config.oidc]`、または環境変数 `GROK_OIDC_ISSUER` と `GROK_OIDC_CLIENT_ID` を通じて OIDC が設定されている場合
3. **SpaceXAI OAuth2 ブラウザーログイン** -- デフォルト

セッション中は、有効な認証方法がセッション途中のすべての更新を処理します。

---

<a id="troubleshooting"></a>

## トラブルシューティング

<a id="debug-logging"></a>

### デバッグログ

`RUST_LOG` を設定すると、ファイルログとヘッドレスモードの stderr 出力の詳細度を制御できます。（TUI の画面上のトレースペインは固定フィルターを使用し、`RUST_LOG` を無視します。）TUI のファイルログはデフォルトで `DEBUG` です。ヘッドレスモード（`-p`）では回答だけを出力するため、`RUST_LOG` のデフォルトは `off` です。stderr にログを表示するには、`RUST_LOG=error`（またはより広いレベル）を設定してください。

TUI では、`GROK_LOG_FILE` に絶対パスを設定すると、そのファイルにログを書き込めます。

```bash
GROK_LOG_FILE=/tmp/grok.log RUST_LOG=debug grok
tail -f /tmp/grok.log
```

`GROK_LOG_FILE` はリテラルのファイルパスとして扱われます。`1` のような相対値を指定すると、現在のディレクトリに `1` という名前のファイルが書き込まれます。

ヘッドレスモードでは、ログは stderr に出力されます。ファイルへリダイレクトするには、次を実行します。

```bash
RUST_LOG=debug grok -p "hello" 2> /tmp/grok.log
```

<a id="common-log-messages"></a>

### よくあるログメッセージ

| ログメッセージ | 意味 |
|-------------|---------------|
| `auth: running external auth provider` | Grok が認証バイナリを実行しています |
| `auth: external auth provider returned fresh token` | Grok がトークンを解析して保存しました |
| `auth: external auth provider failed` | バイナリが 0 以外で終了したか、stdout が空でした |
| `auth: external auth provider timed out (likely needs interactive auth), killing` | バイナリがタイムアウトまでに終了しなかったため、強制終了されました |
| `auth: failed to start external auth provider` | コマンドを起動できませんでした（バイナリが見つからないなど） |

<a id="common-fixes"></a>

### よくある解決方法

- **"Authentication failed"** -- `grok logout` を実行してキャッシュ済みの認証情報を消去し、`grok login` で再度サインインします。
- **トークンの有効期限が短すぎる** -- `auth_token_ttl` を設定するか、認証プロバイダーの JSON 出力で `expires_in` を返します。
- **OIDC リダイレクトが失敗する** -- IdP でループバックリダイレクト URI（`http://127.0.0.1/callback`）が許可されていることを確認します。
- **外部認証プロバイダーが見つからない** -- `auth_provider_command` のパスが正しく、バイナリが実行可能であることを確認します。
