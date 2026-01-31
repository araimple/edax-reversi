# Edax-Reversi iOS 移植計画

## 概要

Edax (C99製オセロエンジン) を iOS アプリとして動作させるための移植計画。
コアエンジンを静的ライブラリとしてビルドし、Swift UI から呼び出す構成を目指す。

---

## 全体アーキテクチャ

```
┌─────────────────────────────┐
│  Swift UI (SwiftUI)         │  盤面表示・操作・設定画面
├─────────────────────────────┤
│  EdaxEngine.swift           │  Swift ラッパークラス
├─────────────────────────────┤
│  edax_api.h / edax_api.c    │  C API (コールバック方式)
├─────────────────────────────┤
│  Edax Core (C99 静的ライブラリ) │  探索・評価・定石
└─────────────────────────────┘
```

---

## Phase 1: Xcode プロジェクトとビルド基盤

### 目的
C ソースを iOS 向けにコンパイルできる状態にする。

### 作業内容

1. **Xcode プロジェクト作成**
   - iOS App ターゲット (Swift/SwiftUI)
   - C Static Library ターゲット (`libedax`)
   - `all.c` (ユニティビルド) をライブラリターゲットに追加

2. **コンパイラフラグ設定**
   ```
   -std=c99 -DNDEBUG -O2 -flto
   -arch arm64
   -DPOPCOUNT -DUSE_GCC_ARM
   ```
   - `-Ofast` は iOS では `-O2` に置き換え (安全性のため)
   - `-fwhole-program` は静的ライブラリでは使えないので除外

3. **無効化すべきマクロ**
   - `USE_GAS_X64` — x86 インラインアセンブリ。iOS は ARM64 のみ
   - `USE_MSVC_X64` — Windows 用
   - `MOVE_GENERATOR_SSE` — x86 SSE 命令。`settings.h` で `MOVE_GENERATOR_CARRY` を使用

4. **`settings.h` の確認**
   ```c
   #define MOVE_GENERATOR MOVE_GENERATOR_CARRY  // SSE ではないことを確認
   ```

### 検証基準
- `all.c` が arm64 ターゲットで警告なしにコンパイルできる

---

## Phase 2: プラットフォーム依存コードの修正

### 2-1. アーキテクチャ固有コード (`bit.c`, `board.c`)

| 項目 | 現状 | iOS での対応 |
|------|------|-------------|
| POPCOUNT | x86 ASM / `__builtin_popcountll()` | `__builtin_popcountll()` (Clang 対応済み) |
| Bit Scan (BSF/BSR) | x86 ASM / `__builtin_ctzll()` | `__builtin_ctzll()` / `__builtin_clzll()` |
| MMX 演算 (`board.c`) | `USE_GAS_MMX` で x86 MMX ASM | 純 C フォールバックパスを使用 |
| アトミック加算 (`util.h`) | x86 `lock xaddq` | `__atomic_fetch_add()` または pthread mutex |

- `bit.c` は `USE_GCC_ARM` 定義で ARM パスが既にある。動作確認が必要
- `board.c` の MMX パスは `#ifdef USE_GAS_MMX` で囲まれており、定義しなければ自動的にスキップ

### 2-2. スレッド関連 (`util.h`, `util.c`, `ybwc.c`)

| 項目 | 現状 | iOS での対応 |
|------|------|-------------|
| スレッド生成 | `pthread_create()` | そのまま使用可 (iOS は pthread 対応) |
| SpinLock | `OSSpinLock` (macOS, **非推奨**) | `os_unfair_lock` に置換 |
| CPU アフィニティ | Linux: `pthread_setaffinity_np()` | iOS では不要 (既にno-opフォールバックあり) |
| CPU コア数取得 | macOS: `sysctl()` | `sysctl()` がそのまま使える |
| アトミック操作 | x86 ASM | `<stdatomic.h>` または GCC builtins |

**SpinLock 修正** (`util.h`):
```c
// 変更前 (macOS パス)
#include <libkern/OSAtomic.h>
typedef OSSpinLock SpinLock;

// 変更後 (iOS/macOS 共通)
#include <os/lock.h>
typedef os_unfair_lock SpinLock;
#define spin_lock(s)   os_unfair_lock_lock(s)
#define spin_unlock(s) os_unfair_lock_unlock(s)
#define spin_init(s)   *(s) = OS_UNFAIR_LOCK_INIT
#define spin_free(s)   (void)(s)
```

### 2-3. 時刻取得 (`util.c`)

- `clock_gettime(CLOCK_MONOTONIC)` — iOS 10+ で使用可能。変更不要
- `gettimeofday()` フォールバックもそのまま動作

---

## Phase 3: ファイル I/O の抽象化

### 問題点
- 現状は相対パス `"data/eval.dat"`, `"data/book.dat"` を `fopen()` で開く
- iOS にはカレントディレクトリの概念がない
- アプリサンドボックス内のパスが必要

### 対応方針

1. **読み取り専用データ** → App Bundle に同梱
   - `eval.dat` (評価関数の重み)
   - `book.dat` (定石データ)

2. **読み書きデータ** → Documents ディレクトリ
   - `book.dat` (学習で更新する場合)
   - `game.ggf` (棋譜保存)

3. **`options.c` の修正**
   ```c
   // 初期化時に iOS 側から渡されたベースパスを使う
   void edax_set_data_dir(const char *bundle_dir, const char *docs_dir);
   ```
   - `eval_file` → `{bundle_dir}/eval.dat`
   - `book_file` → `{docs_dir}/book.dat`
   - `game_file` → `{docs_dir}/game.ggf`

4. **Swift 側でパスを渡す**
   ```swift
   let bundlePath = Bundle.main.resourcePath!
   let docsPath = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first!.path
   edax_set_data_dir(bundlePath, docsPath)
   ```

### 必要なデータファイル
- `eval.dat` — 必須。これがないとエンジンが動かない。入手先の確認が必要
- `book.dat` — 任意。なくても動作するが棋力が下がる

---

## Phase 4: C API の設計 (`edax_api.h` / `edax_api.c`)

### 問題点
- 現状は全プロトコル (edax, gtp, xboard, nboard, ggs, cassio) が `stdin`/`stdout` に依存
- iOS アプリでは標準入出力は使えない

### 対応方針: コールバック方式の C API を新規作成

```c
// edax_api.h
#ifndef EDAX_API_H
#define EDAX_API_H

#include "board.h"
#include "move.h"

// 探索結果
typedef struct {
    int best_move;       // 最善手 (A1=0 ... H8=63)
    int score;           // 評価値
    int depth;           // 探索深さ
    long long nodes;     // 探索ノード数
    double time;         // 思考時間 (秒)
    char pv[256];        // 読み筋 (文字列)
} EdaxSearchResult;

// コールバック
typedef void (*EdaxSearchCallback)(const EdaxSearchResult *result, void *context);

// 初期化・終了
int  edax_init(const char *bundle_dir, const char *docs_dir, int n_threads);
void edax_cleanup(void);

// 盤面操作
void edax_new_game(void);
int  edax_play_move(int move);              // 手を打つ (0-63)
int  edax_get_legal_moves(unsigned long long *moves);  // 合法手ビットボード
void edax_get_board(unsigned long long *player, unsigned long long *opponent);
int  edax_get_turn(void);                   // 0=黒, 1=白
int  edax_can_move(void);
void edax_undo(void);

// 探索
void edax_search_start(int depth, int time_ms, EdaxSearchCallback cb, void *context);
void edax_search_stop(void);

// 定石
int  edax_book_move(void);                  // 定石手を返す (-1 = なし)

// ユーティリティ
const char* edax_move_to_string(int move);  // "D3" 等
int  edax_string_to_move(const char *str);

#endif
```

### 実装のポイント
- `play.c` の `Play` 構造体を内部で保持し、API 関数から操作
- `search.c` の `search_run()` をバックグラウンドスレッドで実行
- コールバックは探索スレッドから呼ばれる。Swift 側で `DispatchQueue.main.async` に転送

---

## Phase 5: Swift ラッパーと UI

### Bridging Header (`EdaxBridge.h`)
```c
#import "edax_api.h"
```

### Swift ラッパー (`EdaxEngine.swift`)
```swift
import Foundation

@MainActor
class EdaxEngine: ObservableObject {
    @Published var board: [[Int]] = []    // 盤面状態
    @Published var isThinking = false
    @Published var lastResult: SearchResult?

    func initialize() {
        let bundle = Bundle.main.resourcePath!
        let docs = FileManager.default.urls(for: .documentDirectory,
                                             in: .userDomainMask).first!.path
        edax_init(bundle, docs, 2)  // 2スレッド
    }

    func playMove(_ square: Int) {
        edax_play_move(Int32(square))
        updateBoard()
    }

    func startSearch(depth: Int) {
        isThinking = true
        edax_search_start(Int32(depth), 5000, { result, ctx in
            guard let result = result else { return }
            DispatchQueue.main.async {
                let engine = Unmanaged<EdaxEngine>.fromOpaque(ctx!).takeUnretainedValue()
                engine.lastResult = SearchResult(from: result.pointee)
                engine.isThinking = false
            }
        }, Unmanaged.passUnretained(self).toOpaque())
    }

    func cleanup() { edax_cleanup() }
}
```

### UI 構成 (SwiftUI)
- `BoardView` — 8x8 盤面表示 (石の配置、合法手ハイライト)
- `ControlView` — 新規ゲーム、アンドゥ、思考開始/停止
- `InfoView` — 評価値、探索深さ、読み筋の表示
- `SettingsView` — スレッド数、思考時間、難易度設定

---

## Phase 6: テストとチューニング

### テスト項目

1. **ビルド検証**
   - arm64 シミュレータおよび実機でコンパイルが通ること
   - 警告ゼロ (特にポインタ幅、符号付き/符号なし変換)

2. **基本動作**
   - `eval.dat` の読み込み成功
   - 合法手生成が正しい (既知の局面で検証)
   - 探索が正しい結果を返す (`problem/fforum-1-19.obf` の解答と一致)

3. **スレッド安全性**
   - マルチスレッド探索中の UI 操作
   - 探索中断 (`edax_search_stop`) の即時性

4. **パフォーマンス**
   - iPhone 実機での NPS (nodes per second) 計測
   - メモリ使用量の確認 (ハッシュテーブルサイズ調整)
   - バッテリー消費の確認

5. **エッジケース**
   - パス (合法手なし) の処理
   - 対局終了判定
   - メモリ不足時のハッシュテーブル縮小

---

## 修正が必要なファイル一覧

| 優先度 | ファイル | 修正内容 |
|--------|----------|----------|
| 高 | `util.h` | `OSSpinLock` → `os_unfair_lock`、アトミック操作修正 |
| 高 | `options.c` | データファイルパスの外部注入対応 |
| 高 | 新規: `edax_api.h/c` | iOS 向け C API |
| 高 | `settings.h` | `MOVE_GENERATOR_SSE` を使わないことの確認 |
| 中 | `main.c` | iOS ではエントリポイント不要 (API 経由で初期化) |
| 中 | `all.c` | `edax_api.c` のインクルード追加 |
| 中 | `board.c` | MMX パスが無効化されていることの確認 |
| 低 | `util.c` | CPU コア数取得が iOS で動くことの確認 |
| 低 | `bit.c` | ARM64 ビルトインの動作確認 |

---

## 制約と注意事項

- **iOS は ARM64 のみ** — 32-bit (ARMv7) は iOS 11 以降サポート外
- **App Store 審査** — GPL v2 ライセンスの取り扱いに注意。ソースコード公開義務
- **`eval.dat` の入手** — Edax 公式サイトからダウンロードするか、自前で学習が必要
- **バックグラウンド実行** — iOS はバックグラウンドで長時間 CPU を使えない。探索はフォアグラウンド限定
- **メモリ制限** — iPhone のアプリメモリ上限 (~1-2GB)。ハッシュテーブルサイズを制限する必要あり
