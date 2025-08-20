# shrup - Shell Script Preprocessor 仕様書

## 概要

`shrup`は、Shell scriptファイルの`#include`ディレクティブを解決してファイルを結合し、単一のスクリプトファイルを生成するプリプロセッサーツールです。

## プロジェクト情報

- **プロジェクト名**: shrup
- **言語**: Rust (edition 2021)

## 実行仕様

### コマンドライン引数

```bash
shrup.exe <input> <output> [options]
```

#### 位置引数
- `INPUT`: プリプロセスするShell scriptファイルのパス
- `OUTPUT`: 結合後のShell scriptファイルの出力パス

#### オプション引数
- `--debug, -d`: デバッグモード（includeコメントを出力に含める）
- `--max-depth <NUMBER>`: 最大include深度（デフォルト: 100）

### 使用例

```bash
# 基本的な使用
shrup main.sh output.sh

# デバッグモード
shrup --debug main.sh output.sh

# 最大深度を指定
shrup --max-depth 50 main.sh output.sh
```

## 機能仕様

### 1. Include構文解析

#### サポートする構文形式
- `#include <filepath>` （山括弧）
- `#include "filepath"` （ダブルクオート）
- `#include 'filepath'` （シングルクオート）
- `#include filepath` （クオートなし）

#### パス解決ルール
- **相対パス**: inputファイルが存在するディレクトリからの相対位置
- **絶対パス**: inputファイルのディレクトリをベースディレクトリとして解決

### 2. ファイル結合処理

- includeされたファイルの内容をディレクティブの位置に挿入
- 再帰的なincludeに対応（A→B→Cのようなチェーンを解決）
- 循環参照の検出とエラーハンドリング

### 3. エラーハンドリング

#### エラー種別
- **FileNotFound**: includeファイルが存在しない
- **CircularDependency**: 循環参照を検出
- **MaxDepthExceeded**: 最大include深度を超過
- **PermissionDenied**: ファイル読み込み権限不足
- **InvalidIncludeDirective**: 不正なinclude構文
- **IoError**: その他のI/Oエラー

#### エラー処理方針
- エラー発生時は即座に処理を終了
- エラーメッセージとエラーチェーンを表示
- 終了コード1で終了

### 4. デバッグモード

デバッグモード（`--debug`）有効時は、includeされたファイルの前後にコメントを挿入：

```bash
# --- Included from utils/functions.sh ---
function log() {
    echo "[$(date)] $1"
}
# --- End of utils/functions.sh ---
```

## アーキテクチャ設計

### モジュール構成

#### `src/main.rs`
- CLI引数処理とエントリポイント
- 入力ファイル検証
- プリプロセッサーの設定とビルド
- エラーハンドリングと表示

#### `src/lib.rs`
- ライブラリエントリポイント
- 全モジュールの公開とre-export

#### `src/error.rs`
- カスタムエラー型定義
- `thiserror`を使用したエラー実装
- Result型エイリアス定義

#### `src/parser.rs`
- Include構文解析エンジン
- 構文パターン検出（角括弧、クオート等）
- `IncludeDirective`構造体定義

#### `src/preprocessor.rs`
- メインプリプロセッシングロジック
- ファサードパターンによる統合制御
- 再帰的ファイル処理
- `PreprocessorBuilder`による設定

#### `src/resolver.rs`
- ファイルパス解決
- 循環参照検出
- Include深度管理
- 処理コンテキスト管理

### 主要データ構造

#### `IncludeDirective`
```rust
pub struct IncludeDirective {
    pub line_number: usize,      // 行番号（1-indexed）
    pub file_path: String,       // includeするファイルパス
    pub source_file: PathBuf,    // 元ファイルパス
    pub quote_type: IncludeQuoteType, // クオート種別
}
```

#### `ProcessingConfig`
```rust
pub struct ProcessingConfig {
    pub debug_mode: bool,        // デバッグモード
    pub max_include_depth: usize, // 最大include深度
    pub base_directory: PathBuf,  // ベースディレクトリ
}
```

#### `ProcessingContext`
```rust
pub struct ProcessingContext {
    visited_files: HashSet<PathBuf>,  // 循環参照検出用
    include_stack: Vec<PathBuf>,      // 現在のincludeスタック
    config: ProcessingConfig,         // 設定情報
}
```

### 処理フロー

1. **CLI引数解析** (`main.rs`)
   - 入力/出力ファイルパス取得
   - オプション設定読み込み
   - 入力ファイル存在確認

2. **プリプロセッサー構築** (`preprocessor.rs`)
   - 設定に基づくプリプロセッサー作成
   - ベースディレクトリ設定

3. **ファイル処理開始** (`preprocessor.rs`)
   - 入力ファイル読み込み
   - 処理コンテキスト初期化

4. **Include解析** (`parser.rs`)
   - 各行のinclude構文スキャン
   - ディレクティブ抽出と検証

5. **ファイル解決** (`resolver.rs`)
   - パス解決（相対/絶対）
   - ファイル存在確認
   - 循環参照チェック

6. **再帰処理** (`preprocessor.rs`)
   - includeファイルの再帰的処理
   - コンテンツ結合
   - デバッグコメント挿入

7. **出力生成** (`preprocessor.rs`)
   - 最終結果の出力ファイル書き込み

## セキュリティ仕様

### ファイルアクセス制御
- ファイルパストラバーサル攻撃の防止
- シンボリックリンクの適切な処理（canonicalize使用）
- ファイル読み込み権限の事前チェック

### リソース保護
- 最大include深度による無限再帰防止
- 循環参照検出による無限ループ防止
- メモリ使用量の適切な管理

## テスト仕様

### 単体テスト
- **Parser Tests**: 各クオート形式の構文解析
- **Resolver Tests**: パス解決とエラーハンドリング
- **Context Tests**: 循環参照と深度制限

### 統合テスト
- **Basic Include**: 単純なファイルinclude
- **Recursive Include**: 多階層include処理
- **Debug Mode**: デバッグコメント出力
- **Error Cases**: 各種エラーシナリオ

### 依存関係

#### 実行時依存
- `anyhow`: エラーハンドリング
- `clap`: CLI引数解析
- `thiserror`: カスタムエラー型

#### 開発時依存
- `tempfile`: テスト用一時ファイル

## パフォーマンス仕様

### 設計方針
- 大容量ファイル（1MB以上）の効率的処理
- メモリ使用量の最適化
- String操作の最小化

### 制限事項
- 最大include深度: 100（設定可能）
- ファイルサイズ制限: なし（メモリ制限による）
- 同時処理ファイル数: include深度に依存

## 今後の拡張可能性

### 機能拡張
- 条件付きinclude（`#ifdef`等）
- マクロ展開機能
- 並行処理による高速化

### 設定拡張
- 設定ファイル対応
- include検索パスの複数指定
- カスタムinclude構文サポート