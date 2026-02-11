package edax

/** Bit manipulation utilities for bitboards. */

/** Transpose a bitboard along the A1-H8 diagonal. */
fun transpose(b: ULong): ULong {
    var x = b
    var t: ULong

    t = (x xor (x shr 7)) and 0x00AA00AA00AA00AAuL
    x = x xor t xor (t shl 7)
    t = (x xor (x shr 14)) and 0x0000CCCC0000CCCCuL
    x = x xor t xor (t shl 14)
    t = (x xor (x shr 28)) and 0x00000000F0F0F0F0uL
    x = x xor t xor (t shl 28)

    return x
}

/** Mirror horizontally (swap columns A<->H, B<->G, etc.). */
fun horizontalMirror(b: ULong): ULong {
    var x = b
    x = ((x shr 1) and 0x5555555555555555uL) or ((x shl 1) and 0xAAAAAAAAAAAAAAAAuL)
    x = ((x shr 2) and 0x3333333333333333uL) or ((x shl 2) and 0xCCCCCCCCCCCCCCCCuL)
    x = ((x shr 4) and 0x0F0F0F0F0F0F0F0FuL) or ((x shl 4) and 0xF0F0F0F0F0F0F0F0uL)
    return x
}

/** Mirror vertically (swap rows 1<->8, 2<->7, etc.). */
fun verticalMirror(b: ULong): ULong {
    var x = b
    x = ((x shr 8) and 0x00FF00FF00FF00FFuL) or ((x shl 8) and 0xFF00FF00FF00FF00uL)
    x = ((x shr 16) and 0x0000FFFF0000FFFFuL) or ((x shl 16) and 0xFFFF0000FFFF0000uL)
    x = ((x shr 32) and 0x00000000FFFFFFFFuL) or ((x shl 32) and 0xFFFFFFFF00000000uL)
    return x
}

/** Get the index of the first (least significant) set bit. */
fun firstBit(b: ULong): Int = b.countTrailingZeroBits()

/** Get the index of the last (most significant) set bit. */
fun lastBit(b: ULong): Int = 63 - b.countLeadingZeroBits()
