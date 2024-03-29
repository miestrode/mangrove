use std::{
    fmt::{self, Display},
    iter,
    ops::{BitAnd, BitOr, BitOrAssign, BitXor, BitXorAssign, Not},
    str::FromStr,
};

use rustifact::ToTokenStream;

use crate::square::Square;

#[macro_export]
/// Macro for generating a bitboard. Performs no input validation. An invocation must look like:
///
/// ```ignore
/// # use hash_bootstrap::bb;
///
/// bb!(
///     0bXXXXXXXX
///     0bXXXXXXXX
///     0bXXXXXXXX
///     0bXXXXXXXX
///     0bXXXXXXXX
///     0bXXXXXXXX
///     0bXXXXXXXX
///     0bXXXXXXXX
/// );
/// ```
/// Where `X` is either `0` or `1`, and represents the bits of the bitboard. Note that newlines
/// aren't specifically needed.
macro_rules! bb {
    ($line0:tt $line1:tt $line2:tt $line3:tt $line4:tt $line5:tt $line6:tt $line7:tt) => {
        BitBoard(
            ((($line0 as u64) << 56)
                | ($line1 << 48)
                | ($line2 << 40)
                | ($line3 << 32)
                | ($line4 << 24)
                | ($line5 << 16)
                | ($line6 << 8)
                | $line7)
                .reverse_bits()
                .swap_bytes(),
        )
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ToTokenStream)]
/// A Bitboard is a 64-bit integer representing the 64 squares of a Chess board. Each bit of the
/// integer is mapped to a Chess board square. The bits can represent anything, but typically
/// represent certain predicates that hold true on squares with value `1`.
///
/// For example, consider a bitboard of all the white pawns. Each square with a `1` is a square
/// holding a white pawn, and each square with a `0` doesn't have on it a white pawn.
///
/// Bitboards are internally used to represent the game board, via [`Board`] and subsequently for
/// [`Game`].
///
/// Generally bitboards aren't constructed, but rather are incrementally modified, although
/// static construction facilities do exist in the form of [`bb!`]. Also, one can directly construct
/// a bitboard via using `BitBoard(x)`, where `x` is a `u64`.
pub struct BitBoard(pub u64);

struct PartialSubsetIter {
    bitboard: BitBoard,
    subset: u64,
}

impl Iterator for PartialSubsetIter {
    type Item = BitBoard;

    fn next(&mut self) -> Option<Self::Item> {
        self.subset = self.subset.wrapping_sub(self.bitboard.0) & self.bitboard.0;

        if self.subset == 0 {
            None
        } else {
            Some(BitBoard(self.subset))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(2usize.pow(self.bitboard.count_ones())))
    }
}

struct BitIter {
    bitboard: BitBoard,
}

impl Iterator for BitIter {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bitboard.is_empty() {
            None
        } else {
            Some(self.bitboard.pop_first_one().unwrap())
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let exact_size = self.bitboard.count_ones() as usize;

        (exact_size, Some(exact_size))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A color in Chess, either [`White`](Color::White) or [`Black`](Color::Black). Used in [`Piece`],
/// [`Board`] and more.
pub enum Color {
    White,
    Black,
}

impl Not for Color {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("string must be `w` or `b`")]
pub struct ParseColorError;

impl FromStr for Color {
    type Err = ParseColorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "w" => Ok(Self::White),
            "b" => Ok(Self::Black),
            _ => Err(ParseColorError),
        }
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Color::White => 'w',
            Color::Black => 'b',
        }
        .fmt(f)
    }
}

impl BitBoard {
    /// The empty bitboard, containing no `1`s.
    pub const EMPTY: Self = Self(0);

    /// The full bitboard, containing all `1`s.
    pub const FULL: Self = Self(u64::MAX);

    /// A bitboard containing `1`s for each square on the A file.
    pub const A_FILE: Self = bb!(
        0b10000000
        0b10000000
        0b10000000
        0b10000000
        0b10000000
        0b10000000
        0b10000000
        0b10000000
    );

    /// A bitboard containing `1`s for each square on the H file.
    pub const H_FILE: Self = bb!(
        0b00000001
        0b00000001
        0b00000001
        0b00000001
        0b00000001
        0b00000001
        0b00000001
        0b00000001
    );

    /// A bitboard containing `1`s for each square on the A, or H file.
    pub const EDGE_FILES: Self = bb!(
        0b10000001
        0b10000001
        0b10000001
        0b10000001
        0b10000001
        0b10000001
        0b10000001
        0b10000001
    );

    /// A bitboard containing `1`s for each square on the first rank.
    pub const RANK_1: Self = bb!(
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b11111111
    );

    /// A bitboard containing `1`s for each square on the second rank.
    pub const RANK_2: Self = bb!(
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b11111111
        0b00000000
    );

    /// A bitboard containing `1`s for each square on the eighth rank.
    pub const RANK_8: Self = bb!(
        0b11111111
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
    );

    /// A bitboard containing `1`s for each square on the seventh rank.
    pub const RANK_7: Self = bb!(
        0b00000000
        0b11111111
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
    );

    pub const EDGE_RANKS: Self = bb!(
        0b11111111
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b11111111
    );

    pub const EDGES: BitBoard = bb!(
        0b11111111
        0b10000001
        0b10000001
        0b10000001
        0b10000001
        0b10000001
        0b10000001
        0b11111111
    );

    // Used to check both if a piece attacks a spot between the king and rook and if the space
    // between them is empty.
    pub const WHITE_KING_SIDE_CASTLE_MASK: Self = bb!(
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000110
    );

    // Used to check if there are any pieces between the rook and king
    pub const WHITE_QUEEN_SIDE_CASTLE_OCCUPATION_MASK: Self = bb!(
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b01110000
    );

    // Used to check if there are any attacks between the king and the king's final spot
    pub const WHITE_QUEEN_SIDE_CASTLE_ATTACK_MASK: Self = bb!(
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00110000
    );

    pub const BLACK_KING_SIDE_CASTLE_MASK: Self = Self::WHITE_KING_SIDE_CASTLE_MASK.vertical_flip();

    pub const BLACK_QUEEN_SIDE_CASTLE_ATTACK_MASK: Self =
        Self::WHITE_QUEEN_SIDE_CASTLE_ATTACK_MASK.vertical_flip();

    pub const BLACK_QUEEN_SIDE_CASTLE_OCCUPATION_MASK: Self =
        Self::WHITE_QUEEN_SIDE_CASTLE_OCCUPATION_MASK.vertical_flip();

    pub const PAWN_START_RANKS: Self = bb!(
        0b00000000
        0b11111111
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b11111111
        0b00000000
    );

    pub const WHITE_EN_PASSANT_CAPTURE_RANKS: Self = bb!(
        0b00000000
        0b00000000
        0b11111111
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
    );

    pub const BLACK_EN_PASSANT_CAPTURE_RANKS: Self = bb!(
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b11111111
        0b00000000
        0b00000000
    );

    pub const KING_CASTLE_MOVES: Self = bb!(
        0b00101010
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00000000
        0b00101010
    );

    pub fn king_side_castle_mask(color: Color) -> Self {
        match color {
            Color::White => Self::WHITE_KING_SIDE_CASTLE_MASK,
            Color::Black => Self::BLACK_KING_SIDE_CASTLE_MASK,
        }
    }

    pub fn queen_side_castle_occupation_mask(color: Color) -> Self {
        match color {
            Color::White => Self::WHITE_QUEEN_SIDE_CASTLE_OCCUPATION_MASK,
            Color::Black => Self::BLACK_QUEEN_SIDE_CASTLE_OCCUPATION_MASK,
        }
    }

    pub fn queen_side_castle_attack_mask(color: Color) -> Self {
        match color {
            Color::White => Self::WHITE_QUEEN_SIDE_CASTLE_ATTACK_MASK,
            Color::Black => Self::BLACK_QUEEN_SIDE_CASTLE_ATTACK_MASK,
        }
    }

    /// Checks if the bitboard contains a single `1` bit.
    pub fn is_a_single_one(&self) -> bool {
        self.0.is_power_of_two()
    }

    /// Checks if the bitboard is full, implying it's equal to the full bitboard [`BitBoard::FULL`].
    pub fn is_full(&self) -> bool {
        *self == Self::FULL
    }

    /// Checks if the bitboard is empty, implying it's equal to the empty bitboard [`BitBoard::EMPTY`].
    pub fn is_empty(&self) -> bool {
        *self == Self::EMPTY
    }

    /// Returns an iterator over the all the subsets of this bitboard, where a subset `x` of `y`
    /// satisfies `x & y == x`. This means that the empty bitboard is always a subset of a bitboard
    /// and so also appears here.
    ///
    /// # Example
    /// ```ignore
    /// # use hash_bootstrap::BitBoard;
    ///
    /// let full = BitBoard::FULL;
    ///
    /// for subset in full.subsets() {
    ///     println!("{subset}");
    /// }
    /// ```
    ///
    /// # Implementation
    /// Internally this uses a carry-rippler implementation, instead of something like `PDEP`.
    pub fn subsets(&self) -> impl Iterator<Item = BitBoard> {
        iter::once(BitBoard::EMPTY).chain(PartialSubsetIter {
            bitboard: *self,
            subset: 0,
        })
    }

    /// Returns an iterator over every single `1` bit in this bitboard, where each `1` bit is
    /// represented by it's corresponding square.
    ///
    /// # Example
    /// ```rust
    /// # use hash_bootstrap::BitBoard;
    ///
    ///  // Ad infinitum!
    ///  for square in BitBoard::FULL.bits() {
    ///     println!("{square}");
    ///  }
    /// ```
    pub fn bits(&self) -> impl Iterator<Item = Square> {
        BitIter { bitboard: *self }
    }

    /// Returns the first `1` bit of the bitboard, according to the square-ordering (See [`Square`])
    /// as a square. If there are no `1`s in this bitboard [`None`] is returned.
    pub fn first_one_as_square(&self) -> Option<Square> {
        // If conversion fails, this means the bitboard is empty, as there will be 64 trailing zeros
        Square::try_from(self.0.trailing_zeros() as u8).ok()
    }

    /// Removes the first `1` bit of the bitboard (according to square-ordering, see
    /// [`Square::ALL`]) and returns it's position as a square.
    ///
    /// If the bitboard is empty [`None`](None) is returned and [`Some`](Some) is
    /// otherwise.
    pub fn pop_first_one(&mut self) -> Option<Square> {
        self.first_one_as_square().map(|square| {
            self.0 &= self.0 - 1;
            square
        })
    }

    fn shift_visually_right(self, squares: u8) -> Self {
        Self(self.0 << squares)
    }

    fn shift_visually_left(self, squares: u8) -> Self {
        Self(self.0 >> squares)
    }

    /// Moves the bits in this bitboard a rank up, relative to the color supplied. This means that
    /// if the color supplied is [`Black`](Color::Black) for example, the result will be "up" from
    /// black's side.
    ///
    /// If any bits would be moved out of the board, they will "disappear".
    pub fn move_one_up(self, color: Color) -> Self {
        match color {
            Color::White => (self & !Self::RANK_8).shift_visually_right(8),
            Color::Black => (self & !Self::RANK_1).shift_visually_left(8),
        }
    }

    /// Moves the bits in this bitboard two ranks up, relative to the color supplied. This means that
    /// if the color supplied is [`Black`](Color::Black) for example, the result will be "up" from
    /// black's side - as in how a player in the black chair would see things.
    ///
    /// If any bits would be moved out of the board, they will "disappear".
    pub fn move_two_up(self, color: Color) -> Self {
        match color {
            Color::White => (self & !(Self::RANK_7 | Self::RANK_8)).shift_visually_right(16),
            Color::Black => (self & !(Self::RANK_1 | Self::RANK_2)).shift_visually_left(16),
        }
    }

    /// Moves the bits in this bitboard a rank down, relative to the color supplied. This means that
    /// if the color supplied is [`Black`](Color::Black) for example, the result will be "down" from
    /// black's side - as in how a player in the black chair would see things.
    ///
    /// If any bits would be moved out of the board, they will "disappear".
    pub fn move_one_down(self, color: Color) -> Self {
        self.move_one_up(!color)
    }

    /// Moves the bits in this bitboard a file up, relative to the color supplied. This means that
    /// if the color supplied is [`Black`](Color::Black) for example, the result will be "up" from
    /// black's side - as in how a player in the black chair would see things.
    ///
    /// If any bits would be moved out of the board, they will "disappear".
    pub fn move_one_right(self, color: Color) -> Self {
        match color {
            Color::White => (self & !Self::H_FILE).shift_visually_right(1),
            Color::Black => (self & !Self::A_FILE).shift_visually_left(1),
        }
    }

    /// Moves the bits in this bitboard a file down, relative to the color supplied. This means that
    /// if the color supplied is [`Black`](Color::Black) for example, the result will be "down" from
    /// black's side - as in how a player in the black chair would see things.
    ///
    /// If any bits would be moved out of the board, they will "disappear".
    pub fn move_one_left(self, color: Color) -> Self {
        self.move_one_right(!color)
    }

    /// A combination of applying [`BitBoard::move_one_up`] and [`BitBoard::move_one_right`].
    pub fn move_one_up_right(self, color: Color) -> Self {
        self.move_one_up(color).move_one_right(color)
    }

    /// A combination of applying [`BitBoard::move_one_up`] and [`BitBoard::move_one_left`].
    pub fn move_one_up_left(self, color: Color) -> Self {
        self.move_one_up(color).move_one_left(color)
    }

    /// A combination of applying [`BitBoard::move_one_down`] and [`BitBoard::move_one_left`].
    pub fn move_one_down_left(self, color: Color) -> Self {
        self.move_one_down(color).move_one_left(color)
    }

    /// A combination of applying [`BitBoard::move_one_down`] and [`BitBoard::move_one_right`].
    pub fn move_one_down_right(self, color: Color) -> Self {
        self.move_one_down(color).move_one_right(color)
    }

    /// Counts the number of bits set to `1` in the bitboard.
    ///
    /// # Example
    /// ```rust
    /// # use hash_bootstrap::BitBoard;
    ///
    /// let bb = BitBoard::FULL;
    ///
    /// assert_eq!(bb.count_ones(), 64);
    /// ```
    pub fn count_ones(&self) -> u32 {
        self.0.count_ones()
    }

    /// Gets the value of the bit at the corresponding square, as a boolean, with `1` being `true` and `0`, `false`.
    ///
    /// # Example
    /// ```rust
    /// # use hash_bootstrap::{BitBoard, Square};
    ///
    /// let bb = BitBoard::EMPTY;
    ///
    /// assert!(!bb.get_bit(Square::H1));
    /// ```
    pub fn get_bit(&self, square: Square) -> bool {
        (*self & square.into()) != BitBoard::EMPTY
    }

    /// Toggles the bit as specified by the square. If the bit was a `1`, it will become `0`, and if it was `0` it would become `1`.
    ///
    /// # Example
    /// ```rust
    /// # use hash_bootstrap::{BitBoard, Square};
    ///
    /// let mut bb = BitBoard::EMPTY;
    ///
    /// bb.toggle_bit(Square::E4);
    /// assert!(bb.get_bit(Square::E4));
    /// ```
    pub fn toggle_bit(&mut self, square: Square) {
        *self ^= BitBoard::from(square);
    }

    /// Flips the bitboard along the horizontal axis. For example, given:
    /// ```text
    /// . 1 1 1 1 . . .
    /// . 1 . . . 1 . .
    /// . 1 . . . 1 . .
    /// . 1 . . 1 . . .
    /// . 1 1 1 . . . .
    /// . 1 . 1 . . . .
    /// . 1 . . 1 . . .
    /// . 1 . . . 1 . .
    /// ```
    ///
    /// The result would be:
    /// ```text
    /// . 1 . . . 1 . .
    /// . 1 . . 1 . . .
    /// . 1 . 1 . . . .
    /// . 1 1 1 . . . .
    /// . 1 . . 1 . . .
    /// . 1 . . . 1 . .
    /// . 1 . . . 1 . .
    /// . 1 1 1 1 . . .
    /// ```
    pub const fn vertical_flip(self) -> Self {
        Self(self.0.swap_bytes())
    }

    /// Smears all of the `1` bits of the bitboard one rank up relative to the context `color`.
    /// Equivalent to `bb.move_one_up(color) + bb`.
    ///
    /// As an example, consider the input:
    /// ```text
    /// . . . 1 1 1 1 .
    /// . . 1 . . . 1 .
    /// . . 1 . . . 1 .
    /// . . . 1 . . 1 .
    /// . . . . 1 1 1 .
    /// . . . . 1 . 1 .
    /// . . . 1 . . 1 .
    /// . . 1 . . . 1 .
    /// ```
    ///
    /// The output would be a bitboard like:
    /// ```text
    /// . . 1 1 1 1 1 .
    /// . . 1 . . . 1 .
    /// . . 1 1 . . 1 .
    /// . . . 1 1 1 1 .
    /// . . . . 1 1 1 .
    /// . . . 1 1 . 1 .
    /// . . 1 1 . . 1 .
    /// . . 1 . . . 1 .
    /// ```
    ///
    /// Notice the bit duplication.
    pub fn smear_one_up(self, color: Color) -> Self {
        self.move_one_up(color) | self
    }

    pub fn is_subset_of(&self, other: Self) -> bool {
        *self & other == *self
    }
}

impl Not for BitBoard {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl BitAnd for BitBoard {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitOr for BitBoard {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for BitBoard {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl BitXor for BitBoard {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        BitBoard(self.0 ^ rhs.0)
    }
}

impl BitXorAssign for BitBoard {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs;
    }
}
