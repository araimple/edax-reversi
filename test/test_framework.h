/**
 * @file test_framework.h
 * @brief Simple test framework for edax
 */

#ifndef TEST_FRAMEWORK_H
#define TEST_FRAMEWORK_H

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

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

#endif /* TEST_FRAMEWORK_H */
