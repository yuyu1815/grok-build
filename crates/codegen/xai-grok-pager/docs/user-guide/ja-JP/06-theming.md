<a id="theming-and-appearance-customization"></a>

# テーマと外観のカスタマイズ

Grok Build の TUI で使われるすべての色は、中央のテーマで管理されます。Grok の実行中にテーマを切り替えたり、OS のライト / ダーク表示に追従させたり、設定ファイルでスクロールバックのレイアウト、アニメーション、ブロックのスタイルを調整したりできます。

---

<a id="available-themes"></a>

## 利用可能なテーマ

Grok には 5 つの組み込みテーマと、システムの外観に追従する `auto` オプションがあります。

| テーマ | 設定名 | 説明 | Truecolor が必要 |
|-------|-------------|-------------|--------------------|
| **GrokNight** | `groknight`, `grok-night`, `dark` | マゼンタをアクセントにしたニュートラルなダークテーマ。デフォルトのテーマです。256 色および 16 色のターミナルでもきれいに量子化されます。 | いいえ |
| **GrokDay** | `grokday`, `grok-day`, `light`, `day` | 明るいターミナル背景向けのライトテーマ。 | いいえ |
| **TokyoNight** | `tokyonight`, `tokyo-night`, `tokyo` | Tokyo Night パレットを使用した、青みのある暗い背景。量子化すると本来の特色が失われます。 | はい |
| **RosePineMoon** | `rosepine`, `rose-pine`, `rosepine-moon`, `rose-pine-moon` | Rosé Pine ファミリーの、モーブをアクセントにした落ち着いたダークパレット。 | はい |
| **OscuraMidnight** | `oscura`, `oscura-midnight` | 紫をアクセントにした深いダークテーマ。 | はい |

テーマ名では大文字と小文字を区別しません。`auto` オプション（別名 `system`）については、[自動テーマ（システムの外観）](#auto-theme-system-appearance)を参照してください。

<a id="minimal-mode-has-no-theming"></a>

### Minimal モードではテーマを使用しない

**Minimal モード**（`--minimal`）では、常にターミナルネイティブの固定パレット 1 種類で描画され、`theme` 設定は完全に無視されます（フル TUI には引き続き適用されます）。Minimal はターミナル自体の背景へ直接描画するため、ターミナルのデフォルトの前景色 / 背景色と 16 色の ANSI パレット（`git` や `ls` と同じ色）を使用します。そのため、検出や設定を行わなくても、ライト / ダークどちらのターミナルプロファイルでも読みやすく表示されます。Minimal モードでは、`/theme` と `/settings` のテーマ行は利用できません。

---

<a id="switching-themes"></a>

## テーマの切り替え

<a id="in-the-tui"></a>

### TUI で切り替える

`/theme` スラッシュコマンド（別名 `/t`）を実行して、テーマピッカーを開きます。矢印キーでリスト内を移動すると、各テーマがリアルタイムでプレビューされます。Enter キーを押すと選択内容を適用して保存し、Escape キーを押すと元に戻ります。

ピッカーを使わずに切り替えるには、名前を直接指定します。

```
/theme tokyonight
```

ピッカーで選択せずに `/theme` だけを送信すると、次のテーマへ切り替わります。

<a id="via-config-file"></a>

### 設定ファイルで切り替える

`~/.grok/config.toml` でテーマを設定します。

```toml
[ui]
theme = "tokyonight"
```

---

<a id="auto-theme-system-appearance"></a>

## 自動テーマ（システムの外観）

`theme = "auto"` を設定すると、Grok が OS のライト / ダーク表示に追従し、テーマを自動的に切り替えます。

```toml
[ui]
theme = "auto"
```

デフォルトでは、ダークモードは **GrokNight**、ライトモードは **GrokDay** に対応します。`auto_dark_theme` と `auto_light_theme` でそれぞれの対応を上書きできます。

```toml
[ui]
theme = "auto"
auto_dark_theme = "tokyonight"
auto_light_theme = "grokday"
```

`theme = "system"` は `theme = "auto"` の別名です。

<a id="how-detection-works"></a>

### 検出方法

| プラットフォーム | 方法 |
|----------|--------|
| **macOS** | `AppleInterfaceStyle` システム環境設定を読み取る |
| **Linux** | XDG Desktop Portal（`org.freedesktop.appearance.color-scheme`）へ問い合わせる |
| **Windows** | システムの個人用設定レジストリを読み取る |
| **SSH / ヘッドレス** | 起動時に OSC 11 でターミナルの背景色を問い合わせる方法へフォールバックする |

起動後、Grok は 5 秒ごとに外観の変更をポーリングします。OS のライト / ダークモードを切り替えると、再起動せずに数秒以内で反映されます。

<a id="via-the-settings-pane"></a>

### 設定ペインで設定する

`/settings`（別名 `/config`）を実行し、**Appearance** カテゴリを開くと、**Auto dark theme** と **Auto light theme** を対話形式で設定できます。`/theme` ピッカーで `auto` を選択すると、これらの対応を使用する自動モードが有効になります。

---

<a id="color-support-detection"></a>

## 色サポートの検出

Grok は起動時に、ターミナルの色機能レベルを検出します。

| レベル | 説明 | 検出方法 |
|-------|-------------|-----------|
| **Truecolor**（24 ビット） | フル RGB カラー。すべてのテーマが設計どおりに描画されます。 | `COLORTERM=truecolor` または同等のターミナル機能 |
| **256 色** | インデックスパレット。RGB 値は最も近いパレット項目にマッピングされます。 | 標準の xterm-256color |
| **16 色** | ANSI 名のみ。色は最も近い ANSI カラーにマッピングされます。 | 基本的なターミナルサポート |

`NO_COLOR` を設定すると、Grok は色を出力せず、モノクロで描画します。

`/terminal-setup` を実行すると、検出されたレベル（`color` 行）と、このターミナルでピッカーに表示されるテーマ（`themes` 行）を確認できます。Truecolor が利用できない場合、問題セクションに、有効化する方法（または Terminal.app では有効化できないこと）が表示されます。

<a id="automatic-quantization"></a>

### 自動量子化

すべてのテーマはフル RGB 値で定義されています。Grok は起動時に、検出した機能レベルに合わせてすべての色を量子化します。つまり、次のように処理されます。

- **Truecolor** ターミナルでは、色は変更されません。
- **256 色**ターミナルでは、各 RGB 値が最も近いインデックスパレット項目にマッピングされます。
- **16 色**ターミナルでは、色が ANSI 名にマッピングされます。

GrokNight と GrokDay は、きれいに量子化されるニュートラルグレーを使用します。TokyoNight、RosePineMoon、OscuraMidnight は、量子化すると本来の特色が失われる独特な色合いの背景を使用しているため、Truecolor 非対応のターミナルではテーマピッカーに表示されません。

<a id="runtime-generated-colors"></a>

### 実行時に生成される色

実行時に生成される色（シンタックスハイライト、背景のブレンド）も同じパイプラインで量子化されるため、すべての種類のターミナルで一貫した外観になります。

---

<a id="cursor-color"></a>

## カーソルの色

Grok は、アクティブな Grok セッションであることを示すため、OSC 12 エスケープシーケンスを使用して、ターミナルカーソルを現在のテーマの `accent_user` 色に設定します。カーソルの色は次のように処理されます。

- 起動時とテーマ切り替え時に適用されます。
- 終了時に OSC 112 を介してターミナルのデフォルトへ戻されます。

OSC 12 に対応するターミナル（最新のターミナルの多く）で動作します。

---

<a id="compact-mode"></a>

## コンパクトモード

`/compact-mode` スラッシュコマンドでコンパクトモードを切り替えます。コンパクトモードでは、次の変更が行われます。

- 外側の垂直パディングを削除します（上下の余白が 0 になります）。
- 水平パディングを最小（1 列）に減らします。
- プロンプト領域と情報ブロックの上部パディングを減らします。

設定は `~/.grok/config.toml` の `[ui].compact_mode` に保存され、再起動後も維持されます。

小さい画面でコンテンツ領域を最大化するには、コンパクトモードを使用してください。

---

<a id="syntax-highlighting"></a>

## シンタックスハイライト

Grok には、コードブロックのシンタックスハイライト用に 3 つの `.tmTheme` ファイルが組み込まれており、アクティブなテーマに応じて 1 つが選択されます。

- `grok-night.tmTheme` -- GrokNight、RosePineMoon、OscuraMidnight
- `grok-day.tmTheme` -- GrokDay
- `tokyo-night.tmTheme` -- TokyoNight

テーマを切り替えると、Grok が対応するファイルを自動的に選択します。`.tmTheme` ファイルはバイナリに組み込まれているため、独自のファイルには置き換えられません。

---

<a id="deep-customization-with-pagertoml"></a>

## pager.toml による詳細なカスタマイズ

TUI の外観を細かく制御するには、`~/.grok/pager.toml` を作成します。このファイルで、スクロールバックのレイアウト、ブロックのスタイル、アニメーションなどを制御できます。すべての設定にはデフォルト値があるため、上書きする値だけを指定してください。（開発ビルドでは、すべてのデフォルト値をコメントアウトしたテンプレートとしてこのファイルが生成されます。上書きするには行のコメントを解除してください。コメントのままの値は、今後も新しいデフォルト値に追従します。）

<a id="layout"></a>

### レイアウト

ビューポートのパディングとブロック間隔を制御します。

```toml
[scrollback.layout]
outer_vpad = 1          # ビューポートの垂直パディング（上 / 下）
outer_hpad_left = 2     # 左余白（最小: 1）
outer_hpad_right = 2    # 右余白（最小: 1）
block_pad_left = 2      # アクセント線とコンテンツの間のパディング
block_pad_right = 2     # 右端のコンテンツ後のパディング
```

<a id="scrollbar"></a>

### スクロールバー

```toml
[scrollback.scrollbar]
enabled = true          # スクロールバーを表示 / 非表示
gap_left = 0            # コンテンツとスクロールバーの間隔（0 = 隣接）
gap_right = 0           # スクロールバーと画面端の間隔（0 = 画面端）
# scrollbar_bg = "none" # 背景色を上書き（テーマのデフォルトを使う場合は "none"）
# scrollbar_fg = "none" # つまみの色を上書き（テーマのデフォルトを使う場合は "none"）
```

<a id="scroll-behavior"></a>

### スクロール動作

```toml
[scrollback.scroll]
margin = 0                  # 選択項目の上下に表示するコンテキスト行（0 = 端）
min_page_fraction = 0       # 最小スクロール量（ビューポートに対する割合、0～100）
follow_indicator = "center" # "center" = 下矢印を表示、"none" = 非表示
follow_auto_select = true   # 追従中に最新項目を自動選択
follow_by_overscroll = true # 最下部を越えてスクロールすると追従モードを有効化
anchor_on_fold = true       # 折りたたみ時にブロックヘッダーの画面位置を維持
```

<a id="display-options"></a>

### 表示オプション

```toml
[scrollback.display]
sticky_headers = true              # スクロールで通過したユーザープロンプトをヘッダーとして固定
tab_width = 4                      # タブ文字あたりの空白数（0 = そのまま渡す）
expandable_indicator = true        # 折りたたみ可能な項目が閉じているときに "›" を表示
expandable_indicator_char = "›"    # 使用する文字（デフォルト: "›"）
collapsed_accent_char = "❙"        # 折りたたまれたグループ化可能ブロックのアクセント（従来の Windows コンソールでは "|" にフォールバック）
dim_accent = 0.5                   # 薄いアクセントのブレンド係数（0.0～1.0）
line_under_last_entry = false      # 最後の項目の下に水平線を表示
selection_buttons = false          # 選択ボックスにコピー / 表示ボタンを表示
```

<a id="animation"></a>

### アニメーション

```toml
[animation]
fps = 30           # フレームレート（1～60）。高いほど滑らかだが CPU 使用率が増加
wave_rows = 32     # アクセントアニメーションの 1 波周期あたりの行数
```

<a id="block-styling-edit-diffs"></a>

### ブロックのスタイル: 編集差分

```toml
[scrollback.blocks.edit]
indent = true                   # 差分コンテンツをインデント
vpad = false                    # 差分の上下にパディングを追加
# expanded_by_default = true    # 未設定: config.toml の [ui] collapsed_edit_blocks に従う
                                # （フラグがオン = 折りたたまれた 1 行表示）。いずれかの形式に固定するにはコメントを解除
hunk_separator = "…"            # ハンク間の区切り（"…"、"───"、"⋯"、または区切りなしの場合は ""）
dual_line_numbers = false       # 2 列の行番号（GitHub のように旧 + 新）
# line_summary = false          # 折りたたみヘッダーに +N/-M を表示。未設定の場合は同じフラグに従う
# bg = "none"                   # ブロックの背景（"none"、"light"、"dark"）
```

<a id="block-styling-thinkingreasoning"></a>

### ブロックのスタイル: 思考 / 推論

```toml
[scrollback.blocks.thinking]
accent_enabled = true       # 思考ブロックにアクセント線を表示
animate = true              # 思考中にアクセント線をアニメーション表示
truncated_lines = 3         # 省略モードで表示する行数
bg_blend = 70               # Markdown 色と背景のブレンド率（0～100）
header = true               # "Thinking..." ヘッダーを表示
header_bright = false       # 明るいヘッダースタイル（薄い / 控えめなスタイルとの比較）
```

<a id="block-styling-tool-calls"></a>

### ブロックのスタイル: ツール呼び出し

```toml
[scrollback.blocks.tool]
muted_collapsed = true     # 折りたたまれたツール呼び出しをグレー表示
dim_details = true          # 括弧内の詳細（行数、マッチ数）を薄く表示
bullet = "diamond"          # ツールヘッダー前の箇条書き記号のスタイル
```

利用可能な箇条書き記号のスタイル:

| 設定値 | 文字 | 説明 |
|-------------|-----------|-------------|
| `none` | （なし） | 記号なし |
| `dot` | `·` | 中点（最小） |
| `small-circle` | `•` | ビュレット |
| `circle` | `●` | 塗りつぶし円 |
| `small-triangle` | `▸` | 右向きの小さい三角形 |
| `triangle` | `▶` | 右向きの三角形 |
| `diamond` | `◆` | 塗りつぶしひし形（デフォルト） |

<a id="block-styling-execute-shell-commands"></a>

### ブロックのスタイル: 実行（シェルコマンド）

```toml
[scrollback.blocks.execute]
first_lines = 2                   # 省略モードで先頭に表示する出力行数
last_lines = 3                    # 省略モードで末尾に表示する出力行数
accent_enabled = true             # アクセント線を表示（実行中はアニメーション）
header_style = "label"            # "shell"（$ 接頭辞）または "label"（Run 接頭辞）
muted_command_collapsed = true    # 折りたたみ時にコマンドテキストを薄く表示
```

<a id="block-styling-user-prompts-scrollback"></a>

### ブロックのスタイル: ユーザープロンプト（スクロールバック）

```toml
[scrollback.blocks.prompt]
vpad = true            # 垂直パディング
bg = "light"           # 背景（"none"、"light"、"dark"）
show_prefix = true     # プロンプトの接頭辞文字を表示
min_lines = 2          # 省略 / 固定モードでの最小コンテンツ行数
```

<a id="prompt-input-widget"></a>

### プロンプト入力ウィジェット

```toml
[prompt]
collapse_unfocused = true    # スクロールバックにフォーカスしているときに折りたたむ
mouse_hover = true           # マウスオーバー時にホバー強調を表示
show_prefix = true           # プロンプトの接頭辞文字を表示
```

<a id="todo-badges"></a>

### Todo バッジ

```toml
[todo]
badge_format = "default"   # "default" = 2/5（完了 / 合計）、"colon" = [▶:1 □:4 ✓:3 ✗:2]、"comma" = [1 ▶, 4 □, 3 ✓, 2 ✗]
```

<a id="terminal-behavior"></a>

### ターミナルの動作

```toml
[terminal]
alt_screen = "auto"    # "auto"、"always"、または "never"
```

代替画面のポリシー:

- `auto` -- 通常のターミナルと通常の tmux ではフルスクリーン、tmux コントロールモードと Zellij ではインライン。
- `always` -- 常にフルスクリーンへ移行。
- `never` -- フルスクリーンへ移行せず、メインのスクロールバック内でインライン実行。

<a id="plugins-ui"></a>

### プラグイン UI

```toml
disable_plugins = false   # true にすると /hooks、/plugins コマンドと注釈を非表示
```

---

<a id="theme-color-slots"></a>

## テーマのカラースロット

各テーマでは、TUI 全体で使用される次のカラースロットを定義します。

**背景:** `bg_base`, `bg_light`, `bg_dark`, `bg_highlight`, `bg_hover`, `bg_terminal`, `bg_visual`

**アクセント:** `accent_user`, `accent_assistant`, `accent_thinking`, `accent_tool`, `accent_system`, `accent_error`, `accent_success`, `accent_running`, `accent_skill`, `accent_plan`, `accent_verify`, `accent_feedback`, `accent_remember`, `accent_model`

**テキスト:** `text_primary`, `text_secondary`

**グレー:** `gray_dim`, `gray`, `gray_bright`

**セマンティック:** `command`, `path`, `running`, `warning`, `fuzzy_accent`

**境界線とスクロールバー:** `selection_border`, `hover_border`, `prompt_border`, `prompt_border_active`, `scrollbar_bg`, `scrollbar_fg`

**貼り付け:** `paste_bg`, `paste_fg`, `paste_dim`

**差分:** `diff_delete_bg`, `diff_delete_fg`, `diff_insert_bg`, `diff_insert_fg`, `diff_equal_fg`, `diff_gutter_fg`

**Markdown:** 見出しの色（`md_heading_h1`～`md_heading_h6`）、`md_code`, `md_code_bg`, `md_text`, `md_muted`, `md_task_checked`, `md_task_unchecked`, `link_fg`

テーマシステムがこれらのスロットを内部で管理し、ターミナルに合わせて自動的に量子化します。
