# Edax Multi-Language Port Documentation

This document describes the multi-language port of the Edax Othello/Reversi engine.

## Overview

The Edax engine has been ported from C to multiple languages:

| Language | Directory | Status | Tests |
|----------|-----------|--------|-------|
| C (original) | `src/` | Complete | 89 tests |
| Rust | `edax-rs/` | Complete | 185 tests |
| Swift | `swift/` | Complete | 50+ tests |
| Kotlin | `kotlin/` | Complete | 50+ tests |

All implementations share:
- Identical bitboard representation (64-bit player/opponent)
- Same parallel prefix move generation algorithm
- Same direction-based flip calculation
- Identical perft results for verification

## Bitboard Representation

Each square maps to a bit in a 64-bit integer:

```
Square Index:
  A B C D E F G H
1  0  1  2  3  4  5  6  7
2  8  9 10 11 12 13 14 15
3 16 17 18 19 20 21 22 23
4 24 25 26 27 28 29 30 31
5 32 33 34 35 36 37 38 39
6 40 41 42 43 44 45 46 47
7 48 49 50 51 52 53 54 55
8 56 57 58 59 60 61 62 63
```

Initial position constants:
- `INIT_PLAYER = 0x0000000810000000` (E4=28, D5=35)
- `INIT_OPPONENT = 0x0000001008000000` (D4=27, E5=36)
- `INIT_MOVES = 0x0000102004080000` (D3=19, C4=26, F5=37, E6=44)

## Perft Verification

All implementations produce identical perft results:

| Depth | Nodes |
|-------|-------|
| 0 | 1 |
| 1 | 4 |
| 2 | 12 |
| 3 | 56 |
| 4 | 244 |
| 5 | 1,396 |
| 6 | 8,200 |
| 7 | 55,092 |
| 8 | 390,216 |

## Test Categories

### Board Operations
- `test_board_init`: Initial position setup
- `test_board_pass`: Turn swap (player ↔ opponent)
- `test_board_count_empties`: Empty square counting
- `test_board_get_square_color`: Square state query (0=player, 1=opponent, 2=empty)

### Move Generation
- `test_get_moves_initial`: 4 legal moves from initial position
- `test_get_moves_no_moves`: No moves when no opponent discs
- `test_get_moves_no_horizontal_wrap`: Edge masking prevents wrap-around
- `test_get_moves_all_8_directions`: All 8 directions tested

### Flip Calculation
- `test_flip_d3`: D3 (19) flips D4 (27) in +8 direction
- `test_flip_c4`: C4 (26) flips D4 (27) in +1 direction
- `test_flip_f5`: F5 (37) flips E5 (36) in -1 direction
- `test_flip_e6`: E6 (44) flips E5 (36) in -8 direction
- `test_flip_multiple_directions`: E8 flips 6 discs vertically
- `test_flip_long_diagonal`: H8 flips 6 discs diagonally

### Game State
- `test_board_is_pass`: Pass detection
- `test_board_is_game_over`: Game termination (neither player can move)
- `test_board_score`: Score calculation with empty square allocation
- `test_mobility`: Legal move counting

### Symmetry Operations
- `test_transpose`: A1-H8 diagonal reflection
- `test_horizontal_mirror`: Column swap (A↔H, B↔G, etc.)
- `test_vertical_mirror`: Row swap (1↔8, 2↔7, etc.)

## Language-Specific Details

### C Test Suite (`test/`)

```bash
cd test
gcc -std=c99 -Wall -Wextra -O2 -o test_minimal test_minimal.c
./test_minimal
```

Files:
- `test_framework.h`: Assertion macros
- `test_minimal.c`: 89 standalone tests
- `test_board.c`: Tests using edax modules
- `test_core.c`: Core functionality tests

### Rust (`edax-rs/`)

```bash
cd edax-rs
cargo test                    # Run all 185 tests
cargo test -p edax-core board # Run board tests only
```

Key modules:
- `board.rs`: Board struct with 49 tests
- `flip.rs`: Flip calculation
- `perft.rs`: Perft with 9 tests
- `bit.rs`: Bit manipulation with 10 tests

### Swift (`swift/`)

```bash
cd swift
swift test
```

Package structure:
- `Sources/EdaxCore/Board.swift`: Board implementation
- `Sources/EdaxCore/Bit.swift`: Bit operations
- `Sources/EdaxCore/Perft.swift`: Perft function
- `Tests/EdaxCoreTests/`: Test files

### Kotlin (`kotlin/`)

```bash
cd kotlin
gradle test
```

Project structure:
- `src/main/kotlin/edax/Board.kt`: Board class
- `src/main/kotlin/edax/Bit.kt`: Bit functions
- `src/main/kotlin/edax/Perft.kt`: Perft function
- `src/test/kotlin/edax/`: Test classes

## Algorithm Details

### Parallel Prefix Move Generation

The move generation uses parallel prefix algorithm for efficiency:

```
For each direction (8 total):
  1. Apply edge mask (for horizontal/diagonal directions)
  2. Shift and AND with opponent to find potential flips
  3. Repeat 6 times (max chain length)
  4. Final shift gives candidate moves
  5. AND with empty squares for legal moves
```

### Flip Calculation

For each of 8 directions:
1. Start from move position
2. Shift in direction, collecting opponent discs
3. If player disc found at end, all collected discs flip
4. Otherwise, no flip in this direction

### Edge Masking

Horizontal and diagonal directions use mask `0x7E7E7E7E7E7E7E7E` to prevent wrap-around:
- Excludes columns A (bit 0) and H (bit 7) of each row
- Vertical directions use full mask `0xFFFFFFFFFFFFFFFF`

## Contributing

When adding new test cases:
1. Add to C test suite first (`test/test_minimal.c`)
2. Verify with `./test_minimal`
3. Add matching tests to Rust, Swift, and Kotlin
4. Ensure perft results match across all implementations
