<a id="project-rules-agentsmd"></a>

# プロジェクトルール（AGENTS.md）

プロジェクトルールを使用すると、プロジェクトやディレクトリごとに Grok を設定できます。リポジトリに AGENTS.md ファイルを配置することで、コーディング規約、ビルド手順、スタイルガイドなど、そのコードベースで作業する際に Grok が従うべき指示を設定できます。

---

<a id="what-are-project-rules"></a>

## プロジェクトルールとは？

プロジェクトルールは、Grok が読み込んでコンテキストに追加する Markdown ファイルです。Grok は、そのディレクトリツリー内のすべての対話で内容に従います。

これはプロジェクトの規約を Grok に伝える主要な仕組みであるため、セッションごとに繰り返し説明する必要はありません。

---

<a id="supported-file-names"></a>

## 対応するファイル名

Grok は各ディレクトリ内で、次のファイル名をこの順序で確認します。

- `Agents.md`
- `Claude.md`
- `CLAUDE.md`
- `CLAUDE.local.md`
- `AGENT.md`
- `AGENTS.md`

Grok はディレクトリ内で一致するすべてのファイルを読み込むため、`AGENTS.md` と `CLAUDE.md` の両方があるフォルダーでは、両方の内容が追加されます。大文字と小文字を区別しないファイルシステムでは、`Agents.md` と `AGENTS.md` のように同じファイルとして解決される名前は重複排除され、1 つとして数えられます。`Claude.md`、`CLAUDE.md`、`CLAUDE.local.md` は、Claude Code のワークフローとの互換性のために対応しています。Claude 互換性が有効な場合（デフォルト）、Grok はホームレベルの `~/.claude/` ディレクトリでもこれらのファイル名を検索し、各ディレクトリ階層で `.claude/CLAUDE.md` と `.claude/CLAUDE.local.md` も確認します。これらは Claude Code がプロジェクトメモリに使用する場所です。Cursor 互換性が有効な場合、ホームレベルの `~/.cursor/` ディレクトリも同様に検索されます。

<a id="rules-directories"></a>

### ルールディレクトリ

AGENTS.md ファイルに加えて、Grok はリポジトリルートから現在の作業ディレクトリまでの各階層（`<dir>`）にあるルールディレクトリ内の `*.md` ファイルを検索します。

| 場所 | 備考 |
|----------|-------|
| `<dir>/.grok/rules/` | 常に検索 |
| `<dir>/.claude/rules/` | Claude 互換性（設定可能） |
| `<dir>/.cursor/rules/` | Cursor 互換性（設定可能） |

Grok はデフォルトで Claude と Cursor のルールディレクトリを検索します。特定ベンダーの検索を無効にするには、設定の `[compat]` セクションにある該当項目、または対応する環境変数を設定します。詳しくは[設定](05-configuration.md#harness-compatibility)を参照してください。

---

<a id="how-discovery-works"></a>

## 検出の仕組み

Grok は次の順序でプロジェクトルールを検索します。

1. **グローバルルール**: `~/.grok/`（すべてのプロジェクトに適用）
2. **リポジトリルール**: git リポジトリ内の場合、リポジトリルートから現在の作業ディレクトリまでのすべてのディレクトリ（両端を含む）
3. **CWD のみ**: git リポジトリ内でない場合、現在の作業ディレクトリのみ

<a id="example"></a>

### 例

次のようなプロジェクト構成があるとします。

```
~/projects/my-app/
  AGENTS.md              # 「TypeScript を使用し、ESLint ルールに従う。」
  src/
    AGENTS.md            # 「関数コンポーネントを優先する。」
    components/
      AGENTS.md          # 「スタイルには CSS modules を使用する。」
```

Grok を `~/projects/my-app/src/components/` で実行すると、3 つのファイルがすべて読み込まれます。指示は累積されるため、Grok はそのすべてを認識します。

<a id="deeper-files-take-precedence"></a>

### 深い階層のファイルが優先される

Grok はリポジトリルートから現在の作業ディレクトリまでの順にファイルを並べるため、深いディレクトリのファイルほどコンテキスト内で後に配置され、指示が競合した場合に優先されます。上の例で、ルートに「styled-components を使用する」と記載され、`components/AGENTS.md` に「CSS modules を使用する」と記載されている場合、後に配置される CSS modules の指示が優先されます。

<a id="auto-loading-behavior"></a>

### 自動読み込みの動作

- Grok はセッション開始時に、リポジトリルートから現在の作業ディレクトリまでのファイルを自動的に読み込みます。
- Grok が最初の対象範囲外にあるディレクトリのファイルを読み取り、一覧表示、または編集すると、その場所にあるプロジェクト指示ファイルを検出してパスを記録し、タスクに適用される場合に読み込みます。

---

<a id="what-to-put-in-project-rules"></a>

## プロジェクトルールに記載する内容

<a id="coding-conventions"></a>

### コーディング規約

```markdown
# コーディング標準

- すべての新しいコードに TypeScript を使用する
- クラスコンポーネントより、フックを使った関数コンポーネントを優先する
- デフォルトで `const` を使用し、再代入が必要な場合のみ `let` を使用する
- 1 行の最大長: 100 文字
```

<a id="build-and-test-instructions"></a>

### ビルドとテストの手順

```markdown
# ビルドとテスト

- コミット前に `npm test` を実行する
- コードスタイルの確認には `npm run lint` を使用する
- `npm run build` でビルドし、TypeScript エラーがないことを確認する
- 統合テスト: `npm run test:e2e`（Docker が必要）
```

<a id="style-guides"></a>

### スタイルガイド

```markdown
# スタイルガイド

- Airbnb JavaScript Style Guide に従う
- 2 スペースのインデントを使用する
- 複数行の配列やオブジェクトでは必ず末尾にカンマを付ける
- 文字列連結よりテンプレートリテラルを優先する
```

<a id="pr-and-commit-requirements"></a>

### PR とコミットの要件

```markdown
# バージョン管理

- コミットメッセージは conventional commits 形式で記述する
- ブランチ名の先頭に `feature/`、`fix/`、または `chore/` を付ける
- すべての PR はマージ前に少なくとも 1 件の承認を必要とする
- 機能ブランチは squash merge する
```

<a id="architecture-notes"></a>

### アーキテクチャに関する注意

```markdown
# アーキテクチャ

- API ルートは `src/routes/` にリソースごとに 1 ファイルで配置する
- ビジネスロジックは `src/services/` に配置する
- データベースクエリは `src/repositories/` に配置する
- `src/services/` では `src/routes/` から決してインポートしない
```

---

<a id="scoping-rules-to-subdirectories"></a>

## サブディレクトリへのルール適用範囲

AGENTS.md ファイルは、そのファイルがあるフォルダーをルートとするディレクトリツリー全体に適用されます。これを利用して、コードベースの部分ごとに異なる指示を設定できます。

```
my-monorepo/
  AGENTS.md                    # モノレポ全体のルール
  packages/
    frontend/
      AGENTS.md                # 「React を使用し、CSS modules を優先する。」
    backend/
      AGENTS.md                # 「Express を使用し、REST の規約に従う。」
    shared/
      AGENTS.md                # 「このパッケージにはフレームワーク固有のコードを含めない。」
```

---

<a id="session-rules-flags"></a>

## セッションルールのフラグ

ファイルを編集せず、1 回のセッションだけにルールを追加するには、`--rules`（別名 `--append-system-prompt`）を指定します。

```bash
grok --rules "Always use TypeScript. Prefer functional components."
```

Grok はこのテキストをセッションのシステムプロンプトに追加します。セッション固有のカスタマイズに使用してください。

システムプロンプト全体を置き換えるには、`--system-prompt-override`（別名 `--system-prompt`）を指定します。Grok はテキストをそのまま使用し、デフォルトのシステムプロンプトと `--rules` の両方を無視します。（一方、`--rules` で渡したテキストは `<human_rules>` ブロックで囲まれ、デフォルトのプロンプトに追加されます。）

---

<a id="file-size"></a>

## ファイルサイズ

Grok は各プロジェクト指示ファイルを全文読み込みます。文字数の上限や切り捨てはありません。それでも、指示は簡潔で焦点を絞ったものにしてください。短く具体的なルールは長いルールよりも Grok が従いやすく、読み込むファイルはすべてコンテキストを消費します。

---

<a id="gitignore-filtering"></a>

## Gitignore による除外

`.gitignore` で無視されるファイルは、検出時にスキップされます。個人用の上書きを共有リポジトリに含めないようにするには、`CLAUDE.local.md` のような認識されるファイル名を gitignore に追加します。

```gitignore
# .gitignore
CLAUDE.local.md
```

最上位の指示ファイルとして Grok が検出するのは、[対応するファイル名](#supported-file-names)に記載された名前だけであり、`AGENTS.local.md` や `notes.md` のような独自の名前は検出されません。（`.grok/rules/` のようなルールディレクトリ内では、名前に関係なくすべての `*.md` ファイルが読み込まれます。）

---

<a id="the-grok-project-directory"></a>

## .grok/ プロジェクトディレクトリ

AGENTS.md ファイルに加えて、プロジェクトルートの `.grok/` ディレクトリには、プロジェクトレベルの追加設定を格納できます。

| パス | 用途 |
|------|---------|
| `.grok/config.toml` | プロジェクト単位の MCP サーバー、プラグイン、権限ルール（その他の設定は `~/.grok/config.toml` からのみ読み込まれます） |
| `.grok/skills/` | プロジェクト単位のスキル定義 |
| `.grok/plugins/` | プロジェクト単位のプラグイン |
| `.grok/agents/` | プロジェクト単位のエージェント定義 |
| `.grok/hooks/` | プロジェクト単位のライフサイクルフック |
| `.grok/lsp.json` | LSP サーバー設定 |

これらはすべて任意です。それぞれの詳細については、対応するガイドを参照してください。

---

<a id="inspecting-loaded-rules"></a>

## 読み込まれたルールの確認

読み込まれたすべてのプロジェクト指示を確認するには、`grok inspect` を使用します。

```bash
grok inspect
```

検出された各プロジェクト指示ファイルについて、そのパスと概算トークン数が表示されます。Grok がルールを認識していることの確認に使用してください。

---

<a id="best-practices"></a>

## ベストプラクティス

1. **ルートから始める。** 最も重要なプロジェクト全体のルールを、リポジトリルートの AGENTS.md に配置します。

2. **具体的にする。** 「モダンな JavaScript を使用する」より「TypeScript を使用する」の方が適切です。「コードを整形する」より「コミット前に `cargo fmt` を実行する」の方が適切です。

3. **短く保つ。** 長い指示より簡潔な指示の方が従われやすくなります。

4. **大規模なリポジトリではサブディレクトリの適用範囲を使用する。** モノレポの各部分には異なる規約がある場合があります。ディレクトリごとの AGENTS.md を使用し、ルールの適用範囲を適切に限定してください。

5. **ルールをバージョン管理する。** チーム全体が利用できるように、AGENTS.md をリポジトリへコミットします。ユーザー固有の上書きは `~/.grok/`（グローバルルール）に配置します。

6. **ドキュメントを重複させない。** AGENTS.md には、プロジェクトの README のコピーではなく、実行可能な指示を記載してください。必要に応じて外部ドキュメントへリンクします。

7. **定期的に見直す。** プロジェクトの進化に合わせ、現在の規約に一致するようルールを更新してください。
