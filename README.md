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

## ビルド・実行方法

```sh
# ビルド
cargo build --release

# 実行
cargo run --release

# テスト実行
cargo test
```

## キー操作一覧

### メイン一覧画面
- `j` / `↓`: 下へ移動（末尾からは先頭へ循環）
- `k` / `↑`: 上へ移動（先頭からは末尾へ循環）
- `Space`: 完了トグル（Todo ↔ Done、completed_at を自動付与/解除）
- `i`: 排他進行中（他の InProgress タスクを Todo に戻し、選択タスクのみ InProgress に設定）
- `I`: 並行進行中（他のタスクを変更せず、選択タスクを InProgress に設定）
- `a`: 新規タスク追加（タイトル簡易入力ポップアップ表示）
- `e` / `Enter`: 詳細編集モーダルを開く
- `v`: 詳細閲覧モーダルを開く
- `d`: 削除確認モーダルを開く（`y` で削除、`n`/`Esc` でキャンセル）
- `A`: アクティブ一覧 ↔ アーカイブ一覧の表示切り替え
- `?`: ヘルプモーダルを開く
- `q` / `Esc`: アプリケーションを終了

### 編集モーダル画面
- `Tab` / `Shift+Tab`: 項目間移動（次へ / 前へ）
- `↓` / `↑`: 項目間移動（説明欄以外）
- `Space`, `←` / `→`: ステータス・優先度の順送り・逆送り
- `←` / `→` (期日欄): 1日加減（テキスト直接編集も可能、消去で「なし」）
- `Enter` (説明欄): 改行入力
- `Ctrl+Enter`: 保存して閉じる（タイトル欄等では `Enter` でも保存可）
- `Esc`: 保存せずにキャンセル

### 詳細閲覧モーダル画面
- `j` / `↓`: 下へスクロール
- `k` / `↑`: 上へスクロール
- `Esc` / `q` / `Enter` / `v`: 閉じる

