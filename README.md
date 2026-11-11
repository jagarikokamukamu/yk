# yk (yarukoto)

Rust製のTUI Todo管理ツールです。

## 技術スタック
- 言語: Rust (2021 edition)
- 描画: `ratatui`, `crossterm`
- データベース: `rusqlite` (SQLite)
- 日時・設定: `chrono`, `serde`, `toml`, `dirs`

## ファイル配置
- 設定ファイル: `%APPDATA%/yk/config.toml` (環境変数 `YK_CONFIG_PATH` で指定可能)
- データベース: `%APPDATA%/yk/tasks.db` (環境変数 `YK_DB_PATH` で指定可能)

### 設定項目 (`config.toml`)
```toml
# 完了タスクをアーカイブへ移動する日数（デフォルト: 1）
archive_after_days = 1

# 起動時の初期画面: "list" (Active) または "kanban" (Kanban)（デフォルト: "list"）
default_view = "list"
```

## ビルド・実行方法

```sh
# ビルド
cargo build --release

# TUI実行 (引数なし)
cargo run --release

# CLI実行
cargo run --release -- --help

# テスト実行
cargo test
```

## CLIコマンド

サブコマンドを指定して実行すると、TUIを開かずにターミナルから直接タスクを操作できます（引数なしで実行した場合はTUIが起動します）。

```sh
# タスクの追加 (alias: a)
yk add "牛乳を買う" -d 2026-10-31 -p high -m "成分無調整のもの"

# タスク一覧の表示 (alias: ls)
yk list
yk list --all       # アーカイブ済みを含む全タスク表示
yk list --archive   # アーカイブ済みタスクのみ表示

# タスクを進行中にする (alias: doing)
yk start 1          # 排他進行中（他をTodoに戻す）
yk start 1 -p       # 並行進行中（他を変更しない）

# タスクを完了にする (alias: finish, complete)
yk done 1           # 複数指定可: yk done 1 2 3

# タスクを削除する (alias: delete, del)
yk rm 1             # 確認プロンプト (y/N) を表示
yk rm 1 -f          # 確認なしで即時削除 (複数指定可: yk rm 1 2 -f)

# 定期タスクの管理 (alias: r)
yk routine list                                       # 定期タスク雛形一覧 (alias: ls)
yk routine add "週報作成" -f mon -d fri -p high        # 毎週月曜追加・金曜期日の定期タスク追加 (alias: a)
yk routine add "ゴミ出し" -f tue                      # 毎週火曜追加・火曜期日（-d省略時は追加曜日と同じ）
yk routine add "読書" -f sat -d none                  # 毎週土曜追加・期日なし
yk routine add "日次ふりかえり" -f daily -p medium    # 毎日の定期タスク追加（期日: 当日）
yk routine rm 1                                       # 定期タスクの削除 (alias: delete, del)
yk routine generate                                   # 到来した定期タスクを発行 (alias: gen)
yk routine generate 1                                 # 特定IDの定期タスクを即時手動発行
```

## TUIキー操作一覧

### 画面切り替え（ナビゲーション）
- `1` 〜 `4`: 各画面へ直接移動（`1: ACTIVE`, `2: KANBAN`, `3: ARCHIVE`, `4: ROUTINE`）
- `Tab` / `BackTab`: 4画面の巡回切り替え（`Active ↔ Kanban ↔ Archive ↔ Routine`）
- `Active` ↔ `Kanban` 間の画面切り替え時、選択中のタスク位置（列・行）が自動的に引き継がれます。
- `?`: ヘルプモーダルを開く
- `q` / `Esc`: アプリケーションを終了

### 未完了タスク一覧画面 (1: Active)
- `j` / `↓`: 下へ移動（末尾からは先頭へ循環）
- `k` / `↑`: 上へ移動（先頭からは末尾へ循環）
- `Space`: 完了トグル（Todo ↔ Done、completed_at を自動付与/解除）
- `i`: 排他進行中（他の InProgress タスクを Todo に戻し、選択タスクのみ InProgress に設定）
- `I`: 並行進行中（他のタスクを変更せず、選択タスクを InProgress に設定）
- `a`: 新規タスク追加（タイトル簡易入力ポップアップ表示）
- `e` / `Enter`: 詳細編集モーダルを開く
- `v`: 詳細閲覧モーダルを開く
- `d`: 削除確認モーダルを開く（`y` で削除、`n`/`Esc` でキャンセル）

### 看板モード画面 (2: Kanban)
TODO / DOING / DONE の3カラムで未完了タスクを表示します。
- `h` / `←`: 左カラムへ移動
- `l` / `→`: 右カラムへ移動
- `j` / `↓`: カラム内タスクを下へ移動（循環）
- `k` / `↑`: カラム内タスクを上へ移動（循環）
- `H` / `<`: ステータス前進（左列へ移動: `Done` → `Doing` → `Todo`、カーソル追従）
- `L` / `>`: ステータス後退（右列へ移動: `Todo` → `Doing` → `Done`、カーソル追従）
- `Space`: 完了トグル（Todo/Doing ↔ Done、移動先へカーソル追従）
- `i`: 排他進行中（Doing列へ移動、カーソル追従）
- `I`: 並行進行中（Doing列へ移動、カーソル追従）
- `a`: 新規タスク追加（常に Todo 列へ追加）
- `e` / `Enter`: 詳細編集モーダルを開く
- `v`: 詳細閲覧モーダルを開く
- `d`: 削除確認モーダルを開く

### アーカイブ画面 (3: Archive)
完了から一定日数（設定値: `archive_after_days`）経過したタスクの一覧です。
- `j` / `↓`: 下へ移動（末尾からは先頭へ循環）
- `k` / `↑`: 上へ移動（先頭からは末尾へ循環）
- `Space`: 未完了への復元（ステータスを Todo に戻し、即座に Active / Kanban 画面側へ復帰）
- `v`: 詳細閲覧モーダルを開く
- `d`: 削除確認モーダルを開く

### 定期タスク一覧画面 (4: Routine)
- `j` / `↓`: 下へ移動（末尾からは先頭へ循環）
- `k` / `↑`: 上へ移動（先頭からは末尾へ循環）
- `a`: 新規定期タスク追加モーダルを開く
- `e` / `Enter`: 定期タスク編集モーダルを開く
- `v`: 定期タスク詳細閲覧モーダルを開く
- `p` / `Space`: 選択中の定期タスクから実体タスクを手動即時発行する（アプリ起動時にも当日分が自動発行されます）
- `d`: 削除確認モーダルを開く（`y` で削除、`n`/`Esc` でキャンセル）

### 編集モーダル画面 (タスク / 定期タスク共通)
- `Tab` / `Shift+Tab`: 項目間移動（次へ / 前へ）
- `↓` / `↑`: 項目間移動（説明欄以外）
- `Space`, `←` / `→`: ステータス・優先度・頻度の変更
- `←` / `→` (期日欄): 1日加減（テキスト直接編集も可能、消去で「なし」）
- `Enter` (説明欄): 改行入力
- `Ctrl+Enter`: 保存して閉じる（タイトル欄等では `Enter` でも保存可）
- `Esc`: 保存せずにキャンセル

### 詳細閲覧モーダル画面
- `j` / `↓`: 下へスクロール
- `k` / `↑`: 上へスクロール
- `Esc` / `q` / `Enter` / `v`: 閉じる

