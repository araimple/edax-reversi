# CLAUDE.md

## Project Overview

Edax is a high-performance Othello (Reversi) engine written in C99. It uses bitboard representation, parallel search (YBWC), and an opening book with learning capability. Author: Richard Delorme. License: GPL v2+.

A Rust port (`edax-rs`) is under active development in `edax-rs/`.

## C Version Build

```bash
cd src
make build                    # default: ARCH=x64-modern COMP=icc OS=linux
make build COMP=gcc           # use GCC
make build ARCH=x64 COMP=gcc OS=linux    # explicit options
make pgo-build COMP=gcc       # profile-guided optimization build
make debug COMP=gcc           # debug build with symbols
make clean                    # clean build artifacts
```

### Key build variables

- `ARCH`: `x64-modern` (default, with popcount), `x64`, `x32`, `ARM`, `ARMv7`
- `COMP`: `icc` (default), `gcc`, `g++`, `clang`
- `OS`: `linux` (default), `windows`, `osx`, `android`

### CI build commands (from GitHub Actions)

- Linux: `make build ARCH=x64-modern COMP=gcc OS=linux`
- Windows: `make build ARCH=x64 COMP=gcc OS=windows`
- macOS: `make build ARCH=x64-modern COMP=gcc OS=osx`

## Rust Version Build

```bash
cd edax-rs
cargo build                   # debug build
cargo build --release         # optimized release build
cargo test                    # run all tests (152 tests)
cargo run -p edax-cli -- bench          # search benchmark
cargo run -p edax-cli -- interactive    # interactive mode
cargo run -p edax-cli -- match          # AI match
```

`eval.dat` must be accessible at runtime. The Rust code searches these paths:
- `eval.dat`, `data/eval.dat`, `../data/eval.dat`, `../../data/eval.dat`
- Or set `EDAX_EVAL` environment variable to the file path

## Project Structure

```
src/           # C source and headers (~74 files), Makefile, NMakefile (MSVC)
include/       # Compatibility headers (stdbool.h)
problem/       # OBF test problem files (fforum-*.obf)
data/          # eval.dat (evaluation weights, 13.9 MB)
edax-rs/       # Rust port (workspace)
├── Cargo.toml
├── Cargo.lock
└── crates/
    ├── edax-core/           # Core engine library
    │   ├── src/
    │   │   ├── lib.rs       # Module declarations
    │   │   ├── board.rs     # Bitboard representation, move execution
    │   │   ├── bit.rs       # Bit manipulation (transpose, mirror)
    │   │   ├── flip.rs      # Move flip generation (carry-based)
    │   │   ├── perft.rs     # Performance test (move generation counting)
    │   │   ├── eval.rs      # 47-feature pattern evaluation (C-compatible)
    │   │   ├── hash.rs      # Transposition table (4-way set-associative)
    │   │   ├── search.rs    # PVS/NWS alpha-beta search, iterative deepening
    │   │   ├── parallel.rs  # YBWC parallel search (scoped threads)
    │   │   ├── book.rs      # Opening book with symmetry-aware lookup
    │   │   ├── protocol.rs  # Edax text protocol engine
    │   │   ├── ai.rs        # AI players (Random, Greedy, Mobility, Search)
    │   │   ├── game.rs      # Game runner
    │   │   └── bench.rs     # Benchmarks
    │   └── tests/
    │       └── integration.rs  # Integration tests (18 tests)
    └── edax-cli/            # CLI binary
        └── src/main.rs      # Interactive, match, tournament, bench modes
```

### Key C source files

- `main.c` — Entry point
- `all.c` — Unity build file (includes all sources for LTO)
- `board.c/h` — Bitboard representation, move validation
- `search.c/h` — Alpha-beta search engine
- `eval.c/h` — Position evaluation (47 pattern features, eval.dat)
- `midgame.c`, `endgame.c` — Search strategies by game phase
- `ybwc.c/h` — Young Brother Wait Concept (parallel search)
- `hash.c/h` — Transposition table
- `move.c/h` — Move generation
- `book.c/h` — Opening book
- `flip_*.c` — Multiple optimized move-flip implementations (SSE, bitboard, carry, etc.)
- `edax.c`, `cassio.c`, `ggs.c`, `gtp.c`, `nboard.c`, `xboard.c` — Protocol interfaces
- `ui.c/h`, `play.c/h` — UI event handling and game play
- `options.c/h` — CLI/config parsing
- `util.c/h` — Threading, string, math utilities

## Testing

### C version

Problem solving with OBF files in `problem/` directory:

```bash
./bin/mEdax -solve problem/fforum-1-19.obf
```

Performance testing (perft) and benchmarks are built into the binary via `-count` and `-bench` flags.

### Rust version

```bash
cd edax-rs
cargo test                              # all 152 tests
cargo test -p edax-core --lib           # unit tests only (134)
cargo test -p edax-core --test integration  # integration tests only (18)
cargo test -p edax-core --lib eval      # eval module tests only
```

## Dependencies

### C version
Minimal — no third-party libraries. System libs only: `libm`, `libpthread`, `librt` (Linux).

### Rust version
No third-party crates. Standard library only (`std`).

## Code Style

### C version
- C99 standard with `-pedantic -W -Wall -Wextra`
- Bitboard-heavy code with `unsigned long long` for 64-bit board representation
- Doxygen-style comments for documentation
- Unity build approach via `all.c` for whole-program optimization

### Rust version
- Rust 2021 edition
- `u64` for bitboard representation
- `#[rustfmt::skip]` for large constant arrays
- Tests in `#[cfg(test)] mod tests` blocks within each module

## Rust Port: C Compatibility

The Rust evaluation function produces **identical scores** to the C version:
- Loads the same `eval.dat` weight file (47-feature pattern evaluation)
- Same feature-to-coordinate mapping (EVAL_F2X)
- Same symmetry table construction (C9, C10, S10, S8, S7, S6, S5, S4)
- Same unpacking of packed weights from eval.dat
- Same `search_eval_0` scoring formula: sum 47 weights, +/-64, /128, clamp [-63,63]
- Perft results verified identical to C version (depths 0-8)
