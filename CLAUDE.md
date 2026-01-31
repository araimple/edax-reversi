# CLAUDE.md

## Project Overview

Edax is a high-performance Othello (Reversi) engine written in C99. It uses bitboard representation, parallel search (YBWC), and an opening book with learning capability. Author: Richard Delorme. License: GPL v2+.

## Build

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

## Project Structure

```
src/           # All C source and headers (~74 files), Makefile, NMakefile (MSVC)
include/       # Compatibility headers (stdbool.h)
problem/       # OBF test problem files (fforum-*.obf)
```

### Key source files

- `main.c` — Entry point
- `all.c` — Unity build file (includes all sources for LTO)
- `board.c/h` — Bitboard representation, move validation
- `search.c/h` — Alpha-beta search engine
- `eval.c/h` — Position evaluation
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

Problem solving with OBF files in `problem/` directory:

```bash
./bin/mEdax -solve problem/fforum-1-19.obf
```

Performance testing (perft) and benchmarks are built into the binary via `-count` and `-bench` flags.

## Dependencies

Minimal — no third-party libraries. System libs only: `libm`, `libpthread`, `librt` (Linux).

## Code Style

- C99 standard with `-pedantic -W -Wall -Wextra`
- Bitboard-heavy code with `unsigned long long` for 64-bit board representation
- Doxygen-style comments for documentation
- Unity build approach via `all.c` for whole-program optimization
