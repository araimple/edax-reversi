/**
 * @file test_minimal.c
 * @brief Minimal standalone tests - board and move generation only
 */

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>

/* Test framework */
static int tests_run = 0;
static int tests_passed = 0;
static int tests_failed = 0;

#define TEST_ASSERT(condition, msg) do { \
    tests_run++; \
    if (condition) { \
        tests_passed++; \
    } else { \
        tests_failed++; \
        printf("  FAIL: %s (line %d): %s\n", __func__, __LINE__, msg); \
    } \
} while(0)

#define TEST_ASSERT_EQ(expected, actual, msg) do { \
    tests_run++; \
    if ((expected) == (actual)) { \
        tests_passed++; \
    } else { \
        tests_failed++; \
        printf("  FAIL: %s (line %d): %s (expected %lld, got %lld)\n", \
               __func__, __LINE__, msg, (long long)(expected), (long long)(actual)); \
    } \
} while(0)

#define TEST_ASSERT_EQ_HEX(expected, actual, msg) do { \
    tests_run++; \
    if ((expected) == (actual)) { \
        tests_passed++; \
    } else { \
        tests_failed++; \
        printf("  FAIL: %s (line %d): %s (expected 0x%llx, got 0x%llx)\n", \
               __func__, __LINE__, msg, (unsigned long long)(expected), (unsigned long long)(actual)); \
    } \
} while(0)

#define RUN_TEST(test_func) do { \
    printf("Running %s...\n", #test_func); \
    int before = tests_failed; \
    test_func(); \
    if (tests_failed == before) printf("  OK\n"); \
} while(0)

#define TEST_SUMMARY() do { \
    printf("\n========================================\n"); \
    printf("Tests: %d, Passed: %d, Failed: %d\n", tests_run, tests_passed, tests_failed); \
    printf("========================================\n"); \
} while(0)

/* ================================================================
 * Minimal board implementation for testing
 * ================================================================ */

typedef struct {
    uint64_t player;
    uint64_t opponent;
} Board;

#define INIT_PLAYER   0x0000000810000000ULL
#define INIT_OPPONENT 0x0000001008000000ULL
#define INIT_MOVES    0x0000102004080000ULL

static inline int popcount64(uint64_t x) {
    return __builtin_popcountll(x);
}

static inline int first_bit64(uint64_t x) {
    return __builtin_ctzll(x);
}

void board_init(Board *board) {
    board->player = INIT_PLAYER;
    board->opponent = INIT_OPPONENT;
}

void board_pass(Board *board) {
    uint64_t tmp = board->player;
    board->player = board->opponent;
    board->opponent = tmp;
}

int board_count_empties(const Board *board) {
    return 64 - popcount64(board->player | board->opponent);
}

int board_get_square_color(const Board *board, int x) {
    return 2 - 2 * ((board->player >> x) & 1) - ((board->opponent >> x) & 1);
}

/* ================================================================
 * get_moves implementation (standard parallel prefix algorithm)
 * ================================================================ */

uint64_t get_moves(uint64_t P, uint64_t O) {
    uint64_t mask = O & 0x7E7E7E7E7E7E7E7EULL;
    uint64_t moves, flip;
    uint64_t empty = ~(P | O);

    /* Right direction (+1) */
    flip = mask & (P << 1);
    flip |= mask & (flip << 1);
    flip |= mask & (flip << 1);
    flip |= mask & (flip << 1);
    flip |= mask & (flip << 1);
    flip |= mask & (flip << 1);
    moves = flip << 1;

    /* Left direction (-1) */
    flip = mask & (P >> 1);
    flip |= mask & (flip >> 1);
    flip |= mask & (flip >> 1);
    flip |= mask & (flip >> 1);
    flip |= mask & (flip >> 1);
    flip |= mask & (flip >> 1);
    moves |= flip >> 1;

    /* Down direction (+8) */
    flip = O & (P << 8);
    flip |= O & (flip << 8);
    flip |= O & (flip << 8);
    flip |= O & (flip << 8);
    flip |= O & (flip << 8);
    flip |= O & (flip << 8);
    moves |= flip << 8;

    /* Up direction (-8) */
    flip = O & (P >> 8);
    flip |= O & (flip >> 8);
    flip |= O & (flip >> 8);
    flip |= O & (flip >> 8);
    flip |= O & (flip >> 8);
    flip |= O & (flip >> 8);
    moves |= flip >> 8;

    /* Down-right direction (+9) */
    flip = mask & (P << 9);
    flip |= mask & (flip << 9);
    flip |= mask & (flip << 9);
    flip |= mask & (flip << 9);
    flip |= mask & (flip << 9);
    flip |= mask & (flip << 9);
    moves |= flip << 9;

    /* Up-left direction (-9) */
    flip = mask & (P >> 9);
    flip |= mask & (flip >> 9);
    flip |= mask & (flip >> 9);
    flip |= mask & (flip >> 9);
    flip |= mask & (flip >> 9);
    flip |= mask & (flip >> 9);
    moves |= flip >> 9;

    /* Down-left direction (+7) */
    flip = mask & (P << 7);
    flip |= mask & (flip << 7);
    flip |= mask & (flip << 7);
    flip |= mask & (flip << 7);
    flip |= mask & (flip << 7);
    flip |= mask & (flip << 7);
    moves |= flip << 7;

    /* Up-right direction (-7) */
    flip = mask & (P >> 7);
    flip |= mask & (flip >> 7);
    flip |= mask & (flip >> 7);
    flip |= mask & (flip >> 7);
    flip |= mask & (flip >> 7);
    flip |= mask & (flip >> 7);
    moves |= flip >> 7;

    return moves & empty;
}

/* ================================================================
 * flip implementation
 * ================================================================ */

uint64_t flip_disc(int sq, uint64_t P, uint64_t O) {
    uint64_t move = 1ULL << sq;
    uint64_t flip = 0;
    uint64_t f;
    int dir;

    /* 8 directions: +1, -1, +8, -8, +9, -9, +7, -7 */
    static const int directions[] = {1, -1, 8, -8, 9, -9, 7, -7};
    static const uint64_t masks[] = {
        0x7E7E7E7E7E7E7E7EULL, 0x7E7E7E7E7E7E7E7EULL,
        0xFFFFFFFFFFFFFFFFULL, 0xFFFFFFFFFFFFFFFFULL,
        0x7E7E7E7E7E7E7E7EULL, 0x7E7E7E7E7E7E7E7EULL,
        0x7E7E7E7E7E7E7E7EULL, 0x7E7E7E7E7E7E7E7EULL
    };

    for (dir = 0; dir < 8; dir++) {
        uint64_t m = masks[dir] & O;
        int d = directions[dir];
        f = 0;
        uint64_t x = move;

        if (d > 0) {
            x <<= d;
            while (x & m) {
                f |= x;
                x <<= d;
            }
            if (x & P) flip |= f;
        } else {
            x >>= (-d);
            while (x & m) {
                f |= x;
                x >>= (-d);
            }
            if (x & P) flip |= f;
        }
    }

    return flip;
}

/* ================================================================
 * Tests
 * ================================================================ */

void test_board_init(void) {
    Board board;
    board_init(&board);

    TEST_ASSERT_EQ_HEX(INIT_PLAYER, board.player, "initial player bitboard");
    TEST_ASSERT_EQ_HEX(INIT_OPPONENT, board.opponent, "initial opponent bitboard");
    TEST_ASSERT_EQ(2, popcount64(board.player), "player disc count");
    TEST_ASSERT_EQ(2, popcount64(board.opponent), "opponent disc count");
    TEST_ASSERT_EQ(60, board_count_empties(&board), "empty count");
}

void test_get_moves_initial(void) {
    Board board;
    board_init(&board);

    uint64_t moves = get_moves(board.player, board.opponent);

    TEST_ASSERT_EQ(4, popcount64(moves), "initial position has 4 moves");
    TEST_ASSERT_EQ_HEX(INIT_MOVES, moves, "initial legal moves");
    TEST_ASSERT((moves & (1ULL << 19)) != 0, "D3 is legal");
    TEST_ASSERT((moves & (1ULL << 26)) != 0, "C4 is legal");
    TEST_ASSERT((moves & (1ULL << 37)) != 0, "F5 is legal");
    TEST_ASSERT((moves & (1ULL << 44)) != 0, "E6 is legal");
}

void test_get_moves_no_moves(void) {
    Board board;
    board.player = 0x0000000000000001ULL;
    board.opponent = 0x0000000000000000ULL;

    uint64_t moves = get_moves(board.player, board.opponent);
    TEST_ASSERT_EQ(0, moves, "no opponent means no moves");
}

void test_flip_d3(void) {
    Board board;
    board_init(&board);

    /* D3 (19) flips D4 (27) in the +8 (down) direction */
    uint64_t flipped = flip_disc(19, board.player, board.opponent);
    TEST_ASSERT_EQ_HEX(1ULL << 27, flipped, "D3 flips D4");
    TEST_ASSERT_EQ(1, popcount64(flipped), "D3 flips 1 disc");
}

void test_flip_c4(void) {
    Board board;
    board_init(&board);

    uint64_t flipped = flip_disc(26, board.player, board.opponent);
    TEST_ASSERT_EQ_HEX(1ULL << 27, flipped, "C4 flips D4");
    TEST_ASSERT_EQ(1, popcount64(flipped), "C4 flips 1 disc");
}

void test_flip_f5(void) {
    Board board;
    board_init(&board);

    uint64_t flipped = flip_disc(37, board.player, board.opponent);
    TEST_ASSERT_EQ_HEX(1ULL << 36, flipped, "F5 flips E5");
    TEST_ASSERT_EQ(1, popcount64(flipped), "F5 flips 1 disc");
}

void test_flip_e6(void) {
    Board board;
    board_init(&board);

    uint64_t flipped = flip_disc(44, board.player, board.opponent);
    TEST_ASSERT_EQ_HEX(1ULL << 36, flipped, "E6 flips E5");
    TEST_ASSERT_EQ(1, popcount64(flipped), "E6 flips 1 disc");
}

void test_flip_no_flip(void) {
    Board board;
    board_init(&board);

    uint64_t flipped = flip_disc(0, board.player, board.opponent);
    TEST_ASSERT_EQ(0, flipped, "A1 is illegal, no flips");
}

void test_board_pass(void) {
    Board board;
    board_init(&board);

    uint64_t orig_player = board.player;
    uint64_t orig_opponent = board.opponent;

    board_pass(&board);

    TEST_ASSERT_EQ_HEX(orig_opponent, board.player, "pass swaps player");
    TEST_ASSERT_EQ_HEX(orig_player, board.opponent, "pass swaps opponent");
}

void test_board_count_empties(void) {
    Board board;

    board_init(&board);
    TEST_ASSERT_EQ(60, board_count_empties(&board), "initial has 60 empties");

    board.player = 0xFFFFFFFFFFFFFFFFULL;
    board.opponent = 0;
    TEST_ASSERT_EQ(0, board_count_empties(&board), "full player has 0 empties");

    board.player = 0;
    board.opponent = 0;
    TEST_ASSERT_EQ(64, board_count_empties(&board), "empty board has 64 empties");
}

void test_board_get_square_color(void) {
    Board board;
    board_init(&board);

    TEST_ASSERT_EQ(0, board_get_square_color(&board, 28), "E4 is player (0)");
    TEST_ASSERT_EQ(0, board_get_square_color(&board, 35), "D5 is player (0)");
    TEST_ASSERT_EQ(1, board_get_square_color(&board, 27), "D4 is opponent (1)");
    TEST_ASSERT_EQ(1, board_get_square_color(&board, 36), "E5 is opponent (1)");
    TEST_ASSERT_EQ(2, board_get_square_color(&board, 0), "A1 is empty (2)");
}

void test_bit_count(void) {
    TEST_ASSERT_EQ(0, popcount64(0), "popcount(0) = 0");
    TEST_ASSERT_EQ(1, popcount64(1), "popcount(1) = 1");
    TEST_ASSERT_EQ(64, popcount64(0xFFFFFFFFFFFFFFFFULL), "popcount(all 1s) = 64");
    TEST_ASSERT_EQ(32, popcount64(0xAAAAAAAAAAAAAAAAULL), "popcount(alternating) = 32");
    TEST_ASSERT_EQ(4, popcount64(INIT_PLAYER | INIT_OPPONENT), "initial discs = 4");
}

void test_first_bit(void) {
    TEST_ASSERT_EQ(0, first_bit64(1ULL), "first_bit(1) = 0");
    TEST_ASSERT_EQ(63, first_bit64(0x8000000000000000ULL), "first_bit(high) = 63");
    TEST_ASSERT_EQ(4, first_bit64(0x10ULL), "first_bit(0x10) = 4");
    TEST_ASSERT_EQ(0, first_bit64(0xFFFFFFFFFFFFFFFFULL), "first_bit(all) = 0");
    TEST_ASSERT_EQ(19, first_bit64(INIT_MOVES), "first_bit(INIT_MOVES) = 19");
}

/* ================================================================
 * Edge wrapping tests - critical for bitboard correctness
 * ================================================================ */

void test_get_moves_no_horizontal_wrap(void) {
    Board board;
    /* Player at H4, opponent at A4 - should NOT be able to flip */
    board.player = 1ULL << 31;   /* H4 */
    board.opponent = 1ULL << 24; /* A4 */

    uint64_t moves = get_moves(board.player, board.opponent);
    /* Should have no moves - H4 cannot reach A4 by wrapping */
    TEST_ASSERT_EQ(0, popcount64(moves & 0xFF000000ULL), "no horizontal wrap in row 4");
}

void test_get_moves_edge_mask(void) {
    Board board;
    /* Player at A4, opponent at B4, C4, D4, E4, F4, G4 */
    board.player = 1ULL << 24;   /* A4 */
    board.opponent = 0x7E000000ULL; /* B4-G4 */

    uint64_t moves = get_moves(board.player, board.opponent);
    /* Should be able to place at H4 (31) */
    TEST_ASSERT((moves & (1ULL << 31)) != 0, "can flip entire row");
}

/* ================================================================
 * Multi-direction flip tests
 * ================================================================ */

void test_flip_multiple_directions(void) {
    Board board;
    /* Setup: player at corners, opponent in middle lines
     * Player can place in center and flip in multiple directions
     */
    board.player = (1ULL << 0) | (1ULL << 7) | (1ULL << 56) | (1ULL << 63); /* corners */
    board.opponent = (1ULL << 9) | (1ULL << 14) | (1ULL << 49) | (1ULL << 54) |
                     (1ULL << 18) | (1ULL << 21) | (1ULL << 42) | (1ULL << 45) |
                     (1ULL << 27) | (1ULL << 28) | (1ULL << 35) | (1ULL << 36); /* surrounding */

    /* Center at 27,28,35,36 - let's test from position with clear diagonal */
    /* Simpler test: player at E1, opponent at E2-E7, can place E8 */
    board.player = 1ULL << 4;  /* E1 */
    board.opponent = (1ULL << 12) | (1ULL << 20) | (1ULL << 28) |
                     (1ULL << 36) | (1ULL << 44) | (1ULL << 52); /* E2-E7 */

    uint64_t flipped = flip_disc(60, board.player, board.opponent); /* E8 */
    TEST_ASSERT_EQ(6, popcount64(flipped), "E8 flips 6 discs vertically");
}

void test_flip_long_diagonal(void) {
    Board board;
    /* Player at A1, opponent along diagonal A2-G7 */
    board.player = 1ULL << 0;  /* A1 */
    board.opponent = (1ULL << 9) | (1ULL << 18) | (1ULL << 27) |
                     (1ULL << 36) | (1ULL << 45) | (1ULL << 54); /* B2-G7 */

    uint64_t flipped = flip_disc(63, board.player, board.opponent); /* H8 */
    TEST_ASSERT_EQ(6, popcount64(flipped), "H8 flips 6 discs diagonally");
}

/* ================================================================
 * All 8 direction tests from center
 * ================================================================ */

void test_flip_all_8_directions(void) {
    Board board;
    uint64_t moves;

    /* Test that get_moves correctly identifies all 8 directions */
    /* Player at D4 (27) center, opponent surrounding in all 8 directions */
    board.player = (1ULL << 27);  /* D4 center */
    board.opponent = (1ULL << 18) | (1ULL << 19) | (1ULL << 20) |  /* C3, D3, E3 */
                     (1ULL << 26) | (1ULL << 28) |                  /* C4, E4 */
                     (1ULL << 34) | (1ULL << 35) | (1ULL << 36);    /* C5, D5, E5 */

    moves = get_moves(board.player, board.opponent);

    /* Player should be able to move to all 8 positions beyond the opponent ring */
    TEST_ASSERT((moves & (1ULL << 9)) != 0, "B2 move (NW diagonal)");
    TEST_ASSERT((moves & (1ULL << 11)) != 0, "D2 move (N vertical)");
    TEST_ASSERT((moves & (1ULL << 13)) != 0, "F2 move (NE diagonal)");
    TEST_ASSERT((moves & (1ULL << 25)) != 0, "B4 move (W horizontal)");
    TEST_ASSERT((moves & (1ULL << 29)) != 0, "F4 move (E horizontal)");
    TEST_ASSERT((moves & (1ULL << 41)) != 0, "B6 move (SW diagonal)");
    TEST_ASSERT((moves & (1ULL << 43)) != 0, "D6 move (S vertical)");
    TEST_ASSERT((moves & (1ULL << 45)) != 0, "F6 move (SE diagonal)");
}

/* ================================================================
 * Pass and game over detection
 * ================================================================ */

int board_is_pass(const Board *board) {
    return get_moves(board->player, board->opponent) == 0;
}

int board_is_game_over(const Board *board) {
    if (get_moves(board->player, board->opponent) != 0) return 0;
    if (get_moves(board->opponent, board->player) != 0) return 0;
    return 1;
}

void test_board_is_pass(void) {
    Board board;
    board_init(&board);
    TEST_ASSERT(!board_is_pass(&board), "initial position is not pass");

    /* Position where player must pass: player and opponent are isolated
     * Player at A1 (corner), opponent at H8 (opposite corner) - no flips possible */
    board.player = 1ULL;           /* A1 only */
    board.opponent = 1ULL << 63;   /* H8 only */
    TEST_ASSERT(board_is_pass(&board), "player must pass with isolated corners");
}

void test_board_is_game_over(void) {
    Board board;
    board_init(&board);
    TEST_ASSERT(!board_is_game_over(&board), "initial position is not game over");

    /* Full board */
    board.player = 0x00000000FFFFFFFFULL;
    board.opponent = 0xFFFFFFFF00000000ULL;
    TEST_ASSERT(board_is_game_over(&board), "full board is game over");

    /* Empty board (both sides pass) */
    board.player = 0x0000000000000001ULL;
    board.opponent = 0x8000000000000000ULL;
    TEST_ASSERT(board_is_game_over(&board), "isolated corners is game over");
}

/* ================================================================
 * Score counting
 * ================================================================ */

int board_score(const Board *board) {
    return popcount64(board->player) - popcount64(board->opponent);
}

void test_board_score(void) {
    Board board;
    board_init(&board);
    TEST_ASSERT_EQ(0, board_score(&board), "initial score is 0");

    board.player = 0xFFFFFFFFFFFFFFFFULL;
    board.opponent = 0;
    TEST_ASSERT_EQ(64, board_score(&board), "all player = +64");

    board.player = 0;
    board.opponent = 0xFFFFFFFFFFFFFFFFULL;
    TEST_ASSERT_EQ(-64, board_score(&board), "all opponent = -64");

    board.player = 0x00000000FFFFFFFFULL;  /* 32 discs */
    board.opponent = 0xFFFFFFFF00000000ULL; /* 32 discs */
    TEST_ASSERT_EQ(0, board_score(&board), "equal = 0");
}

/* ================================================================
 * Mobility (move count) tests
 * ================================================================ */

void test_mobility(void) {
    Board board;
    board_init(&board);

    int mobility = popcount64(get_moves(board.player, board.opponent));
    TEST_ASSERT_EQ(4, mobility, "initial mobility = 4");

    /* After D3 */
    uint64_t f = flip_disc(19, board.player, board.opponent);
    Board next;
    next.player = board.opponent ^ f;
    next.opponent = (board.player | (1ULL << 19)) ^ f;

    int opp_mobility = popcount64(get_moves(next.player, next.opponent));
    TEST_ASSERT_EQ(3, opp_mobility, "after D3, opponent has 3 moves");
}

/* ================================================================
 * Symmetry tests
 * ================================================================ */

uint64_t horizontal_mirror(uint64_t b) {
    b = ((b >> 1) & 0x5555555555555555ULL) | ((b << 1) & 0xAAAAAAAAAAAAAAAAULL);
    b = ((b >> 2) & 0x3333333333333333ULL) | ((b << 2) & 0xCCCCCCCCCCCCCCCCULL);
    b = ((b >> 4) & 0x0F0F0F0F0F0F0F0FULL) | ((b << 4) & 0xF0F0F0F0F0F0F0F0ULL);
    return b;
}

uint64_t vertical_mirror(uint64_t b) {
    b = ((b >>  8) & 0x00FF00FF00FF00FFULL) | ((b <<  8) & 0xFF00FF00FF00FF00ULL);
    b = ((b >> 16) & 0x0000FFFF0000FFFFULL) | ((b << 16) & 0xFFFF0000FFFF0000ULL);
    b = ((b >> 32) & 0x00000000FFFFFFFFULL) | ((b << 32) & 0xFFFFFFFF00000000ULL);
    return b;
}

void test_symmetry(void) {
    Board board;
    board_init(&board);

    /* Initial position: H-mirror of player equals opponent */
    TEST_ASSERT_EQ(board.opponent, horizontal_mirror(board.player), "H-mirror player = opponent");

    /* Test mirror operations on known values */
    uint64_t test = 0x0102040810204080ULL; /* diagonal A8-H1 */
    TEST_ASSERT_EQ(0x8040201008040201ULL, horizontal_mirror(test), "H-mirror of diagonal");

    /* Single corner tests */
    TEST_ASSERT_EQ(1ULL << 7, horizontal_mirror(1ULL << 0), "H-mirror A1 = H1");
    TEST_ASSERT_EQ(1ULL << 56, vertical_mirror(1ULL << 0), "V-mirror A1 = A8");
    TEST_ASSERT_EQ(1ULL << 63, vertical_mirror(1ULL << 7), "V-mirror H1 = H8");
}

/* ================================================================
 * Board transpose (diagonal reflection)
 * ================================================================ */

uint64_t transpose(uint64_t b) {
    uint64_t t;
    t = (b ^ (b >> 7)) & 0x00AA00AA00AA00AAULL;
    b = b ^ t ^ (t << 7);
    t = (b ^ (b >> 14)) & 0x0000CCCC0000CCCCULL;
    b = b ^ t ^ (t << 14);
    t = (b ^ (b >> 28)) & 0x00000000F0F0F0F0ULL;
    b = b ^ t ^ (t << 28);
    return b;
}

void test_transpose(void) {
    /* Transpose swaps rows and columns */
    /* A1 (0) stays at A1 (0) */
    TEST_ASSERT_EQ(1ULL, transpose(1ULL), "transpose A1 = A1");

    /* H1 (7) swaps to A8 (56) */
    TEST_ASSERT_EQ(1ULL << 56, transpose(1ULL << 7), "transpose H1 = A8");

    /* A8 (56) swaps to H1 (7) */
    TEST_ASSERT_EQ(1ULL << 7, transpose(1ULL << 56), "transpose A8 = H1");

    /* H8 (63) stays at H8 (63) */
    TEST_ASSERT_EQ(1ULL << 63, transpose(1ULL << 63), "transpose H8 = H8");

    /* Double transpose is identity */
    uint64_t test = INIT_PLAYER;
    TEST_ASSERT_EQ(test, transpose(transpose(test)), "double transpose = identity");
}

/* ================================================================
 * Move sequence test (play game, verify result)
 * ================================================================ */

void board_do_move(Board *board, int sq) {
    uint64_t flipped = flip_disc(sq, board->player, board->opponent);
    uint64_t new_player = board->opponent ^ flipped;
    uint64_t new_opponent = (board->player | (1ULL << sq)) ^ flipped;
    board->player = new_player;
    board->opponent = new_opponent;
}

void test_move_sequence(void) {
    Board board;
    board_init(&board);

    /* Play a known opening sequence: D3, C3, C4, C5 */
    int moves[] = {19, 18, 26, 34};
    int i;
    for (i = 0; i < 4; i++) {
        uint64_t legal = get_moves(board.player, board.opponent);
        TEST_ASSERT((legal & (1ULL << moves[i])) != 0, "move is legal");
        board_do_move(&board, moves[i]);
    }

    /* Verify disc counts after 4 moves: started with 4, added 4 */
    int total = popcount64(board.player) + popcount64(board.opponent);
    TEST_ASSERT_EQ(8, total, "8 discs after 4 moves");
}

/* ================================================================
 * Unique board representation tests
 * ================================================================ */

void board_unique(const Board *board, Board *unique) {
    /* Return the lexicographically smallest of all 8 symmetries */
    Board sym, best;
    best = *board;

    /* Try all 8 symmetries: identity, H, V, HV, T, TH, TV, THV */
    sym.player = horizontal_mirror(board->player);
    sym.opponent = horizontal_mirror(board->opponent);
    if (sym.player < best.player || (sym.player == best.player && sym.opponent < best.opponent))
        best = sym;

    sym.player = vertical_mirror(board->player);
    sym.opponent = vertical_mirror(board->opponent);
    if (sym.player < best.player || (sym.player == best.player && sym.opponent < best.opponent))
        best = sym;

    sym.player = vertical_mirror(horizontal_mirror(board->player));
    sym.opponent = vertical_mirror(horizontal_mirror(board->opponent));
    if (sym.player < best.player || (sym.player == best.player && sym.opponent < best.opponent))
        best = sym;

    sym.player = transpose(board->player);
    sym.opponent = transpose(board->opponent);
    if (sym.player < best.player || (sym.player == best.player && sym.opponent < best.opponent))
        best = sym;

    sym.player = horizontal_mirror(transpose(board->player));
    sym.opponent = horizontal_mirror(transpose(board->opponent));
    if (sym.player < best.player || (sym.player == best.player && sym.opponent < best.opponent))
        best = sym;

    sym.player = vertical_mirror(transpose(board->player));
    sym.opponent = vertical_mirror(transpose(board->opponent));
    if (sym.player < best.player || (sym.player == best.player && sym.opponent < best.opponent))
        best = sym;

    sym.player = vertical_mirror(horizontal_mirror(transpose(board->player)));
    sym.opponent = vertical_mirror(horizontal_mirror(transpose(board->opponent)));
    if (sym.player < best.player || (sym.player == best.player && sym.opponent < best.opponent))
        best = sym;

    *unique = best;
}

void test_board_unique(void) {
    Board board, unique;
    board_init(&board);

    board_unique(&board, &unique);

    /* Initial position should be its own unique form (it's highly symmetric) */
    /* All symmetries of initial position should give same unique form */
    Board h_board, h_unique;
    h_board.player = horizontal_mirror(board.player);
    h_board.opponent = horizontal_mirror(board.opponent);
    board_unique(&h_board, &h_unique);

    TEST_ASSERT(unique.player == h_unique.player && unique.opponent == h_unique.opponent,
                "H-mirror has same unique form");
}

/* ================================================================
 * Hash function tests
 * ================================================================ */

uint64_t board_hash(const Board *board) {
    /* Simple hash combining player and opponent with mixing */
    uint64_t h = board->player;
    h ^= board->opponent * 0x9E3779B97F4A7C15ULL;
    h ^= h >> 33;
    h *= 0xFF51AFD7ED558CCDULL;
    h ^= h >> 33;
    h *= 0xC4CEB9FE1A85EC53ULL;
    h ^= h >> 33;
    return h;
}

void test_board_hash(void) {
    Board b1, b2;
    board_init(&b1);
    board_init(&b2);

    /* Same boards should have same hash */
    TEST_ASSERT_EQ(board_hash(&b1), board_hash(&b2), "same boards same hash");

    /* Different boards should (likely) have different hash */
    b2.player ^= 1;
    TEST_ASSERT(board_hash(&b1) != board_hash(&b2), "different boards different hash");
}

/* Perft test */
static const uint64_t PERFT_EXPECTED[] = {
    1ULL, 4ULL, 12ULL, 56ULL, 244ULL, 1396ULL, 8200ULL, 55092ULL, 390216ULL
};

uint64_t perft(const Board *board, int depth) {
    if (depth == 0) return 1;

    uint64_t moves = get_moves(board->player, board->opponent);
    uint64_t nodes = 0;

    if (moves == 0) {
        Board passed = *board;
        board_pass(&passed);
        if (get_moves(passed.player, passed.opponent) == 0) {
            return 1;
        }
        return perft(&passed, depth - 1);
    }

    while (moves) {
        int sq = first_bit64(moves);
        moves &= moves - 1;

        Board child = *board;
        uint64_t f = flip_disc(sq, board->player, board->opponent);
        child.player = board->opponent ^ f;
        child.opponent = (board->player | (1ULL << sq)) ^ f;

        nodes += perft(&child, depth - 1);
    }

    return nodes;
}

void test_perft(void) {
    Board board;
    board_init(&board);

    int depth;
    for (depth = 0; depth <= 6; depth++) {
        uint64_t nodes = perft(&board, depth);
        char msg[64];
        sprintf(msg, "perft(%d)", depth);
        TEST_ASSERT_EQ(PERFT_EXPECTED[depth], nodes, msg);
    }
}

/* ================================================================
 * Main
 * ================================================================ */

int main(void) {
    printf("=== Edax C Minimal Tests ===\n\n");

    /* Basic board tests */
    RUN_TEST(test_board_init);
    RUN_TEST(test_board_pass);
    RUN_TEST(test_board_count_empties);
    RUN_TEST(test_board_get_square_color);

    /* Bit operation tests */
    RUN_TEST(test_bit_count);
    RUN_TEST(test_first_bit);

    /* Move generation tests */
    RUN_TEST(test_get_moves_initial);
    RUN_TEST(test_get_moves_no_moves);
    RUN_TEST(test_get_moves_no_horizontal_wrap);
    RUN_TEST(test_get_moves_edge_mask);

    /* Flip tests */
    RUN_TEST(test_flip_d3);
    RUN_TEST(test_flip_c4);
    RUN_TEST(test_flip_f5);
    RUN_TEST(test_flip_e6);
    RUN_TEST(test_flip_no_flip);
    RUN_TEST(test_flip_multiple_directions);
    RUN_TEST(test_flip_long_diagonal);
    RUN_TEST(test_flip_all_8_directions);

    /* Game state tests */
    RUN_TEST(test_board_is_pass);
    RUN_TEST(test_board_is_game_over);
    RUN_TEST(test_board_score);
    RUN_TEST(test_mobility);

    /* Symmetry tests */
    RUN_TEST(test_symmetry);
    RUN_TEST(test_transpose);

    /* Unique board tests */
    RUN_TEST(test_board_unique);

    /* Hash tests */
    RUN_TEST(test_board_hash);

    /* Move sequence tests */
    RUN_TEST(test_move_sequence);

    /* Perft (move generation verification) */
    RUN_TEST(test_perft);

    TEST_SUMMARY();

    return tests_failed > 0 ? 1 : 0;
}
