/**
 * @file test_board.c
 * @brief Comprehensive tests for board module
 */

#include "test_framework.h"
#include "../src/board.h"
#include "../src/move.h"
#include "../src/bit.h"

/* Test data: known positions and expected results */

/* Initial position */
#define INIT_PLAYER   0x0000000810000000ULL
#define INIT_OPPONENT 0x0000001008000000ULL

/* Initial legal moves for black: D3, C4, F5, E6 */
#define INIT_MOVES    0x0000102004080000ULL

/* ================================================================
 * board_init tests
 * ================================================================ */

void test_board_init(void) {
    Board board;
    board_init(&board);

    TEST_ASSERT_EQ_HEX(INIT_PLAYER, board.player, "initial player bitboard");
    TEST_ASSERT_EQ_HEX(INIT_OPPONENT, board.opponent, "initial opponent bitboard");

    /* Verify disc counts */
    TEST_ASSERT_EQ(2, bit_count(board.player), "player disc count");
    TEST_ASSERT_EQ(2, bit_count(board.opponent), "opponent disc count");
    TEST_ASSERT_EQ(60, board_count_empties(&board), "empty count");
}

/* ================================================================
 * board_set tests
 * ================================================================ */

void test_board_set(void) {
    Board board;

    /* Standard initial position string */
    int result = board_set(&board, "---------------------------OX------XO--------------------------- X");
    TEST_ASSERT(result == 0 || result == 64, "board_set should succeed");
    TEST_ASSERT_EQ_HEX(INIT_PLAYER, board.player, "board_set player");
    TEST_ASSERT_EQ_HEX(INIT_OPPONENT, board.opponent, "board_set opponent");

    /* All black */
    Board board2;
    board_set(&board2, "XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX X");
    TEST_ASSERT_EQ(64, bit_count(board2.player), "all black: player count");
    TEST_ASSERT_EQ(0, bit_count(board2.opponent), "all black: opponent count");

    /* All white */
    Board board3;
    board_set(&board3, "OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOO X");
    TEST_ASSERT_EQ(0, bit_count(board3.player), "all white: player count");
    TEST_ASSERT_EQ(64, bit_count(board3.opponent), "all white: opponent count");

    /* Empty board */
    Board board4;
    board_set(&board4, "---------------------------------------------------------------- X");
    TEST_ASSERT_EQ(0, bit_count(board4.player), "empty: player count");
    TEST_ASSERT_EQ(0, bit_count(board4.opponent), "empty: opponent count");
}

/* ================================================================
 * get_moves tests
 * ================================================================ */

void test_get_moves_initial(void) {
    Board board;
    board_init(&board);

    unsigned long long moves = get_moves(board.player, board.opponent);

    /* Initial position: D3(19), C4(26), F5(37), E6(44) */
    TEST_ASSERT_EQ(4, bit_count(moves), "initial position has 4 moves");
    TEST_ASSERT_EQ_HEX(INIT_MOVES, moves, "initial legal moves");

    /* Verify individual squares */
    TEST_ASSERT((moves & (1ULL << 19)) != 0, "D3 is legal");
    TEST_ASSERT((moves & (1ULL << 26)) != 0, "C4 is legal");
    TEST_ASSERT((moves & (1ULL << 37)) != 0, "F5 is legal");
    TEST_ASSERT((moves & (1ULL << 44)) != 0, "E6 is legal");
}

void test_get_moves_no_moves(void) {
    /* Board where player has no moves */
    Board board;
    board.player = 0x0000000000000001ULL;   /* single disc in corner A1 */
    board.opponent = 0x0000000000000000ULL; /* no opponent */

    unsigned long long moves = get_moves(board.player, board.opponent);
    TEST_ASSERT_EQ(0, moves, "no opponent means no moves");
}

void test_get_moves_pass_position(void) {
    /* Position where current player must pass */
    Board board;
    board.player = 0x00000000000000FFULL;   /* bottom row */
    board.opponent = 0x000000000000FF00ULL; /* second row */

    unsigned long long moves = get_moves(board.player, board.opponent);
    /* Player can't flip any opponent discs */
    TEST_ASSERT_EQ(0, bit_count(moves), "player must pass");

    /* But opponent can move */
    unsigned long long opp_moves = get_moves(board.opponent, board.player);
    TEST_ASSERT(bit_count(opp_moves) > 0, "opponent has moves");
}

void test_get_moves_all_directions(void) {
    /* Test flipping in all 8 directions from center */
    Board board;
    /* Player at E5 (36), opponent surrounding */
    board.player = 1ULL << 36;  /* E5 */
    board.opponent = (1ULL << 27) | (1ULL << 28) | (1ULL << 29) |  /* D4, E4, F4 */
                     (1ULL << 35) | (1ULL << 37) |                  /* D5, F5 */
                     (1ULL << 43) | (1ULL << 44) | (1ULL << 45);    /* D6, E6, F6 */

    unsigned long long moves = get_moves(board.player, board.opponent);

    /* Player should be able to place in 8 directions */
    TEST_ASSERT((moves & (1ULL << 18)) != 0, "can place at C3 (NW)");
    TEST_ASSERT((moves & (1ULL << 20)) != 0, "can place at E3 (N)");
    TEST_ASSERT((moves & (1ULL << 22)) != 0, "can place at G3 (NE)");
    TEST_ASSERT((moves & (1ULL << 34)) != 0, "can place at C5 (W)");
    TEST_ASSERT((moves & (1ULL << 38)) != 0, "can place at G5 (E)");
    TEST_ASSERT((moves & (1ULL << 50)) != 0, "can place at C7 (SW)");
    TEST_ASSERT((moves & (1ULL << 52)) != 0, "can place at E7 (S)");
    TEST_ASSERT((moves & (1ULL << 54)) != 0, "can place at G7 (SE)");
}

/* ================================================================
 * flip tests
 * ================================================================ */

void test_flip_d3(void) {
    Board board;
    board_init(&board);

    /* Play D3 (square 19) - flips D4 (27) in the +8 (down) direction */
    unsigned long long flipped = flip[19](board.player, board.opponent);

    /* Should flip D4 (square 27) */
    TEST_ASSERT_EQ_HEX(1ULL << 27, flipped, "D3 flips D4");
    TEST_ASSERT_EQ(1, bit_count(flipped), "D3 flips 1 disc");
}

void test_flip_c4(void) {
    Board board;
    board_init(&board);

    /* Play C4 (square 26) */
    unsigned long long flipped = flip[26](board.player, board.opponent);

    /* Should flip D4 (square 27) */
    TEST_ASSERT_EQ_HEX(1ULL << 27, flipped, "C4 flips D4");
    TEST_ASSERT_EQ(1, bit_count(flipped), "C4 flips 1 disc");
}

void test_flip_no_flip(void) {
    Board board;
    board_init(&board);

    /* Try illegal move A1 (square 0) */
    unsigned long long flipped = flip[0](board.player, board.opponent);
    TEST_ASSERT_EQ(0, flipped, "A1 is illegal, no flips");
}

void test_flip_multiple_directions(void) {
    Board board;
    /* Setup for multiple direction flip */
    board_set(&board, "---------------------------OX-----OXO-----OX------------------- X");

    /* E5 should flip in multiple directions */
    unsigned long long flipped = flip[36](board.player, board.opponent);
    TEST_ASSERT(bit_count(flipped) >= 2, "E5 should flip multiple discs");
}

/* ================================================================
 * board_update / board_restore tests
 * ================================================================ */

void test_board_update_restore(void) {
    Board board, original;
    Move move;

    board_init(&board);
    original = board;

    /* Make move D3 */
    board_get_move(&board, 19, &move);
    TEST_ASSERT(move.flipped != 0, "D3 should have flips");

    board_update(&board, &move);

    /* Verify board changed */
    TEST_ASSERT(board.player != original.player || board.opponent != original.opponent,
                "board should change after update");

    /* Restore */
    board_restore(&board, &move);

    /* Should be back to original */
    TEST_ASSERT_EQ_HEX(original.player, board.player, "player restored");
    TEST_ASSERT_EQ_HEX(original.opponent, board.opponent, "opponent restored");
}

void test_board_update_sequence(void) {
    Board board;
    Move moves[4];
    int squares[] = {19, 18, 10, 34};  /* D3, C3, C2, C5 */

    board_init(&board);

    /* Make 4 moves */
    for (int i = 0; i < 4; i++) {
        unsigned long long legal = get_moves(board.player, board.opponent);
        if (legal & (1ULL << squares[i])) {
            board_get_move(&board, squares[i], &moves[i]);
            board_update(&board, &moves[i]);
        }
    }

    /* Verify disc count increased */
    int total_discs = bit_count(board.player) + bit_count(board.opponent);
    TEST_ASSERT(total_discs >= 8, "should have at least 8 discs after 4 moves");

    /* Undo all moves */
    for (int i = 3; i >= 0; i--) {
        board_restore(&board, &moves[i]);
    }

    /* Should be back to initial */
    Board init;
    board_init(&init);
    TEST_ASSERT_EQ_HEX(init.player, board.player, "player after undo sequence");
    TEST_ASSERT_EQ_HEX(init.opponent, board.opponent, "opponent after undo sequence");
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

    /* Pass should swap player and opponent */
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
    board.player = 0x00000000FFFFFFFFULL;   /* 32 discs */
    board.opponent = 0xFFFFFFFF00000000ULL; /* 32 discs */

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

    /* E4 (28) and D5 (35) are player (BLACK) */
    TEST_ASSERT_EQ(0, board_get_square_color(&board, 28), "E4 is player (0)");
    TEST_ASSERT_EQ(0, board_get_square_color(&board, 35), "D5 is player (0)");

    /* D4 (27) and E5 (36) are opponent (WHITE) */
    TEST_ASSERT_EQ(1, board_get_square_color(&board, 27), "D4 is opponent (1)");
    TEST_ASSERT_EQ(1, board_get_square_color(&board, 36), "E5 is opponent (1)");

    /* A1 (0) is empty */
    TEST_ASSERT_EQ(2, board_get_square_color(&board, 0), "A1 is empty (2)");
}

/* ================================================================
 * board_equal / board_compare tests
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

    /* Symmetry 0 should be identity */
    board_symetry(&board, 0, &sym);
    TEST_ASSERT(board_equal(&board, &sym), "symmetry 0 is identity");

    /* Test all 8 symmetries preserve disc count */
    for (int s = 0; s < 8; s++) {
        board_symetry(&board, s, &sym);
        TEST_ASSERT_EQ(bit_count(board.player), bit_count(sym.player), "symmetry preserves player count");
        TEST_ASSERT_EQ(bit_count(board.opponent), bit_count(sym.opponent), "symmetry preserves opponent count");
    }
}

/* ================================================================
 * count_last_flip tests
 * ================================================================ */

void test_count_last_flip(void) {
    /* Create a 1-empty position */
    Board board;
    board.player = 0x00000000FFFFFFFEULL;   /* 31 discs, A1 empty */
    board.opponent = 0xFFFFFFFF00000000ULL; /* 32 discs */

    /* Count flips if player places at A1 */
    int n = count_last_flip(0, board.player);
    /* Result is 2x the number of flipped discs */
    TEST_ASSERT(n >= 0, "count_last_flip returns non-negative");
}

/* ================================================================
 * get_stability tests
 * ================================================================ */

void test_get_stability(void) {
    Board board;

    /* Board with corner disc - should be stable */
    board.player = 1ULL;  /* A1 */
    board.opponent = 0xFEULL;  /* rest of row 1 */

    int stability = get_stability(board.player, board.opponent);
    TEST_ASSERT(stability >= 1, "corner disc should be stable");
}

void test_get_stability_full_edge(void) {
    Board board;

    /* Player has full top edge */
    board.player = 0xFFULL;
    board.opponent = 0xFF00ULL;

    int stability = get_stability(board.player, board.opponent);
    TEST_ASSERT_EQ(8, stability, "full edge should have 8 stable discs");
}

/* ================================================================
 * Main
 * ================================================================ */

int main(void) {
    printf("=== Board Tests ===\n\n");

    /* Initialize required data */
    edge_stability_init();

    /* board_init */
    RUN_TEST(test_board_init);

    /* board_set */
    RUN_TEST(test_board_set);

    /* get_moves */
    RUN_TEST(test_get_moves_initial);
    RUN_TEST(test_get_moves_no_moves);
    RUN_TEST(test_get_moves_pass_position);
    RUN_TEST(test_get_moves_all_directions);

    /* flip */
    RUN_TEST(test_flip_d3);
    RUN_TEST(test_flip_c4);
    RUN_TEST(test_flip_no_flip);
    RUN_TEST(test_flip_multiple_directions);

    /* board_update / board_restore */
    RUN_TEST(test_board_update_restore);
    RUN_TEST(test_board_update_sequence);

    /* board_pass */
    RUN_TEST(test_board_pass);

    /* board_is_game_over */
    RUN_TEST(test_board_is_game_over_initial);
    RUN_TEST(test_board_is_game_over_full);

    /* board_count_empties */
    RUN_TEST(test_board_count_empties);

    /* board_get_square_color */
    RUN_TEST(test_board_get_square_color);

    /* board_equal */
    RUN_TEST(test_board_equal);

    /* board_symetry */
    RUN_TEST(test_board_symetry);

    /* count_last_flip */
    RUN_TEST(test_count_last_flip);

    /* get_stability */
    RUN_TEST(test_get_stability);
    RUN_TEST(test_get_stability_full_edge);

    TEST_SUMMARY();

    return tests_failed > 0 ? 1 : 0;
}
