# `/model` モデル切り替え契約カプセル

```text
/model の入力
    ↓
候補表示・直接指定・現在値表示・help
    ↓
active session の model / effort を切り替える
    ↓
切り替え成功後だけ次回 session 用設定を保存する
```

## 1. 識別情報

| 項目 | 値 |
| --- | --- |
| Issue | `yuyu1815/grok-build#9` |
| 縦スライス | `/model` による active session の model / effort 切り替え |
| 文書状態 | `draft` |
| Contract revision | `draft-5` |
| 移植元 Claude Code revision | `6f6f12b37f529488b10e53928dd5508bb93535c7` |
| Rust base revision | `b189869b7755d2b482969acf6c92da3ecfeffd36` |

`draft-1` は、調査結果とユーザーが承認した Rust 固有の適応を固定した契約である。2026-07-17 にユーザーが path と revision を明示承認した。`draft-2` は、ユーザーが追加で許可した「Issue #9 の既承認 scope 内の根拠追加・実装整合 revision の自動承認」に基づき、実在 path の訂正と実装記録だけを追加した。`draft-3` は、Claude source に存在する current 表示の effort 条件と plan-mode session override 分岐を追加調査し、ユーザーが承認した Rust 到達可能範囲の A 案を固定する。`draft-4` は、picker の effort が model ごとの記憶ではなく picker 全体の単一 state であることと、model support 差の遷移を追加する。各 revision は前 revision の意味を上書きせず変更履歴として分離する。
`draft-1` は、調査結果とユーザーが承認した Rust 固有の適応を固定した契約である。2026-07-17 にユーザーが path と revision を明示承認した。`draft-2` は、ユーザーが追加で許可した「Issue #9 の既承認 scope 内の根拠追加・実装整合 revision の自動承認」に基づき、実在 path の訂正と実装記録だけを追加した。`draft-3` は、Claude source に存在する current 表示の effort 条件と plan-mode session override 分岐を追加調査し、ユーザーが承認した Rust 到達可能範囲の A 案を固定する。`draft-4` は、picker の effort が model ごとの記憶ではなく picker 全体の単一 state であることと、model support 差の遷移を追加する。`draft-5` は、source mismatch の再確認結果と、Rust で explicit effort を target model の API 境界まで保持する mapping を固定した。各 revision は前 revision の意味を上書きせず変更履歴として分離する。

| Revision | 状態 | 変更 |
| --- | --- | --- |
| `draft-1` | approved | 調査結果と Rust 固有適応を承認 |
| `draft-2` | approved（自動整合） | shell source anchor の実在 path 訂正、実装記録の追加 |
| `draft-3` | approved（ユーザー選択 A） | current の effort 条件を訂正し、Rust に到達不能な plan-mode override parity を deferred として固定 |
| `draft-4` | draft（source mismatch 再調査） | cross-model effort とsupport差の追加調査。approved確定前の途中revision |
| `draft-5` | approved（A parity・自動整合） | Claude 正本の effort matrix と Rust の explicit provenance / API 境界 mapping を固定 |

## 2. Scope

### In scope

- `/model` を引数なしで実行したときの model picker
- `/model default`
- `/model <model>` による直接指定
- help alias と current/info alias
- picker 内での model と effort の選択
- active session への model / effort 反映
- 次回 session 用 model / effort の永続化
- validation、active switch、永続化の各失敗時の観測可能な挙動
- success、cancel、current、help、error の表示

### Non-scope

- Rust server catalog 外の任意 model ID
- 未知 model に対する API 検証
- Claude の tier ごとに変わる候補
- Fast mode
- extra usage または課金表示
- Claude 固有の model 名と候補集合の完全再現
- `/model` 以外の settings UI や session lifecycle の挙動変更

## 3. 契約の境界

この文書では、根拠の性質が異なる二つを混ぜない。

```text
A. Claude source から確定した契約
   └─ 移植元から観測可能な入力・表示・状態遷移

B. Rust 固有のユーザー承認済み適応
   └─ Rust の catalog、ACP、永続化基盤へ接続するための決定
```

## 4. A: Claude source から確定した契約

### 4.1 Entry point と実装経路

コマンドと picker の入口は次のとおり。

- `claude-code/src/commands/model/index.ts`
- `claude-code/src/commands/model/model.tsx`
  - `ModelPickerWrapper`
  - `SetModelAndClose`
  - `ShowModelAndClose`
  - `call`
- `claude-code/src/components/ModelPicker.tsx`
  - `ModelPicker`

model 変更後の state、settings、次の API request までの経路は次のとおり。

```text
state/store.ts:createStore
  ↓
state/onChangeAppState.ts:onChangeAppState
  ↓
utils/settings/settings.ts:updateSettingsForSource
  ↓
utils/file.ts:writeFileSyncAndFlush_DEPRECATED

bootstrap/state.ts:setMainLoopModelOverride
  ↓
hooks/useMainLoopModel.ts:useMainLoopModel
  ↓
utils/handlePromptSubmit.ts
  ↓
services/api/claude.ts:1700
```

### 4.2 入力と dispatch

| 入力 | 観測可能な処理 |
| --- | --- |
| `/model` | picker を開く |
| `/model default` | default 指定として model を変更する |
| `/model <model>` | 指定 model の validation 後に変更する |
| `/model help` | help を表示する |
| `/model -h` | help を表示する |
| `/model --help` | help を表示する |
| `/model list` | 現在値を表示する |
| `/model show` | 現在値を表示する |
| `/model display` | 現在値を表示する |
| `/model current` | 現在値を表示する |
| `/model view` | 現在値を表示する |
| `/model get` | 現在値を表示する |
| `/model check` | 現在値を表示する |
| `/model describe` | 現在値を表示する |
| `/model print` | 現在値を表示する |
| `/model version` | 現在値を表示する |
| `/model about` | 現在値を表示する |
| `/model status` | 現在値を表示する |
| `/model ?` | 現在値を表示する |

### 4.3 Picker の状態と操作

```text
起動
  ↓ current model を初期選択
model 行: ↑ / ↓
effort:   ← / →
  ├─ Enter → 選択を commit
  └─ Esc   → 変更を cancel
```

- picker の visible count は最大 `10`。
- current model が通常候補に含まれない場合は、current model を候補へ追加する。
- `Enter` までは選択中の値であり、commit されない。
- `Esc` は model と effort の変更を適用しない。

effort は model ごとの記憶ではなく、picker 全体で一つの React state (`effort`) と、利用者が Left/Right を操作したかを示す `hasToggledEffort` で管理される。

```text
picker起動
  ├─ effortValueあり  → 単一effortへ初期化
  └─ effortValueなし  → 単一effortは未定義

Up / Down
  ├─ effortValueあり、または操作済み
  │    → 単一effortを維持
  └─ effortValueなし、かつ未操作
       → 表示用stateを移動先model defaultへ更新
       → Enterで明示effortとしては渡さない

Left / Right
  ├─ focused modelがeffort対応 → 単一effortをcycle、hasToggledEffort=true
  └─ effort非対応             → no-op
```

- model A から B へ移動して戻っても、model ごとの過去値へ復元しない。
- `hasToggledEffort=false` の Enter は、画面に default effort が表示されていても `selectedEffort=undefined` とする。
- effort非対応modelへ移動しても単一effort state は破棄しない。ただし Enter では effort を渡さない。
- `max` を保持したまま max 非対応・effort対応modelへ移動した場合、underlying state は `max` のまま、表示と API 適用値は `high` になる。
- 根拠は `claude-code/src/components/ModelPicker.tsx:56-66,143-192,225-249,431-446` と `claude-code/src/utils/effort.ts:151-179` である。

### 4.4 表示契約

Claude 固有で non-scope の表示を除き、次の文言を保持する。

| 状況 | 文言 |
| --- | --- |
| cancel | `Kept model as <model>` |
| success | `Set model to <model> [with <effort> effort]` |
| current | `Current model: <model> [(effort: <effort>)]` |
| help | `Run /model to open the model selection menu, or /model [modelName] to set the model.` |

角括弧部分の `with <effort> effort` は、effort を表示する場合の条件付き部分を表す。

current の角括弧部分も条件付きである。`effortValue` が `undefined` の場合は suffix 全体を省略し、`(effort: none)` は表示しない。

Claude source は plan mode が session 専用 model override を持つ場合、次の二行を表示する。

```text
Current model: <session model> (session override from plan mode)
Base model: <base model> [(effort: <effort>)]
```

根拠は `claude-code/src/commands/model/model.tsx:246-263` の `ShowModelAndClose`、`mainLoopModel`、`mainLoopModelForSession`、`effortValue` である。

### 4.5 状態、settings、副作用、順序

移植元では model 変更後、session state と settings 永続化が接続される。settings write が失敗した場合の観測結果は次のとおり。

```text
state = new model
  ↓
settings write
  ├─ 成功 → disk も新値
  └─ 失敗
       → caller は error 返却値を利用しない
       → session override は新 model
       → rollback しない
       → success 表示
       → 次の API request も新 model
       → disk 値の更新は保証されない
```

- settings write 失敗は通常 UI へ表示されない。
- validation 失敗は settings write 失敗と異なる。
- validation 失敗時は state を変更せず、保存せず、error を表示する。

### 4.6 Permission、error、cancel、retry、timeout

| 観点 | Claude source から確定した契約 |
| --- | --- |
| Permission | `/model` 固有の approval 要求は確認されていない |
| Validation error | state 変更なし、保存なし、error 表示 |
| Persistence error | active session は新値、rollback なし、通常 UI への失敗表示なし |
| Cancel | 選択を適用せず `Kept model as <model>` を表示 |
| Retry | `/model` 内部の自動 retry は確認されていない |
| Timeout | `/model` 固有の timeout は確認されていない |

### 4.7 境界条件と feature flag

- 引数なしは直接指定ではなく picker 起動になる。
- `default` は通常の model 名指定と区別される。
- help alias と current/info alias は上記一覧のとおり dispatch される。
- current model が通常候補外でも、picker から現在値が失われない。
- 表示件数は最大 `10`。
- validation に失敗する model は active state と settings を変更しない。
- 調査対象経路では `/model` 契約を切り替える feature flag は確認されていない。

### 4.8 根拠となる test、fixture、snapshot

調査対象の Claude source では、`/model` に専用の test、fixture、snapshot は確認できなかった。したがって A の oracle は上記の到達可能な製品コードから構成する。コメントや外部資料で製品コードの挙動を補完していない。

## 5. B: Rust 固有のユーザー承認済み適応

### 5.1 候補と validation の境界

```text
Rust server catalog
        ∩
      allowlist
        ↓
/model の選択・直接指定で利用可能な候補
```

- picker と直接指定で受理する候補は、Rust server catalog と allowlist の両方に含まれる model に限定する。
- catalog 外の任意 ID を送信して有効性を試す処理は実装しない。
- Claude 固有候補の完全再現は行わない。

### 5.2 `/model default`

`/model default` は二つの効果を一つの操作として扱う。

```text
保存済み model override を削除
              +
active session を既定 model へ切り替え
```

既定 model は Rust 側の既存設定解決と server catalog が決定する値を使用し、Claude 固有 model 名を埋め込まない。

### 5.3 Effort

picker で明示変更した effort は次の両方へ反映する。

```text
選択した effort
  ├─ active session へ反映
  └─ 次回 session 用設定へ永続化
```

model と effort は同じ commit 操作の選択値として扱う。`Esc` ではどちらも変更しない。

Rust picker も model ごとの effort 配列を持たず、picker-wide の単一 `Option<ReasoningEffort>` と explicit/toggled flag を持つ。

- Up/Down は単一effortをmodel defaultやmodel別記憶へ差し替えない。
- effortが未操作なら、model移動後の Enter も ACP と永続化へ `None` を渡す。
- Left/Right のみ explicit/toggled flagを立てる。
- effort非対応modelでは Left/Right をno-opとし、Enterでは `None` を渡すが、単一state自体は保持する。
- `ReasoningEffort::Xhigh` は Claude の `max` に対応する。選択modelが `Xhigh` を提供せず `High` を提供する場合、表示と ACPへ渡す適用値は `High` とし、picker-wide state は `Xhigh` のまま保持する。Rust ACP switch は適用境界であるため、Claude の後段API downgradeをこの境界へ対応付ける。

### 5.4 Rust で保証する処理順

active switch と永続化を並列 task として競合させず、次の順に直列化する。

```text
候補 validation
  ├─ 失敗
  │    → active state を変更しない
  │    → 永続化しない
  │    → error 表示
  │
  └─ 成功
       ↓
     ACP active switch
       ├─ 失敗
       │    → 旧 active model / effort を維持
       │    → 永続化しない
       │    → error 表示
       │
       └─ 成功
            → active model / effort は新値
            → command success 表示
            ↓
          次回 session 用 model / effort を永続化
            ├─ 成功 → 完了
            └─ 失敗
                 → active model / effort は新値のまま
                 → rollback しない
                 → 次回 session 用設定は旧値
                 → command success 表示を維持
                 → 既存 tracing::warn! 経路へ記録
```

永続化失敗の logging は、実在する次の経路を使用する。

- `xai-grok-pager/src/app/effects/mod.rs:1831-1847`
- 既存文言: `failed to save default model preference: {e}`

新しい独自の成功応答や、保存失敗を成功に見せるための no-op は追加しない。ここで success 表示を維持するのは、active session の切り替え自体が成功済みであるためである。

### 5.4.1 Explicit provenance の Rust mapping

`ModelState.reasoning_effort` は ACP の effective 値と catalog default の両方を表し得るため、値の `Some` だけから explicit を推定しない。`reasoning_effort_explicit` は起動時の user config / CLI effort source から seed し、picker の `effortValue` 相当として扱う。

```text
picker Enter
  ├─ Left/Right 操作済み
  │    → picker の effort を Action へ渡す
  ├─ 未操作 + explicit provenance あり
  │    → target が effort 対応なら router が既存 effort を Action/ACP meta へ補完
  │    → target 非対応なら None（API へ送らない）
  └─ 未操作 + provenance なし
       → None（target catalog default に委ねる）
```

この補完は `clear_default=true` の reset 操作には適用しない。`Xhigh` を `High` へ downgrade するのは shell の `set_session_model` API 適用境界で行い、picker-wide underlying state は `Xhigh` のまま保持する。

### 5.5 表示の適応

- 4.4 の文言を、Claude 固有・non-scope の部分を除いて保持する。
- Rust catalog の model 表示名と effort 表現を `<model>`、`<effort>` に差し込む。
- ACP active switch または validation の失敗は error として利用者へ表示する。
- 永続化だけが失敗した場合は command success 表示を取り消さず、通常 UI に追加の失敗表示を出さず、既存 warning logging を使用する。
- Rust で到達可能な current 表示は、active model の一行表示とする。explicit effort がある場合だけ ` (effort: <effort>)` を付け、ない場合は suffix を省略する。
- Rust の `AgentSession.models.current` と `AppView.models.current` はともに active model へ同期される。Rust の plan mode は model を切り替えず、Claude の `mainLoopModelForSession` に相当する state と provenance、および二行表示へ到達する経路は存在しない。
- したがって plan-mode session override の二行表示を、`user_model_preference`、CLI override、active model と default model の比較などから推測して発火させない。この parity は deferred として明示し、動作したように見せる代替 state を追加しない。

### 5.6 Rust の参照 schema、types、constants、settings、state

実装時に接続する既存基盤は次のとおり。

| 責務 | 参照箇所 |
| --- | --- |
| slash command | `xai-grok-pager/src/slash/commands/model.rs:ModelCommand` |
| default model setting | `xai-grok-pager/src/app/dispatch/settings/setters.rs:set_default_model` |
| model switch completion | `xai-grok-pager/src/app/dispatch/session/lifecycle.rs:handle_switch_model_complete` |
| effect 実行・永続化 | `xai-grok-pager/src/app/effects/mod.rs` |
| ACP session model 設定 | `xai-grok-shell/src/agent/mvp_agent/acp_agent.rs:set_session_model` |
| agent model switch handler | `xai-grok-shell/src/agent/handlers/model_switch.rs:apply` |
| ACP session 実装 | `xai-grok-shell/src/session/acp_session_impl/model_switch.rs:handle_set_session_model` |
| modal state | `xai-grok-pager/src/views/modal.rs:ActiveModal` |
| modal key dispatch | `xai-grok-pager/src/app/modals.rs:handle_modal_key` |
| picker state/rendering | `xai-grok-pager/src/views/picker.rs` |
| modal frame | `xai-grok-pager/src/views/modal_window.rs` |
| enum picker pattern | `xai-grok-pager/src/views/settings_modal.rs:PickingEnum` |
| terminal UI | `ratatui 0.29.0`, `crossterm 0.28.1` |

model 候補、allowlist、既定 model、現在 model、現在 effort、保存済み override は、上記既存基盤が保持する schema、settings、session state を正とする。新しい独立 catalog や二重の session state は作らない。

### 5.7 UI 内部適応

承認契約を Rust の既存 UI へ接続する内部設計は次を基本とする。

```text
ActiveModal::ModelPicker
  └─ ModalWindow
       └─ PickerState
            ├─ ↑↓ model
            └─ ←→ effort
```

これは内部実装の予定であり、外部契約は 4.2 から 5.5 までで固定する。既存型との整合上、同じ観測可能挙動を保つ機械的な型名変更が必要な場合は契約変更とは扱わない。

## 6. 現在の Rust 実装との差分

現在は `PersistSetting` と `SwitchModel` を別 task として spawn し、`JoinSet::join_next` により完了順が非決定になる。

- `xai-grok-pager/src/app/event_loop.rs:process_effects`
- `xai-grok-pager/src/app/dispatch/settings/ui.rs:apply_setting_rollback`

また、保存失敗時の reverse `SwitchModel` と、switch 成功後の `PersistPreferredModel` があり、複合失敗時の状態が複雑である。

```text
現在: switch と persist が競合 + rollback 経路
                    ↓ 変更予定
契約: switch 成功後に persist + 保存失敗時 rollback なし
```

これは approved 後に実装で解消すべき差分であり、移植元の不確実性ではない。

## 7. 変更予定範囲

approved 後の実装 sub は、次の責務に直接必要な製品コードとテストだけを変更する。

- `ModelCommand` の alias、引数なし picker、default、直接指定の dispatch
- model/effort picker の modal state、rendering、key handling
- server catalog と allowlist に基づく候補構築と validation
- ACP active switch 完了後の model/effort 永続化の直列化
- switch 失敗と validation 失敗の error 表示
- persistence 失敗時の既存 warning logging と rollback 撤廃
- 契約を検証する unit / integration test

実装中に別機能の schema、permission、表示、永続化、session lifecycle を変える必要が生じた場合は scope drift として停止する。

## 8. Test と parity 検証計画

### 8.1 実装テスト

```text
command dispatch
  ├─ 引数なし → picker
  ├─ default
  ├─ 直接 model 指定
  ├─ help aliases
  └─ current aliases

picker
  ├─ current model 初期選択
  ├─ ↑↓ model
  ├─ ←→ effort
  ├─ visible count 最大10
  ├─ 候補外 current model の追加
  ├─ Enter commit
  ├─ Esc cancel
  ├─ defaultの異なるmodel間で既存/操作済みeffortを維持
  ├─ model移動後も未操作effortはcommitしない
  ├─ model移動→戻るでmodel別記憶へ復元しない
  ├─ effort非対応modelでstate保持 + commit None
  └─ Xhigh非対応modelで表示/適用High、state Xhigh維持

effect ordering
  ├─ validation失敗 → 無変更・無保存・error
  ├─ switch失敗 → 旧active・無保存・error
  ├─ switch成功 + 保存成功 → active/保存とも新値
  └─ switch成功 + 保存失敗 → active新値・保存旧値・rollbackなし・warn
```

表示テストでは success、cancel、current、help の文言を固定し、effort 表示あり・なしの両方を確認する。`/model default` は override 削除と active session の既定 model 切り替えを同時に確認する。

current/info alias は全 alias について、explicit effort がある場合とない場合を確認する。Claude の plan-mode override 分岐は Rust に同等 state と到達経路がないため、存在しない state を作るテストは追加しない。

### 8.2 独立 parity 検証

実装を担当していない別 sub が二辺を独立に検証する。

```text
Pass 1: Claude source + approved 契約
        → 入力・表示・状態遷移の oracle を再構成

Pass 2: Rust diff + test + 実行結果
        → oracle と Rust 固有の承認済み適応へ照合
```

特に次を parity gate とする。

- alias の欠落がないこと
- picker の同一画面上で model と effort を操作できること
- cancel まで state と settings を変更しないこと
- validation、switch、persist の順序が決定的であること
- switch 失敗時に永続化しないこと
- persist 失敗時に active session を rollback しないこと
- catalog と allowlist の外を候補・直接指定で受理しないこと
- non-scope の Claude 固有機能を成功したように見せる代替実装がないこと

## 9. 調査したが契約根拠として採用しなかった経路

| 経路・情報 | 不採用理由 |
| --- | --- |
| `ArgPicker` | 二段階 autocomplete 用であり、同一画面の `↑↓ model` と `←→ effort` を満たさない |
| 現在の `PersistSetting` / `SwitchModel` 並列 task | 完了順が非決定で、承認済みの直列順序を満たさない |
| 現在の保存失敗 rollback | 移植元の rollback なしと、承認済み Rust 適応に反する |
| Claude tier 別候補、Fast mode、extra usage 表示 | 明示的 non-scope |
| catalog 外 ID の API 検証 | 候補境界の承認済み判断に反する |
| 現行製品、外部文書、ブラックボックス観測 | Claude Code の挙動を決める根拠として使用禁止 |

## 10. Stub、矛盾、欠損、未確認事項

### 移植元

- `/model` 専用 test、fixture、snapshot が欠損しているため、製品コードが直接の oracle になる。
- 調査した到達可能経路内に、契約を一意に決められない stub は確認されていない。
- validation 失敗と settings write 失敗の挙動は異なるが、矛盾ではなく別の失敗段階である。
- `/model` 固有の retry、timeout、permission approval、feature flag は確認されていない。

### Rust

- 現在の並列 effect と rollback は、承認済み順序との差分である。
- 複合失敗を網羅する既存 test は不足している。
- server catalog と allowlist の具体的な既存型名・所有モジュールは、実装時に既存基盤へ接続する範囲で確定する。候補集合の契約自体は「両方の積集合」で承認済みであり、独自 catalog の追加は認めない。
- 現時点で、実装を停止してユーザー判断を再度必要とする未解決の契約判断はない。

## 11. ユーザー決定と承認範囲

ユーザーは `draft-1` 作成前の調査報告に対し、次の判断を承認した。

1. `/model default` は保存済み override を削除し、active session を既定 model へ切り替える。
2. picker で明示変更した effort は active session と次回 session 用設定の両方へ反映する。
3. Claude 固有・non-scope の部分を除き、移植元の表示文言を保持する。
4. ACP active switch を先に行い、成功後だけ model / effort を永続化する。
5. switch 失敗時は旧 active state を維持し、永続化せず、error を表示する。
6. 保存失敗時は active state を新値のまま維持し、rollback せず、次回 session 用設定は旧値のままとし、success 表示を維持して既存 `tracing::warn!` 経路へ記録する。
7. 候補は Rust server catalog と allowlist の積集合に限定する。
8. 2章の Non-scope を実装対象へ含めない。
9. `draft-3` の A 案として、Rust で到達可能な current 一行表示だけを実装し、effort がない場合は suffix を省略する。Claude の plan-mode session override 二行表示は、同等 state/provenance と到達経路が Rust にないことを明記して deferred parity とする。
10. `draft-4` として、cross-model effort は picker-wide single state とし、model ごとの effort 記憶を持たない。effort未操作時の明示保存なしを維持し、support差は4.3と5.3の境界に従う。

この承認は調査上の判断を `draft-1` に記録することを許可したものである。製品コードとテストの実装承認は、ユーザーが次の path と revision を明示的に承認した時点で成立する。

```text
Issue #9 の契約カプセル
docs/contracts/claude-code/model.md
contract revision draft-1
```

その後、ユーザーは上記 path と `draft-1` を明示承認し、製品コードとテストの実装を許可した。また、Issue #9 の既承認 scope 内に限定した根拠追加と実装整合 revision は自動承認として扱う追加決定を行った。`draft-2` はこの追加決定の範囲内である。

2026-07-18、ユーザーは current 表示の追加調査結果に対して A 案を明示承認した。これにより `draft-3` は approved である。A 案は到達可能な一行表示の effort 条件を実装し、存在しない plan-mode override state を推測で追加しない決定である。

同日、cross-model effort の追加調査中に、`effortValue` が未定義かつ未操作の場合だけ focused model default へ表示stateを更新する分岐が判明した。先行する「常に維持」という前提と異なるため、`draft-4` は approved とせず再調査用 draft に戻した。後続revisionでユーザー選択Aを確定する。

`draft-5` では、上記 source 分岐を確定したうえで、Rust 側の `reasoning_effort_explicit` を provenance として使用する mapping を固定した。未操作かつ explicit provenance がある model move では target が effort 対応の場合だけ既存 effort を ACP meta へ補完し、provenance がない場合は catalog default に委ねる。これは Issue #9 の既承認 scope 内の実装整合であり、ユーザーが許可した future draft の自動承認範囲として approved と記録する。

## 12. 実装記録（承認済み契約とは分離）

この節は approved 契約部分と分離した実装記録であり、`draft-2` で追加し、`draft-3` 以降の実装整合記録を追記する。

| 項目 | 値 |
| --- | --- |
| 実装日 | `2026-07-17`〜`2026-07-18` |
| 実装状態 | 実装完了、独立 parity 検証待ち |
| Git state | `codex/issue-9-model-picker` の未コミット diff |

実装フローは次のとおり。

```text
/model
  ├─ 引数なし → ActiveModal::ModelPicker
  │                ├─ ↑↓ model
  │                ├─ ←→ effort
  │                ├─ Enter → SetModelFromCommand
  │                └─ Esc   → 無変更 + cancel表示
  ├─ help/current aliases → 契約文言を表示
  └─ model/default → catalogでvalidation
                         ↓
                    ACP active switch
                      ├─ 失敗 → 無保存・error
                      └─ 成功 → active state反映・success表示
                                      ↓
                                 model/effort保存
                                   └─ 失敗 → rollbackなし・warnのみ
```

主な実装箇所:

- `slash/commands/model.rs`: 入力、alias、direct model、default、help/current
- `views/model_picker.rs`: catalog候補、current初期選択、model/effort操作
- `views/modal.rs`, `app/modals.rs`: `ActiveModal::ModelPicker`、描画、key/cancel
- `app/actions.rs`, `app/dispatch/router.rs`: `/model` 専用 action と switch intent
- `app/effects/mod.rs`, `app/dispatch/session/lifecycle.rs`, `app/dispatch/task_result.rs`: active switch 成功後の永続化、失敗契約

追加した検証対象:

- 引数なし、help/current alias、default、直接指定、effort
- current model 初期選択、候補外 current 追加、Left/Right effort、Esc no-change
- catalog 外 ID 拒否
- switch 前に永続化しないこと
- switch 成功、switch 失敗、default clear、永続化失敗時 no rollback / no UI error

実装時の検査結果:

- `cargo check -p xai-grok-pager`: 成功
- `cargo check -p xai-grok-pager --tests`: repository 既存の `cfg(test)` 境界問題による `158` compile errors で完走不可。今回追加箇所を path filter して確認し、追加実装固有の diagnostics は残っていない。

`draft-3` で追加した実装整合:

- prompt の空引数即 submit は `/model` と alias `/m` の canonical command に限定し、他の optional-args command の autocomplete を維持する。
- current/info alias は effort が `Some` の場合だけ suffix を表示し、`None` の場合は suffix を省略する。
- plan-mode override 二行表示は deferred parity とし、Rust に存在しない provenance state を追加しない。

`draft-4` で追加した実装整合:

- `ModelPickerState` の model別 `selected_efforts` を廃止し、単一 `effort` と `effort_toggled` へ統合する。
- Up/Downでは単一effortを維持し、Left/Rightだけが値とexplicit flagを変更する。
- unsupported model と Xhigh/max support差を source に対応する表示・commit境界で処理する。
- state seam と modal Up/Down/Left/Right/Enter seam の回帰testを追加する。

`draft-5` で追加した実装整合:

- `ModelState.reasoning_effort_explicit` を catalog の effective `Some` と区別し、user config / CLI source から provenance を seed する。
- `SetModelFromCommand` の未操作 `effort=None` は、explicit provenance があり target が effort 対応の場合だけ既存 effort を `Effect::SwitchModel` の ACP meta へ補完する。unsupported target と reset (`clear_default=true`) は `None` のままとする。
- defined medium → target high、undefined + untoggled → target default、unsupported → no-op/None、Xhigh → API boundary High の cross-model 回帰を追加・記録する。

この節の内容は独立検証 sub が oracle として採用してはならない。独立検証は 8.2 の順序どおり、移植元と approved 契約から先に oracle を再構成する。
