# Edax-Reversi Rust 書き直し計画

## 概要

Edax (C99, 約 52,600 行) を Rust で書き直す計画。
安全性・保守性を向上させつつ、元の探索性能を維持することを目標とする。

---

## 現行 C コードの規模

| カテゴリ | 主要ファイル | 行数 (概算) |
|----------|-------------|-------------|
| ビットボード・盤面 | `board.c/h`, `bit.c/h` | 1,900 |
| 着手生成 (flip) | `flip_carry_64.c`, `count_last_flip_carry_64.c` 等 | 3,300 (使用する実装のみ) |
| 探索エンジン | `search.c/h`, `midgame.c`, `endgame.c`, `root.c` | 3,600 |
| 評価関数 | `eval.c/h` | 900 |
| ハッシュテーブル | `hash.c/h` | 800 |
| 並列探索 (YBWC) | `ybwc.c/h` | 780 |
| 定石 | `book.c/h` | 2,700 |
| ゲーム管理 | `play.c/h`, `game.c/h` | 2,960 |
| UI・プロトコル | `edax.c`, `gtp.c`, `xboard.c`, `nboard.c`, `cassio.c`, `ggs.c`, `ui.c/h` | 4,700 |
| ユーティリティ | `util.c/h`, `options.c/h`, `event.c/h`, `stats.c/h` | 2,100 |
| その他 | `base.c`, `opening.c`, `perft.c`, `bench.c`, `obftest.c` 等 | 4,600 |
| **合計** | | **約 28,300** (実際に使用される部分) |

> flip の実装は7種類あるが Rust では1つ (carry ベース + SIMD 版) に絞る。
> `eval_builder.c` (2,200行) は開発ツールなので初期対象外。

---

## Rust プロジェクト構成

```
edax-rs/
├── Cargo.toml
├── crates/
│   ├── edax-core/              # コアエンジン (ライブラリ)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── board.rs        # Board 構造体、ビットボード操作
│   │   │   ├── bit.rs          # ビット演算 (popcount, bsf, bsr)
│   │   │   ├── flip.rs         # 着手生成 (carry ベース)
│   │   │   ├── r#move.rs       # Move, MoveList, Line
│   │   │   ├── empty.rs        # SquareList (空きマス管理)
│   │   │   ├── eval.rs         # 評価関数・重みファイル読込
│   │   │   ├── hash.rs         # トランスポジションテーブル
│   │   │   ├── search.rs       # Search 構造体・制御
│   │   │   ├── midgame.rs      # 中盤探索 (PVS/NWS)
│   │   │   ├── endgame.rs      # 終盤完全読み
│   │   │   ├── root.rs         # ルート探索・反復深化
│   │   │   ├── ybwc.rs         # 並列探索 (YBWC)
│   │   │   ├── book.rs         # 定石データベース
│   │   │   ├── game.rs         # 棋譜データ
│   │   │   ├── opening.rs      # 定石名
│   │   │   ├── stats.rs        # 統計情報
│   │   │   └── constants.rs    # 定数・列挙型
│   │   └── Cargo.toml
│   ├── edax-play/              # ゲーム管理 (ライブラリ)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── play.rs         # Play 構造体 (対局管理)
│   │   │   ├── options.rs      # 設定
│   │   │   └── time.rs         # 時間管理
│   │   └── Cargo.toml
│   └── edax-protocol/          # プロトコル (ライブラリ)
│       ├── src/
│       │   ├── lib.rs
│       │   ├── edax.rs         # Edax CLI プロトコル
│       │   ├── gtp.rs          # Go Text Protocol
│       │   ├── xboard.rs       # XBoard プロトコル
│       │   ├── nboard.rs       # NBoard プロトコル
│       │   └── event.rs        # イベントキュー
│       └── Cargo.toml
├── src/
│   └── main.rs                 # CLI エントリポイント
├── data/                       # eval.dat, book.dat
├── problem/                    # OBF テストファイル
└── benches/
    ├── flip_bench.rs           # flip 関数ベンチマーク
    ├── search_bench.rs         # 探索ベンチマーク
    └── perft_bench.rs          # パフォーマンステスト
```

---

## Phase 1: ビットボードと着手生成

C の心臓部。ここの性能が全体を決める。

### 1-1. 型定義 (`board.rs`, `constants.rs`)

```rust
/// 盤面 — プレイヤーと相手の石をビットボードで表現
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Board {
    pub player: u64,
    pub opponent: u64,
}

/// マス座標 (0=A1 .. 63=H8, 64=PASS, 65=NOMOVE)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Square(pub u8);

impl Square {
    pub const PASS: Square = Square(64);
    pub const NOMOVE: Square = Square(65);
    pub fn to_bit(self) -> u64 { 1u64 << self.0 }
}
```

### 1-2. ビット演算 (`bit.rs`)

C の `bit.c` に相当。Rust 標準ライブラリと LLVM intrinsics を活用:

| C 関数 | Rust 置き換え |
|--------|--------------|
| `bit_count()` (POPCOUNT) | `u64::count_ones()` — LLVM が `popcnt` 命令に最適化 |
| `first_bit()` (BSF) | `u64::trailing_zeros()` — LLVM が `tzcnt`/`bsf` に最適化 |
| `last_bit()` (BSR) | `63 - u64::leading_zeros()` — LLVM が `lzcnt`/`bsr` に最適化 |
| `transpose()` | そのまま移植 (ビットシフト) |
| `vertical_mirror()` | `u64::swap_bytes()` |
| `horizontal_mirror()` | ビットシフトで移植 |

- C の `#ifdef USE_GAS_X64` / `USE_GCC_ARM` の分岐は **不要** — LLVM が自動でアーキテクチャ最適化
- `#[cfg(target_arch = "x86_64")]` で SIMD を追加する余地はあるが、初期は不要

### 1-3. 着手生成 (`flip.rs`)

C の `flip_carry_64.c` をそのまま移植。ルックアップテーブル + キャリー伝搬アルゴリズム。

```rust
/// 各マスの flip 関数 (関数テーブル)
pub static FLIP: [fn(u64, u64) -> u64; 66] = [
    flip_a1, flip_b1, /* ... */ flip_h8, flip_pass,
];

/// A1 に着手した場合の反転石ビットボード
fn flip_a1(player: u64, opponent: u64) -> u64 {
    // carry ベースの実装をそのまま移植
    // ルックアップテーブルは static const で定義
}
```

**注意点:**
- C では各マスに個別関数がある (64+1 関数)。Rust でも同じ構造を維持 (インライン展開のため)
- ルックアップテーブル (`OUTFLANK_*`, `FLIPPED_*_*`, `CONTIG_*`) は `static` 配列で定義
- `count_last_flip` も同様に移植

### 1-4. 盤面操作 (`board.rs`)

```rust
impl Board {
    pub fn new() -> Self { /* 初期配置 */ }
    pub fn do_move(&mut self, sq: Square) -> u64 { /* flip して更新 */ }
    pub fn undo_move(&mut self, sq: Square, flipped: u64) { /* 巻き戻し */ }
    pub fn get_moves(&self) -> u64 { /* 合法手ビットボード */ }
    pub fn can_move(&self) -> bool { self.get_moves() != 0 }
    pub fn pass(&mut self) { std::mem::swap(&mut self.player, &mut self.opponent); }
    pub fn score(&self, n_empties: i32) -> i32 { /* 石差 */ }
    pub fn hash_code(&self) -> u64 { /* Zobrist ハッシュ */ }
    pub fn unique(&self) -> (Board, u8) { /* 8対称の正規形 + 対称番号 */ }
    pub fn stability(&self) -> (i32, i32) { /* 確定石数 */ }
}
```

### Phase 1 の検証
- `problem/fforum-*.obf` の各局面で合法手生成が一致することを確認
- perft (全合法手展開カウント) でC版と数値一致を確認
- `cargo bench` で flip 関数の NPS を C 版と比較

---

## Phase 2: 評価関数

### 2-1. 特徴量システム (`eval.rs`)

C の評価関数は **47 個のパターン特徴量** を三進数でエンコードし、重みテーブルを引く方式:

```rust
/// 評価関数の状態
pub struct Eval {
    feature: [i32; 47],   // 47 パターンの三進数インデックス
    player: u8,           // 視点 (0=黒, 1=白)
}

/// 評価の重み (起動時に eval.dat から読み込み)
pub struct EvalWeight {
    /// weight[player][ply][packed_index]
    /// player: 0/1, ply: 0..60, packed_index: 対称性圧縮後のインデックス
    weight: Vec<Vec<Vec<i16>>>,
}
```

**特徴量の構成 (全47個):**

| 特徴 # | マス数 | パターン | 三進数サイズ |
|--------|--------|---------|-------------|
| 0-3 | 9 | コーナー 3x3 | 19,683 |
| 4-7 | 10 | コーナー拡張 | 59,049 |
| 8-11 | 10 | エッジ拡張 | 59,049 |
| 12-19 | 8 | サイド系 | 6,561 |
| 20-25 | 8 | ライン・対角 | 6,561 |
| 26-29 | 7 | 短い対角 | 2,187 |
| 30-31 | 6 | 短い対角 | 729 |
| 32-33 | 5 | 短い対角 | 243 |
| 34-35 | 4 | 短い対角 | 81 |
| 36 | - | バイアス | 1 |

### 2-2. eval.dat ファイル読み込み

バイナリ形式。ヘッダ + 61 手分 x 114,364 個の `i16` 重み。
対称性テーブル (`EVAL_C9`, `EVAL_C10`, `EVAL_S10`, `EVAL_S8` 等) で展開。

```rust
impl EvalWeight {
    pub fn load(path: &Path) -> Result<Self, EvalError> {
        // ヘッダ検証 ("EDAX" + "EVAL" マジック)
        // エンディアン検出 ("XADE" なら反転)
        // 対称性テーブルで圧縮重みを展開
    }
}
```

### 2-3. 差分更新

着手ごとに全特徴量を再計算せず、変化したマスだけ更新:

```rust
impl Eval {
    /// 着手による特徴量差分更新
    pub fn update(&mut self, sq: Square, flipped: u64) {
        // sq のマスが空→自石: 各特徴量から 2*weight を減算
        // flipped の各マスが相手石→自石: 各特徴量から 1*weight を減算
    }

    /// 評価値を計算
    pub fn score(&self, weights: &EvalWeight, n_empties: i32) -> i32 {
        let ply = 60 - n_empties;
        self.feature.iter().enumerate()
            .map(|(i, &f)| weights.weight[self.player][ply][f as usize] as i32)
            .sum()
    }
}
```

### Phase 2 の検証
- C 版と同じ `eval.dat` を読み込み、同一局面で同一評価値が出ることを確認
- 差分更新の正しさを全更新と比較して検証

---

## Phase 3: ハッシュテーブル

### 3-1. 構造体 (`hash.rs`)

```rust
/// ハッシュエントリのデータ
#[derive(Clone, Copy, Default)]
pub struct HashData {
    pub depth: u8,
    pub selectivity: u8,
    pub cost: u8,
    pub date: u8,
    pub lower: i8,       // -64..64
    pub upper: i8,       // -64..64
    pub move_: [u8; 2],  // 最善手2つ
}

/// ハッシュエントリ
#[derive(Clone)]
pub struct HashEntry {
    pub board: Board,
    pub data: HashData,
}

/// トランスポジションテーブル
pub struct HashTable {
    entries: Vec<HashEntry>,         // N-way セット連想
    locks: Vec<parking_lot::Mutex<()>>,  // セットごとのロック
    mask: usize,
    lock_mask: usize,
    date: u8,
}
```

**C との対応:**
- C は `HASH_N_WAY = 4` のセット連想
- ロックは C では `SpinLock`。Rust では `parking_lot::Mutex` (スピンロック的な実装) を使用
- `HASH_ALIGNED` によるアラインメントは Rust の `#[repr(align)]` で対応

### 3-2. Zobrist ハッシュ

```rust
/// 局面のハッシュ値を計算
pub fn board_hash_code(board: &Board) -> u64 {
    // hash_rank[16][256] テーブルを使用
    // ランクごとに player/opponent の 8-bit パターンをインデックス
}
```

### Phase 3 の検証
- ハッシュ衝突率を C 版と比較
- マルチスレッドアクセス時のデータ整合性テスト

---

## Phase 4: 探索エンジン

最も複雑な部分。C の `search.c`, `midgame.c`, `endgame.c`, `root.c` に対応。

### 4-1. 探索アルゴリズム

Edax は **PVS (Principal Variation Search) + Aspiration Windows** を使用:

```
root_search(board, depth)
  ├── aspiration_window_loop
  │   └── pvs_root(alpha, beta, depth)
  │       ├── first_move: PVS_midgame(-beta, -alpha, depth-1)
  │       └── other_moves: NWS_midgame(-alpha-1, depth-1)
  │           └── if alpha < score < beta: re-search with full window
  │
  ├── midgame (depth > DEPTH_MIDGAME_TO_ENDGAME=15)
  │   ├── PVS_midgame / NWS_midgame
  │   ├── ProbCut pruning
  │   ├── Stability cutoff
  │   ├── Transposition cutoff (TC)
  │   └── Enhanced TC (ETC)
  │
  └── endgame (depth <= 15, n_empties <= depth)
      ├── NWS_endgame (5-10 empties)
      ├── search_solve_4 (4 empties, parity ordering)
      ├── search_solve_3
      ├── search_solve_2 (遅延評価)
      ├── board_score_1 (1-ply exact)
      └── search_solve_0 (石数計算)
```

### 4-2. Search 構造体

```rust
pub struct Search {
    // 盤面状態
    pub board: Board,
    pub empties: SquareList,
    pub n_empties: i32,
    pub player: u8,

    // ハッシュテーブル (3つ)
    pub hash_table: HashTable,      // メイン
    pub pv_table: HashTable,        // PV 用
    pub shallow_table: HashTable,   // 浅い探索用

    // 評価
    pub eval: Eval,

    // 探索パラメータ
    pub depth: i32,
    pub selectivity: i32,
    pub height: i32,
    pub parity: u32,
    pub node_type: Vec<NodeType>,
    pub stop: AtomicStop,

    // 時間管理
    pub time: TimeManager,

    // 結果
    pub result: Arc<Mutex<Result>>,
    pub n_nodes: AtomicU64,

    // 並列探索 (Phase 6 で実装)
    pub tasks: Option<Arc<TaskStack>>,
}
```

### 4-3. 枝刈り技法

全て `settings.h` のフラグに対応する Cargo feature で切り替え可能に:

```toml
[features]
default = ["tc", "sc", "etc", "probcut"]
tc = []          # Transposition Cutoff
sc = []          # Stability Cutoff
etc = []         # Enhanced Transposition Cutoff
probcut = []     # ProbCut (確率的前方枝刈り)
```

**ProbCut の実装:**
```rust
/// 浅い探索の結果から深い探索の結果を確率的に推定して枝刈り
fn probcut(search: &mut Search, alpha: i32, depth: i32, ...) -> Option<i32> {
    // eval_sigma() で誤差推定 (二次多項式モデル)
    // 浅い探索を実行
    // 結果が alpha +/- sigma * t の範囲外なら枝刈り
}
```

### 4-4. 手の並び替え

```rust
/// 手の並び替え (相手の合法手数を減らす手を優先)
fn movelist_evaluate(list: &mut MoveList, search: &Search, hash_data: &HashData) {
    for m in list.iter_mut() {
        m.cost = opponent_mobility_reduction(search, m);
        if m.x == hash_data.move_[0] { m.cost += HASH_MOVE_BONUS; }
    }
    list.sort_by(|a, b| b.cost.cmp(&a.cost));
}
```

### Phase 4 の検証
- `problem/fforum-1-19.obf` の全問題で C 版と同一のスコアを返すことを確認
- 探索ノード数を比較 (多少の差異は手順の違いで許容、大幅な差異はバグ)

---

## Phase 5: 定石データベース

### 5-1. 構造体 (`book.rs`)

```rust
/// 定石の1局面
pub struct Position {
    pub board: Board,
    pub leaf: Link,                  // 未展開の最善手
    pub links: Vec<Link>,           // 展開済みの手とスコア
    pub n_wins: u32,
    pub n_draws: u32,
    pub n_losses: u32,
    pub n_lines: u32,
    pub score: ScoreBound,
    pub level: u8,
    pub done: bool,
    pub todo: bool,
}

#[derive(Clone, Copy)]
pub struct Link {
    pub score: i8,
    pub move_: u8,
}

#[derive(Clone, Copy)]
pub struct ScoreBound {
    pub value: i16,
    pub lower: i16,
    pub upper: i16,
}

/// 定石データベース
pub struct Book {
    positions: HashMap<Board, Position>,  // C はハッシュ配列。Rust は HashMap で十分
    search: Search,
    options: BookOptions,
    need_saving: bool,
}
```

**C との違い:**
- C は `PositionArray` (線形チェイン) によるハッシュテーブル
- Rust では `HashMap<Board, Position>` で十分。Board に `Hash` トレイトを実装
- 全局面は正規形 (`board.unique()`) で格納。検索時に対称変換

### 5-2. ファイル I/O

```rust
impl Book {
    pub fn load(path: &Path) -> Result<Self, BookError> {
        // バイナリ形式: "EDAX" + "BOOK" マジック
        // 各局面: board(16B) + stats(16B) + score(6B) + n_link(1B) + level(1B) + links
    }

    pub fn save(&self, path: &Path) -> Result<(), BookError> { /* 同フォーマット */ }
}
```

### 5-3. 学習・更新

```rust
impl Book {
    pub fn negamax(&mut self) { /* 全局面のスコアをボトムアップ伝搬 */ }
    pub fn deviate(&mut self, threshold: i32) { /* 展開候補をマーク */ }
    pub fn expand(&mut self, board: &Board) { /* 新局面を追加・評価 */ }
    pub fn link(&mut self) { /* 全局面間のリンク構築 */ }
}
```

---

## Phase 6: 並列探索 (YBWC)

### 6-1. YBWC の Rust 実装方針

C の YBWC は以下のモデル:
- **TaskStack**: 事前確保されたスレッドプール (最大64スレッド)
- **Node**: 共有される探索ノード。mutex + condition variable で同期
- **Young Brother Wait**: 最初の手を探索し終えてから残りの手を並列分割

Rust では **rayon** や **crossbeam** の使用も検討したが、YBWC の特殊な制御フロー (投機的分割、途中中断) には合わないため、C と同様のスレッドプール方式を採用:

```rust
pub struct TaskStack {
    tasks: Vec<Mutex<Task>>,
    idle: ArrayQueue<usize>,   // crossbeam の lock-free キュー
    n_tasks: usize,
}

pub struct Task {
    pub search: Search,        // スレッド専用の探索状態
    pub run: AtomicBool,
    pub node: Option<Arc<Node>>,
}

pub struct Node {
    pub best_score: AtomicI32,
    pub best_move: AtomicU8,
    pub alpha: AtomicI32,
    pub beta: i32,             // 不変
    pub n_slaves: AtomicU32,
    pub stop_point: AtomicBool,
    pub moves: Mutex<MoveIterator>,
    pub condvar: Condvar,
}
```

### 6-2. 分割条件

C の `settings.h` から:
- `SPLIT_MIN_DEPTH = 5` — 深さ5以上で分割
- `SPLIT_MIN_MOVES_TODO = 1` — 残り手1以上
- `SPLIT_MAX_SLAVES = 3` — ノードあたり最大3ヘルパー
- 最初の手を探索済みであること (Young Brother Wait 原則)

### 6-3. ハッシュテーブル共有

```rust
// メインの HashTable は Arc で共有
let hash_table = Arc::new(HashTable::new(size));

// 各スレッドの Search は Arc<HashTable> のクローンを持つ
search.hash_table = Arc::clone(&hash_table);
```

ロック粒度: C と同様にセット (N-way バケット) ごとにロック。

---

## Phase 7: ゲーム管理と UI

### 7-1. Play 構造体 (`play.rs`)

```rust
pub struct Play {
    pub board: Board,
    pub initial_board: Board,
    pub search: Search,
    pub result: Result,
    pub book: Arc<Book>,
    pub player: u8,
    pub game: Vec<Move>,          // 着手履歴 (最大80手)
    pub i_game: usize,
    pub state: PlayState,
    pub level: i32,
    pub time: [PlayerTime; 2],
    pub options: Arc<Options>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PlayState {
    Waiting,
    Thinking,
    Pondering,
    Analyzing,
}
```

### 7-2. プロトコル (`edax-protocol` クレート)

```rust
/// プロトコルのトレイト
pub trait Protocol {
    fn init(&mut self, play: &mut Play, book: &mut Book);
    fn handle_command(&mut self, cmd: &str, params: &str, play: &mut Play, book: &mut Book);
}

pub struct EdaxProtocol;    // CLI インタラクティブ
pub struct GtpProtocol;     // Go Text Protocol
pub struct XboardProtocol;  // XBoard/Winboard
pub struct NboardProtocol;  // NBoard
```

### 7-3. イベントループ

```rust
/// stdin からコマンドを読み取るイベントループ
pub fn run_event_loop<P: Protocol>(protocol: &mut P, play: &mut Play) {
    let (tx, rx) = crossbeam_channel::unbounded::<String>();

    // stdin 読み取りスレッド
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            if tx.send(line.unwrap()).is_err() { break; }
        }
    });

    // メインループ
    loop {
        let msg = rx.recv().unwrap();
        let (cmd, params) = parse_command(&msg);
        if cmd == "quit" { break; }
        protocol.handle_command(&cmd, &params, play, book);
    }
}
```

---

## Phase 8: テストとベンチマーク

### 8-1. ユニットテスト

```rust
#[cfg(test)]
mod tests {
    // board: 初期配置、合法手生成、着手実行・巻き戻し
    // flip: 各マス flip 関数の正確性 (既知の局面で検証)
    // eval: 重みファイル読込、評価値計算
    // hash: 格納・検索、衝突時の置換
    // search: 浅い探索で既知の結果と一致
    // book: ファイル読み書きのラウンドトリップ
}
```

### 8-2. 統合テスト

```rust
// tests/fforum.rs
#[test]
fn solve_fforum_1_19() {
    // problem/fforum-1-19.obf の各問題を解き、
    // 正解スコアと一致することを確認
}
```

### 8-3. ベンチマーク

```rust
// benches/flip_bench.rs  — flip 関数の throughput
// benches/search_bench.rs — 1局面の探索 NPS
// benches/perft_bench.rs  — 全着手展開の速度
```

**目標性能:** C 版の 80% 以上の NPS (Rust の安全性コストを考慮)

---

## 依存クレート

| クレート | 用途 | 必要性 |
|---------|------|--------|
| `parking_lot` | 高速 Mutex / RwLock (SpinLock 相当) | 必須 |
| `crossbeam` | Lock-free キュー、スコープドスレッド | 必須 (YBWC) |
| `clap` | CLI 引数パーサ | 推奨 |
| `byteorder` | バイナリファイル読み書き (エンディアン対応) | 必須 (eval.dat/book.dat) |
| `thiserror` | エラー型定義 | 推奨 |
| `criterion` | ベンチマーク | 開発時 |

**外部ライブラリ依存なし** の方針は維持。C の `libm`/`libpthread`/`librt` は Rust 標準ライブラリに内包。

---

## C と Rust の設計差異

### 所有権モデル

| C の設計 | Rust での対応 |
|---------|-------------|
| `Search` が `Board`, `Eval`, `HashTable` を埋め込み (`board[1]`) | `Search` が直接フィールドとして所有 |
| `Play` が `Search` を埋め込み | `Play` が `Search` を所有 |
| `Book` が `Search*` ポインタを保持 | `Book` のメソッドが `&mut Search` を引数で受け取る |
| グローバル `Options options` | `Arc<Options>` を各構造体で共有 |
| `EVAL_WEIGHT` グローバル三次元配列 | `OnceCell<EvalWeight>` で遅延初期化 |
| `flip[]` 関数ポインタ配列 | `static FLIP: [fn(u64,u64)->u64; 66]` |
| `Move` の連結リスト (`next` ポインタ) | `MoveList` 内の配列 + インデックス (`Vec<Move>`) |
| `SquareList` の双方向連結リスト | 独自のインデックスベース双方向リスト |
| `volatile` フラグ (stop, state) | `AtomicBool`, `AtomicI32` 等 |
| `SpinLock` / `Lock` / `Condition` | `parking_lot::Mutex`, `std::sync::Condvar` |

### unsafe の使用箇所 (最小限に抑える)

| 箇所 | 理由 |
|------|------|
| flip テーブルの `static mut` 初期化 | `OnceCell` または `lazy_static` で回避可能 |
| ハッシュテーブルのアラインメント | `#[repr(align(64))]` で対応 |
| SIMD intrinsics (将来の最適化) | `std::arch::x86_64::_mm_*` |

---

## 実装順序とマイルストーン

| Phase | 内容 | 状態 | 検証方法 |
|-------|------|------|----------|
| **1** | ビットボード + 着手生成 | **完了** | perft 一致 (depth 0-8) |
| **2** | 評価関数 | **完了** | C 版と同一スコア (eval.dat 互換) |
| **3** | ハッシュテーブル | **完了** | 単体テスト |
| **4** | 探索エンジン (シングルスレッド) | **完了** | PVS/NWS + 反復深化 |
| **5** | 定石データベース | **完了** | 対称性考慮の保存・読込 |
| **6** | 並列探索 (YBWC) | **完了** | スコープドスレッド方式 |
| **7** | ゲーム管理 + UI プロトコル | **完了** | Edax プロトコル互換 |
| **8** | ベンチマーク + 最適化 | **完了** | 152 テスト全通過 |

---

## 現在の実装状況

### テスト結果
- **ユニットテスト**: 134 件通過
- **統合テスト**: 18 件通過
- **合計**: 152 件全通過

### C 版との互換性
- **perft**: depth 0-8 で C 版と完全一致
- **評価関数**: C 版の `eval.dat` を読み込み、同一局面で同一スコアを返すことを確認
  - 初期局面: -4 (C版: -4)
  - D3 後: +4 (C版: +4)
  - D3 C3 後: -4 (C版: -4)
  - D3 C3 C2 後: +5 (C版: +5)

### 未実装・計画の違い
- **差分評価更新**: C 版は着手ごとに特徴量を差分更新するが、Rust 版は毎回全再計算
- **ProbCut**: 未実装
- **安定石カットオフ (SC)**: 未実装
- **Enhanced Transposition Cutoff (ETC)**: 未実装
- **完全読み最適化**: `search_solve_4/3/2/1/0` の特殊化は未実装
- **GTP/XBoard/NBoard プロトコル**: 未実装 (Edax プロトコルのみ)
- **`book.dat` C 互換フォーマット**: 未実装 (独自バイナリフォーマット)

### 依存クレート
**外部クレートなし** — `std` のみ使用。計画にあった `parking_lot`, `crossbeam`, `clap`, `byteorder` 等は不使用。

---

## リスクと対策

| リスク | 影響 | 対策 |
|--------|------|------|
| flip 関数の性能劣化 | 全体の NPS 低下 | LLVM の最適化を信頼。必要なら `#[inline(always)]` や SIMD 版を追加 |
| YBWC の Rust 実装が複雑 | 開発遅延 | Phase 4 までシングルスレッドで完動させてから着手。最悪 rayon で代替 |
| eval.dat のバイナリ互換性 | 評価が不正 | **解決済み**: C 版と同一局面でスコア一致を確認 |
| 連結リストの所有権問題 (`SquareList`) | 設計が複雑化 | **解決済み**: 配列ベースの実装を採用 |
| ハッシュテーブルの並行アクセス | データ競合 | `Mutex<HashTable>` で保護。C 版より粗い粒度だが安全 |
