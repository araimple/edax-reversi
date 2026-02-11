# Test Makefile for edax
CC = gcc
CFLAGS = -std=c99 -pedantic -W -Wall -Wextra -pipe -D_GNU_SOURCE=1 -O2 \
         -m64 -DUSE_GAS_X64 -DPOPCOUNT -DHAS_CPU_64 -I../src -I../include
LDFLAGS = -lm -lpthread

SRC_DIR = ../src

.PHONY: all clean test run_tests

all: test_board

# Compile test_board with all needed source files directly (unity build style)
test_board: test_board.c
	$(CC) $(CFLAGS) -include $(SRC_DIR)/bit.c -include $(SRC_DIR)/util.c \
	    -DALL_C_UNITY \
	    -o $@ test_board.c $(SRC_DIR)/board.c $(SRC_DIR)/move.c $(SRC_DIR)/hash.c \
	    $(SRC_DIR)/options.c $(SRC_DIR)/stats.c $(SRC_DIR)/base.c $(LDFLAGS)

run_tests: test_board
	@echo "Running test_board..."
	./test_board

clean:
	rm -f test_board test_bit test_eval *.o
