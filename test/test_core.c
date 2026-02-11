/**
 * @file test_core.c
 * @brief Standalone core tests for edax - minimal dependencies
 */

/* Include just the essentials */
#include "util.c"
#include "bit.c"
#include "hash.c"
#include "move.c"
#include "board.c"

/* Test framework */
#include "../test/test_framework.h"

/* Test data */
#define INIT_PLAYER   0x0000000810000000ULL
#define INIT_OPPONENT 0x0000001008000000ULL
#define INIT_MOVES    0x0000102004080000ULL

/* ================================================================
 * board_init tests
 * ================================================================ */

void test_board_init(void) {
    Board board;
    board_init(&board);

    TEST_ASSERT_EQ_HEX(INIT_PLAYER, board.player, "initial player bitboard");
    TEST_ASSERT_EQ_HEX(INIT_OPPONENT, board.opponent, "initial opponent bitboard");
    TEST_ASSERT_EQ(2, bit_count(board.player), "player disc count");
    TEST_ASSERT_EQ(2, bit_count(board.opponent), "opponent disc count");
    TEST_ASSERT_EQ(60, board_count_empties(&board), "empty count");
}

/* ================================================================
 * get_moves tests
 * ================================================================ */

void test_get_moves_initial(void) {
    Board board;
    board_init(&board);

    unsigned long long moves = get_moves(board.player, board.opponent);

    TEST_ASSERT_EQ(4, bit_count(moves), "initial position has 4 moves");
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

    unsigned long long moves = get_moves(board.player, board.opponent);
    TEST_ASSERT_EQ(0, moves, "no opponent means no moves");
}

/* ================================================================
 * flip tests
 * ================================================================ */

void test_flip_d3(void) {
    Board board;
    board_init(&board);

    /* D3 (19) flips D4 (27) in the +8 (down) direction */
    unsigned long long flipped = flip[19](board.player, board.opponent);
    TEST_ASSERT_EQ_HEX(1ULL << 27, flipped, "D3 flips D4");
    TEST_ASSERT_EQ(1, bit_count(flipped), "D3 flips 1 disc");
}

void test_flip_c4(void) {
    Board board;
    board_init(&board);

    unsigned long long flipped = flip[26](board.player, board.opponent);
    TEST_ASSERT_EQ_HEX(1ULL << 27, flipped, "C4 flips D4");
    TEST_ASSERT_EQ(1, bit_count(flipped), "C4 flips 1 disc");
}

void test_flip_f5(void) {
    Board board;
    board_init(&board);

    unsigned long long flipped = flip[37](board.player, board.opponent);
    TEST_ASSERT_EQ_HEX(1ULL << 36, flipped, "F5 flips E5");
    TEST_ASSERT_EQ(1, bit_count(flipped), "F5 flips 1 disc");
}

void test_flip_e6(void) {
    Board board;
    board_init(&board);

    unsigned long long flipped = flip[44](board.player, board.opponent);
    TEST_ASSERT_EQ_HEX(1ULL << 36, flipped, "E6 flips E5");
    TEST_ASSERT_EQ(1, bit_count(flipped), "E6 flips 1 disc");
}

void test_flip_no_flip(void) {
    Board board;
    board_init(&board);

    unsigned long long flipped = flip[0](board.player, board.opponent);
    TEST_ASSERT_EQ(0, flipped, "A1 is illegal, no flips");
}

/* ================================================================
 * board_pass tests
 * ================================================================ */

void test_board_pass(void) {
    Board board;
    board_init(&board);

    unsigned long long orig_player = board.player;
    unsigned long long orig_opponent = board.opponent;

    board_pass(&board);

    TEST_ASSERT_EQ_HEX(orig_opponent, board.player, "pass swaps player");
    TEST_ASSERT_EQ_HEX(orig_player, board.opponent, "pass swaps opponent");
}

/* ================================================================
 * board_is_game_over tests
 * ================================================================ */

void test_board_is_game_over_initial(void) {
    Board board;
    board_init(&board);

    TEST_ASSERT(!board_is_game_over(&board), "initial position is not game over");
}

void test_board_is_game_over_full(void) {
    Board board;
    board.player = 0x00000000FFFFFFFFULL;
    board.opponent = 0xFFFFFFFF00000000ULL;

    TEST_ASSERT(board_is_game_over(&board), "full board is game over");
}

/* ================================================================
 * board_count_empties tests
 * ================================================================ */

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

/* ================================================================
 * board_get_square_color tests
 * ================================================================ */

void test_board_get_square_color(void) {
    Board board;
    board_init(&board);

    TEST_ASSERT_EQ(0, board_get_square_color(&board, 28), "E4 is player (0)");
    TEST_ASSERT_EQ(0, board_get_square_color(&board, 35), "D5 is player (0)");
    TEST_ASSERT_EQ(1, board_get_square_color(&board, 27), "D4 is opponent (1)");
    TEST_ASSERT_EQ(1, board_get_square_color(&board, 36), "E5 is opponent (1)");
    TEST_ASSERT_EQ(2, board_get_square_color(&board, 0), "A1 is empty (2)");
}

/* ================================================================
 * board_equal tests
 * ================================================================ */

void test_board_equal(void) {
    Board b1, b2;
    board_init(&b1);
    board_init(&b2);

    TEST_ASSERT(board_equal(&b1, &b2), "identical boards are equal");

    b2.player ^= 1;
    TEST_ASSERT(!board_equal(&b1, &b2), "different boards are not equal");
}

/* ================================================================
 * board_symetry tests
 * ================================================================ */

void test_board_symetry(void) {
    Board board, sym;
    board_init(&board);

    board_symetry(&board, 0, &sym);
    TEST_ASSERT(board_equal(&board, &sym), "symmetry 0 is identity");

    int s;
    for (s = 0; s < 8; s++) {
        board_symetry(&board, s, &sym);
        TEST_ASSERT_EQ(bit_count(board.player), bit_count(sym.player), "symmetry preserves player count");
        TEST_ASSERT_EQ(bit_count(board.opponent), bit_count(sym.opponent), "symmetry preserves opponent count");
    }
}

/* ================================================================
 * count_last_flip tests
 * ================================================================ */

void test_count_last_flip(void) {
    Board board;
    board.player = 0x00000000FFFFFFFEULL;
    board.opponent = 0xFFFFFFFF00000000ULL;

    int n = count_last_flip(0, board.player);
    TEST_ASSERT(n >= 0, "count_last_flip returns non-negative");
}

/* ================================================================
 * get_stability tests
 * ================================================================ */

void test_get_stability(void) {
    Board board;
    board.player = 1ULL;
    board.opponent = 0xFEULL;

    int stability = get_stability(board.player, board.opponent);
    TEST_ASSERT(stability >= 1, "corner disc should be stable");
}

void test_get_stability_full_edge(void) {
    Board board;
    board.player = 0xFFULL;
    board.opponent = 0xFF00ULL;

    int stability = get_stability(board.player, board.opponent);
    TEST_ASSERT_EQ(8, stability, "full edge should have 8 stable discs");
}

/* ================================================================
 * bit operations tests
 * ================================================================ */

void test_bit_count(void) {
    TEST_ASSERT_EQ(0, bit_count(0), "bit_count(0) = 0");
    TEST_ASSERT_EQ(1, bit_count(1), "bit_count(1) = 1");
    TEST_ASSERT_EQ(64, bit_count(0xFFFFFFFFFFFFFFFFULL), "bit_count(all 1s) = 64");
    TEST_ASSERT_EQ(32, bit_count(0xAAAAAAAAAAAAAAAAULL), "bit_count(alternating) = 32");
    TEST_ASSERT_EQ(4, bit_count(INIT_PLAYER | INIT_OPPONENT), "initial discs = 4");
}

void test_first_bit(void) {
    TEST_ASSERT_EQ(0, first_bit(1ULL), "first_bit(1) = 0");
    TEST_ASSERT_EQ(63, first_bit(0x8000000000000000ULL), "first_bit(high) = 63");
    TEST_ASSERT_EQ(4, first_bit(0x10ULL), "first_bit(0x10) = 4");
}

void test_last_bit(void) {
    TEST_ASSERT_EQ(0, last_bit(1ULL), "last_bit(1) = 0");
    TEST_ASSERT_EQ(63, last_bit(0x8000000000000000ULL), "last_bit(high) = 63");
    TEST_ASSERT_EQ(63, last_bit(0xFFFFFFFFFFFFFFFFULL), "last_bit(all) = 63");
}

/* ================================================================
 * perft test (move generation verification)
 * ================================================================ */

static const unsigned long long PERFT_EXPECTED[] = {
    1ULL,           /* depth 0 */
    4ULL,           /* depth 1 */
    12ULL,          /* depth 2 */
    56ULL,          /* depth 3 */
    244ULL,         /* depth 4 */
    1396ULL,        /* depth 5 */
    8200ULL,        /* depth 6 */
    55092ULL,       /* depth 7 */
    390216ULL,      /* depth 8 */
};

unsigned long long perft(const Board *board, int depth) {
    if (depth == 0) return 1;

    unsigned long long moves = get_moves(board->player, board->opponent);
    unsigned long long nodes = 0;

    if (moves == 0) {
        Board passed = *board;
        board_pass(&passed);
        if (get_moves(passed.player, passed.opponent) == 0) {
            return 1;
        }
        return perft(&passed, depth - 1);
    }

    while (moves) {
        int sq = first_bit(moves);
        moves &= moves - 1;

        Board child = *board;
        unsigned long long f = flip[sq](board->player, board->opponent);
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
        unsigned long long nodes = perft(&board, depth);
        char msg[64];
        sprintf(msg, "perft(%d)", depth);
        TEST_ASSERT_EQ(PERFT_EXPECTED[depth], nodes, msg);
    }
}

/* ================================================================
 * Main
 * ================================================================ */

int main(void) {
    printf("=== Edax C Core Tests ===\n\n");

    edge_stability_init();

    RUN_TEST(test_board_init);
    RUN_TEST(test_get_moves_initial);
    RUN_TEST(test_get_moves_no_moves);
    RUN_TEST(test_flip_d3);
    RUN_TEST(test_flip_c4);
    RUN_TEST(test_flip_f5);
    RUN_TEST(test_flip_e6);
    RUN_TEST(test_flip_no_flip);
    RUN_TEST(test_board_pass);
    RUN_TEST(test_board_is_game_over_initial);
    RUN_TEST(test_board_is_game_over_full);
    RUN_TEST(test_board_count_empties);
    RUN_TEST(test_board_get_square_color);
    RUN_TEST(test_board_equal);
    RUN_TEST(test_board_symetry);
    RUN_TEST(test_count_last_flip);
    RUN_TEST(test_get_stability);
    RUN_TEST(test_get_stability_full_edge);
    RUN_TEST(test_bit_count);
    RUN_TEST(test_first_bit);
    RUN_TEST(test_last_bit);
    RUN_TEST(test_perft);

    TEST_SUMMARY();

    return tests_failed > 0 ? 1 : 0;
}
