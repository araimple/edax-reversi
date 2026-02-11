/// Bit manipulation utilities for bitboards.

/// Transpose a bitboard along the A1-H8 diagonal.
public func transpose(_ b: UInt64) -> UInt64 {
    var x = b
    var t: UInt64

    t = (x ^ (x >> 7)) & 0x00AA00AA00AA00AA
    x = x ^ t ^ (t << 7)
    t = (x ^ (x >> 14)) & 0x0000CCCC0000CCCC
    x = x ^ t ^ (t << 14)
    t = (x ^ (x >> 28)) & 0x00000000F0F0F0F0
    x = x ^ t ^ (t << 28)

    return x
}

/// Mirror horizontally (swap columns A<->H, B<->G, etc.).
public func horizontalMirror(_ b: UInt64) -> UInt64 {
    var x = b
    x = ((x >> 1) & 0x5555555555555555) | ((x << 1) & 0xAAAAAAAAAAAAAAAA)
    x = ((x >> 2) & 0x3333333333333333) | ((x << 2) & 0xCCCCCCCCCCCCCCCC)
    x = ((x >> 4) & 0x0F0F0F0F0F0F0F0F) | ((x << 4) & 0xF0F0F0F0F0F0F0F0)
    return x
}

/// Mirror vertically (swap rows 1<->8, 2<->7, etc.).
public func verticalMirror(_ b: UInt64) -> UInt64 {
    return b.byteSwapped
}

/// Get the index of the first (least significant) set bit.
public func firstBit(_ b: UInt64) -> Int {
    return b.trailingZeroBitCount
}

/// Get the index of the last (most significant) set bit.
public func lastBit(_ b: UInt64) -> Int {
    return 63 - b.leadingZeroBitCount
}
