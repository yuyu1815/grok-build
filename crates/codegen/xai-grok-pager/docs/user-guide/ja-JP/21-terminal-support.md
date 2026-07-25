<a id="terminal-support-and-troubleshooting"></a>

# ターミナル対応とトラブルシューティング

Grok Build はフルスクリーン TUI として動作します。画面の描画には、色、クリップボード、マウス、フルスクリーン制御用のターミナルエスケープシーケンスを使用します。一部のターミナル、マルチプレクサー、SSH セッションでは、これらのシーケンスの処理方法が異なります。

<a id="quick-fixes"></a>

## クイック修正

<a id="truecolor--washed-out-or-wrong-colors"></a>

### Truecolor／色が薄い、または正しくない

```bash
# ~/.zshrc または ~/.bashrc に追加
export COLORTERM=truecolor
```

tmux 内または SSH 経由では、tmux の設定にも次を追加してください。

```tmux
# ~/.tmux.conf または ~/.byobu/.tmux.conf
set -g default-terminal "tmux-256color"
set -as terminal-features ",*:RGB"
```

<a id="recommended-tmux-settings-clipboard--passthrough"></a>

### 推奨 tmux 設定（クリップボード + パススルー）

```tmux
set -g set-clipboard on
set -g allow-passthrough on
```

編集後、次を実行します。

```bash
tmux source-file ~/.tmux.conf
# またはデタッチして再アタッチ
```

<a id="live-diagnostics-inside-grok"></a>

### Grok 内でのリアルタイム診断

次のスラッシュコマンドを実行します。

```
/terminal-setup
```

このコマンドは、Grok が検出したターミナル、マルチプレクサー、**色レベル**、**利用可能なテーマ**、クリップボード経路を報告し、問題と解決方法を一覧表示します。色レベルが truecolor 未満の場合は、truecolor 専用テーマ（TokyoNight、RosePineMoon、OscuraMidnight）を有効にする方法を説明します。または、Terminal.app が本質的に 256 色であることを示します。別名の `/terminal-check` と `/terminal-info` でも同じコマンドを実行できます。

---

<a id="detected-terminals"></a>

## 検出されるターミナル

Grok は環境変数から次のターミナルエミュレーターを検出します。

- **Apple Terminal**（Terminal.app）
- **Ghostty**
- **iTerm2**
- **Warp**
- **WezTerm**
- **Kitty**
- **Alacritty**
- **Rio**
- **foot**（Wayland ネイティブ、Linux）
- **VS Code**、**Cursor**、**Windsurf**、**Zed** の統合ターミナル
- **JetBrains** IDE のターミナル（IntelliJ、PhpStorm など）
- **Grok Desktop**
- **VTE** ベースのターミナル（GNOME Terminal、GNOME Console、Tilix）
- **Windows Terminal**

検出には次の制限があります。

- tmux 内では、Grok がターミナルの識別に必要とする変数が pager まで届きません。
- SSH 経由では、多くのターミナル変数が転送されません。
- tmux のグローバル環境（`tmux -g`）は、現在のセッションではなく、最初にサーバーへアタッチしたクライアントを反映します。

---

<a id="common-problems-and-fixes"></a>

## よくある問題と解決方法

<a id="problem-colors-look-wrong-or-lack-truecolor"></a>

### 問題：色が正しく表示されない、または truecolor が使えない

**原因**：`COLORTERM` が設定されていないか、tmux が 24 ビット RGB 用に設定されていません。

**解決方法**：上記 2 つの設定を適用し、Grok を再起動します。

**確認方法**：`/terminal-setup` を実行します。`color truecolor` と `themes all` が表示されることを確認してください。`color` が `256` または `basic` の場合は、issues セクションに有効化方法が表示されます。

<a id="problem-clipboard-problems"></a>

### 問題：クリップボードが正しく動作しない

Grok は最大 3 つの経路でクリップボードへ書き込みます。これらは `/terminal-setup` の **Clipboard routes** セクションに対応します。

- **native** — Grok は最初に必ず OS ネイティブのクリップボードへ書き込みます。
- **tmux buffer** — tmux 内では、Grok は tmux の貼り付けバッファーにも書き込みます（`tmux load-buffer`）。
- **OSC 52** — Grok は OSC 52 エスケープシーケンスを送信し、外側のターミナルのクリップボードを更新します。tmux 内では常に OSC 52 を送信します。tmux 外では、Linux、SSH 経由、またはディスプレイのないコンテナ内で OSC 52 を送信します。

**Linux Wayland**：data-control プロトコルに対応するコンポジター（GNOME 48 以降、KDE、Sway、Hyprland。`/terminal-setup` の `data-control` 行に `yes` と表示）では、コピー中にターミナルからフォーカスが外れてもコピーできます。古いコンポジター（GNOME 46/47）では、コピー完了のトーストが表示されるまでターミナルにフォーカスを置き、最も確実な経路として `wl-copy` を提供する `wl-clipboard` パッケージをインストールしてください。該当する場合、Grok は起動時に警告を表示します。コンポジターで data-control が正しく動作しない場合は、`GROK_CLIPBOARD_NO_DATA_CONTROL=1` を設定すると、Grok はこのプロトコルを一切使用しなくなります。その場合、コピーには CLI ツール（`wl-copy`／`xclip`）が使われます。

**Linux X11 のセレクション**：X11 の **PRIMARY** と **CLIPBOARD** は別のものです。通常、テキストを選択すると PRIMARY に入り、明示的なコピー操作を行うと CLIPBOARD に入ります。Grok では次のように動作します。

- 修飾キーなしの中クリックは、`DISPLAY` が空でない場合にのみ PRIMARY を読み取ります。純粋な X11 では、ネイティブの arboard リーダーへフォールバックできます。XWayland では `PATH` 上に `xclip` または `xsel` が必要です。Grok は XWayland で arboard へのフォールバックを意図的に無効にしており、Wayland PRIMARY の代替としては使用しません。
- `Ctrl+V` は CLIPBOARD だけを読み取り、PRIMARY へはフォールバックしません。シェルから CLIPBOARD に書き込むには、`printf %s "text" | xclip -selection clipboard` を実行します。
- `Shift+Insert` は引き続き、ターミナルネイティブの選択テキスト貼り付けとして動作します。ネイティブ Wayland の PRIMARY の動作はコンポジターやターミナルによって異なり、`TERM` や受信したマウスイベントから推測されることはありません。

**SSH と選択テキスト**：通常、リモートの Grok プロセスはローカルターミナルの PRIMARY または CLIPBOARD セレクションを読み取れません。ターミナルネイティブの `Shift+Insert` を使うか、ターミナルでその操作によってマウスレポートを回避できる場合は、`Shift` を押しながら中クリックしてください。これにより、リモートプロセスにアクセスさせる代わりに、ターミナルがローカルの選択内容を PTY 経由で送信します。

**既知の制限 — Apple Terminal + SSH**：
Apple Terminal は OSC 52 を無視するため、SSH 経由の Grok セッションからローカルのクリップボードへコピーできません。次の回避策を使用してください。

**一時的な回避策**：通常の `ssh` の代わりに `grok wrap ssh` を使用します（例：`grok wrap ssh user@host`）。このコマンドはローカル PTY 内で対象コマンドを実行し、tmux でラップされたものを含む OSC 52 シーケンスを傍受して、その内容をローカルのクリップボードへ書き込みます。同じコマンドで、クリップボードに到達できないほかのコマンドもラップできます。たとえば、`grok wrap docker exec -it <container> bash` や `grok wrap kubectl exec -it <pod> -- bash` です。

> **警告**：`grok wrap` は**試験的機能**であり、一部の環境では正しく動作しない可能性があります。

**iTerm2 の設定**：
iTerm2 では OSC 52 の明示的な許可が必要です。

1. iTerm2 → **Settings** → **General** → **Selection**
2. **"Applications in terminal may access clipboard"** を有効にする

この設定はセキュリティ上の理由からデフォルトで無効です。無効な場合、Grok（またはほかの TUI）からの OSC 52 による書き込みは無視されます。

**その他の場合の解決方法**：
- tmux の設定に `set -g set-clipboard on` を追加する
- その他のターミナルで SSH を使用する場合は、OSC 52 をネイティブでサポートする iTerm2、Ghostty、WezTerm、Kitty のいずれかへ切り替える

<a id="problem-fullscreen--alternate-screen-not-activating-inline-mode"></a>

### 問題：フルスクリーン／代替画面が有効にならない（インラインモード）

**原因**：Zellij、tmux コントロールモード（`tmux -CC`）、または設定が `never` になっています。

**解決方法**：
- Zellij またはコントロールモードでは、Grok は意図的にインラインで動作します（代替画面は使用しません）。
- フルスクリーンを強制するには、`~/.grok/pager.toml` で `[terminal] alt_screen = "always"` を設定します。
- 代替画面モードを完全に無効にするには、CLI フラグ `--no-alt-screen` を使用します（デバッグ時や、ターミナルで代替画面が問題を起こす場合に便利です）。

<a id="problem-zellij-keybindings-interfere-with-grok-ctrlg-ctrlo-etc"></a>

### 問題：Zellij のキーバインドが Grok と競合する（Ctrl+g、Ctrl+o など）

Zellij は、Grok のようなフルスクリーン TUI に届く前に、多くの Ctrl／Alt キーの組み合わせを処理します。

**最適な解決方法**（Zellij 0.41 以降）：**"Unlock-First (non-colliding)"** プリセットへ切り替えます。

1. `Ctrl+o` → `c` を押す（Configuration を開く）
2. **"Change Mode Behavior"** へ移動する
3. **"Unlock-First (non-colliding)"** を選択する
4. `Enter` を押す（または `Ctrl+a` で永続的に保存する）

設定後、Zellij は**ロック状態**で起動します。ほとんどのキーが Grok に渡されます。Zellij のペインやセッションを操作する必要があるときは、`Ctrl+g` を押して一時的にロックを解除します。

Zellij は TUI ユーザーにこの方法を推奨しています。

<a id="problem-ctrlenter-doesnt-interject-in-wezterm"></a>

### 問題：WezTerm で `Ctrl+Enter` による割り込みができない

**原因**：WezTerm では Kitty keyboard protocol が無効な状態で提供されています。Grok はこのプロトコルを利用して、`Ctrl+Enter`（割り込み）と `Shift+Enter`（複数行モードで送信）を通常の `Enter` と区別します。ほかの多くのターミナルでは、Grok が要求するとこのプロトコルが有効になります。

同じ理由から、Apple Terminal では Grok は `Ctrl+O` を割り込みに割り当てます。

**解決方法**：

`~/.config/wezterm/wezterm.lua` の `config = wezterm.config_builder()` より後に、次を追加します。

```lua
config.enable_kitty_keyboard = true
```

再読み込み（`Cmd+Shift+R` または WezTerm の再起動）を行い、`grok` を再起動します。

**確認方法**：Grok 内で `/terminal-setup` を実行します。ターンの実行中に割り込みのヒントが表示され、`Ctrl+Enter` で割り込めることを確認してください。

**簡易的な回避策**（グローバル設定を変更しない場合）：

```lua
table.insert(config.keys, {
  key = "Enter",
  mods = "CTRL",
  action = wezterm.action.SendString("\x1b[13;5u"),
})
```

<a id="problem-shiftenter-doesnt-insert-a-newline-in-vs-code"></a>

### 問題：VS Code で `Shift+Enter` を押しても改行されない

**原因**：VS Code の統合ターミナル（および Cursor／Windsurf／Zed の派生版）は xterm.js を使用しています。xterm.js は Kitty keyboard protocol を部分的にしか実装しておらず、Shift を押した印字可能文字を誤ってエンコードします（`!@#$%^&*()` が通常の数字として届きます）。そのため、Grok はこれらのターミナルではこのプロトコルをネゴシエートしません。プロトコルがない場合、xterm.js は `Shift+Enter` に対して通常の `Enter` とバイト単位で同一の単独の `CR` を送信するため、キーの組み合わせを区別できず、プロンプトが送信されます。

これは、**SSH 経由**で接続した VS Code（devbox やコンテナへの接続など）にも影響します。`TERM_PROGRAM` が転送されないため、Grok はターミナルを `Unknown` と認識し、同じ理由でプロトコルを使用しません。

**解決方法**：改行を挿入するには **`Alt+Enter`** を使用します。xterm.js はキーボードプロトコルにかかわらず、これを `ESC`+`CR` として確実に送信します。Grok はこの状況を検出すると、プロンプトのヒントバーに `Alt+Enter: newline` と表示します。`/terminal-setup` を実行して確認してください。`Shift+Enter` を使用できない場合、`newline` 行には `Alt+Enter` と表示されます。

<a id="problem-mouse-scrolling-stops-working-native-scrollbar-takes-over"></a>

### 問題：マウススクロールが動作しなくなる（ネイティブのスクロールバーに切り替わる）

Grok のマウスによるスクロールが反応しなくなり、ターミナルのネイティブなスクロールバーに切り替わった場合は、マウスレポートが無効になっています。

**Apple Terminal**：**View > Allow Mouse Reporting**（キーボードショートカット `Cmd+R`）を選択して、再度有効にします。有効な場合、この項目の横にチェックマークが表示されます。

**iTerm2**：**Settings**（`Cmd+,`）→ **Profiles** → **Terminal** を開き、**"Enable mouse reporting"** が有効であることを確認します。または、iTerm2 を再起動します。

<a id="problem-byobu--gnu-screen"></a>

### 問題：Byobu + GNU screen

screen 上の Byobu はベストエフォートでのみサポートされます。tmux 上の Byobu を推奨します。

---

<a id="still-stuck"></a>

## 解決しない場合

`/feedback` を実行して問題を報告してください。
