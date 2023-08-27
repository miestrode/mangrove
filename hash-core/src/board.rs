use std::{mem, str::FromStr};

use hash_build::{BitBoard, Color, Square};

use crate::{
    cache::CacheHash,
    index::{self, zobrist_castling_rights, zobrist_ep_file, zobrist_piece, zobrist_side},
    mg,
    repr::{EpData, Move, MoveMeta, Piece, PieceKind, PieceTable, Pins, Player},
};

#[derive(Clone, Copy)]
pub struct Board {
    pub current_player: Player,
    pub opposing_player: Player,
    pub current_color: Color,
    pub piece_table: PieceTable,
    pub ep_data: Option<EpData>,
    pub hash: u64,
}

impl CacheHash for Board {
    fn hash(&self) -> u64 {
        self.hash
    }
}

impl Board {
    pub fn white_player(&self) -> &Player {
        match self.current_color {
            Color::White => &self.current_player,
            Color::Black => &self.opposing_player,
        }
    }

    pub fn black_player(&self) -> &Player {
        match self.current_color {
            Color::White => &self.opposing_player,
            Color::Black => &self.current_player,
        }
    }

    pub fn get_piece(&self, square: Square) -> Option<Piece> {
        self.piece_table.0[square].map(|kind| Piece {
            kind,
            color: if self.current_player.occupation.get_bit(square) {
                self.current_color
            } else {
                !self.current_color
            },
        })
    }

    #[rustfmt::skip]
    fn update_slide_constraints(&mut self) {
        let diagonal_sliders = self.opposing_player.bishops + self.opposing_player.queens;
        let cross_sliders = self.opposing_player.rooks + self.opposing_player.queens;

        let king_square = self.current_player.king.first_one_as_square();
        let (up, right, down, left) = index::separated_rook_slides(king_square, self.opposing_player.occupation);
        let (up_left, up_right, down_right, down_left) = index::separated_bishop_slides(king_square, self.opposing_player.occupation);

        // NOTE: Macro is here for non-local control flow
        macro_rules! update {
            ($ray:expr, $possible_casters:expr, $pin_mask:expr) => {{
                // The ray stops when it finds an enemy slider, thus this check is sufficient
                if ($ray & $possible_casters).isnt_empty() {
                    let blockers = $ray & self.current_player.occupation;

                    if blockers.is_single_one() {
                        $pin_mask += $ray;
                    } else if blockers.is_empty() {
                        if self.current_player.valid_targets.is_full() {
                            self.current_player.valid_targets = $ray;
                        } else {
                            self.current_player.king_must_move = true;
                            return; // If the king must move then pin and check data are irrelevant
                        }
                    }
                }
            }};
        }

        update!(up,         cross_sliders,    self.current_player.pins.vertical);
        update!(up_right,   diagonal_sliders, self.current_player.pins.diagonal);
        update!(right,      cross_sliders,    self.current_player.pins.horizontal);
        update!(down_right, diagonal_sliders, self.current_player.pins.anti_diagonal);
        update!(down,       cross_sliders,    self.current_player.pins.vertical);
        update!(down_left,  diagonal_sliders, self.current_player.pins.diagonal);
        update!(left,       cross_sliders,    self.current_player.pins.horizontal);
        update!(up_left,    diagonal_sliders, self.current_player.pins.anti_diagonal);
    }

    fn update_non_slide_constraints(&mut self) {
        if self.current_player.king_must_move {
            return;
        }

        // The non-sliding attackers have to be either knights or pawns
        let attackers = (index::knight_attacks(self.current_player.king.first_one_as_square())
            & self.opposing_player.knights)
            + ((self
                .current_player
                .king
                .move_one_up_left(self.current_color)
                + self
                    .current_player
                    .king
                    .move_one_up_right(self.current_color))
                & self.opposing_player.pawns);

        if attackers.is_single_one() {
            if self.current_player.valid_targets.is_full() {
                self.current_player.valid_targets = attackers;
            } else {
                self.current_player.king_must_move = true;
            }
        } else if attackers.isnt_empty() {
            // If this is true, there must be two attackers or more
            self.current_player.king_must_move = true;
        }
    }

    pub fn update_move_constraints(&mut self) {
        self.current_player.king_must_move = false;
        self.current_player.pins = Pins::EMPTY;
        self.current_player.valid_targets = BitBoard::FULL;

        mg::gen_dangers(self);

        self.update_slide_constraints();
        self.update_non_slide_constraints();
    }

    pub(crate) unsafe fn move_piece_unchecked(
        &mut self,
        kind: PieceKind,
        origin: Square,
        target: Square,
    ) {
        let captured_kind = self.piece_table.piece_kind(target);
        self.piece_table.move_piece(origin, target);

        self.hash ^= zobrist_piece(
            Piece {
                kind,
                color: self.current_color,
            },
            origin,
        ) ^ zobrist_piece(
            Piece {
                kind,
                color: self.current_color,
            },
            target,
        );

        // SAFETY: Data is assumed to be valid
        unsafe {
            self.current_player
                .move_piece_unchecked(kind, origin, target);
            if let Some(captured_kind) = captured_kind {
                self.hash ^= zobrist_piece(
                    Piece {
                        kind: captured_kind,
                        color: !self.current_color,
                    },
                    target,
                );

                self.opposing_player.toggle_piece(captured_kind, target);
            }
        }
    }

    // SAFETY: This function assumes the move at hand is actually properly constructed and legal
    // NOTE: The function returns a boolean representing wether the move was a pawn move or piece
    // capture
    pub unsafe fn make_move_unchecked(&mut self, chess_move: &Move) -> bool {
        let past_ep_data = self.ep_data;
        self.ep_data = None;

        if let Some(ep_data) = past_ep_data {
            self.hash ^= zobrist_ep_file(ep_data.pawn.file());
        }

        // Remove the previous castling rights, "stored" in the hash
        self.hash ^= zobrist_castling_rights(&self.current_player.castling_rights)
            ^ zobrist_castling_rights(&self.opposing_player.castling_rights);

        // This only actually affects things if the piece moved captured a castling piece or was a
        // castling piece
        self.current_player.castling_rights.0[chess_move.origin] = false;
        self.opposing_player.castling_rights.0[chess_move.target] = false;

        // Add the new castling rights
        self.hash ^= zobrist_castling_rights(&self.current_player.castling_rights)
            ^ zobrist_castling_rights(&self.opposing_player.castling_rights);

        let is_capture = self.piece_table.0[chess_move.target].is_some();
        let is_pawn_move = chess_move.moved_piece_kind == PieceKind::Pawn;

        // SAFETY: See above
        // TODO: Check if indexing into the piece table like this is faster than storing this
        // information on the move.
        unsafe {
            self.move_piece_unchecked(
                chess_move.moved_piece_kind,
                chess_move.origin,
                chess_move.target,
            )
        };

        match chess_move.meta {
            MoveMeta::Promotion(kind) => {
                self.piece_table.set(Some(kind), chess_move.target);
                self.current_player
                    .toggle_piece(PieceKind::Pawn, chess_move.target);
                self.current_player.toggle_piece(kind, chess_move.target);

                // TODO: Check if there is any optimization to be had by making the hash changes
                // manual (and not be done automatically by the "move_piece_unchecked" function)
                self.hash ^= zobrist_piece(
                    Piece {
                        kind: PieceKind::Pawn,
                        color: self.current_color,
                    },
                    chess_move.target,
                ) ^ zobrist_piece(
                    Piece {
                        kind,
                        color: self.current_color,
                    },
                    chess_move.target,
                )
            }
            MoveMeta::EnPassant => {
                // SAFETY: See above
                let pawn_square = past_ep_data.unwrap().pawn;
                self.opposing_player
                    .toggle_piece(PieceKind::Pawn, pawn_square);
                self.piece_table.set(None, pawn_square);

                self.hash ^= zobrist_piece(
                    Piece {
                        kind: PieceKind::Pawn,
                        color: !self.current_color,
                    },
                    pawn_square,
                );
            }
            MoveMeta::DoublePush => {
                self.hash ^= zobrist_ep_file(chess_move.origin.file());

                self.ep_data = Some(EpData {
                    // SAFETY: See above.
                    capture_point: unsafe {
                        chess_move
                            .target
                            .move_one_down_unchecked(self.current_color)
                    }
                    .as_bitboard(),
                    pawn: chess_move.target,
                });
            }
            MoveMeta::CastleKs => {
                // Based on https://en.wikipedia.org/wiki/Castling
                let (initial_rook, end_rook) = match self.current_color {
                    Color::White => (Square::BOTTOM_RIGHT_ROOK, Square::F1),
                    Color::Black => (Square::TOP_RIGHT_ROOK, Square::F8),
                };

                // SAFETY: See above
                // TODO: Consider using a specialized function to avoid the capture checks that are
                // irrelevant if performance is improved
                unsafe {
                    self.move_piece_unchecked(PieceKind::Rook, initial_rook, end_rook);
                }
            }
            MoveMeta::CastleQs => {
                // Based on https://en.wikipedia.org/wiki/Castling
                let (initial_rook, end_rook) = match self.current_color {
                    Color::White => (Square::BOTTOM_LEFT_ROOK, Square::D1),
                    Color::Black => (Square::TOP_LEFT_ROOK, Square::D8),
                };

                // SAFETY: See above
                // TODO: Consider using a specialized function to avoid the capture checks that are
                // irrelevant if performance is improved
                unsafe {
                    self.move_piece_unchecked(PieceKind::Rook, initial_rook, end_rook);
                }
            }
            MoveMeta::None => {}
        }

        self.hash ^= zobrist_side(self.current_color) ^ zobrist_side(!self.current_color);
        self.current_color = !self.current_color;

        mem::swap(&mut self.current_player, &mut self.opposing_player);
        self.update_move_constraints();

        is_pawn_move || is_capture
    }

    pub fn interpret_move(&self, move_str: &str) -> Result<Move, &'static str> {
        if move_str.len() < 4 || move_str.len() > 5 {
            return Err("Input too short");
        }

        let origin = Square::from_str(&move_str[0..2])?;
        let target = Square::from_str(&move_str[2..4])?;

        let moved_piece_kind = if let Some(piece) = self.get_piece(origin) {
            piece.kind
        } else {
            return Err("Move is impossible in the given context");
        };

        Ok(Move {
            origin,
            target,
            moved_piece_kind,
            meta: if (moved_piece_kind == PieceKind::King)
                && (origin == Square::E1 || origin == Square::E8)
            {
                if target == Square::G1 || target == Square::G8 {
                    MoveMeta::CastleKs
                } else if target == Square::C1 || target == Square::C8 {
                    MoveMeta::CastleQs
                } else {
                    MoveMeta::None
                }
            } else if moved_piece_kind == PieceKind::Pawn {
                if (origin.rank() == 1 || origin.rank() == 6)
                    && (target.rank() == 3 || target.rank() == 4)
                {
                    MoveMeta::DoublePush
                } else if (origin.file() != target.file()) && self.get_piece(target).is_none() {
                    MoveMeta::EnPassant
                } else if move_str.len() == 5 {
                    MoveMeta::Promotion(match &move_str[4..5] {
                        "q" => PieceKind::Queen,
                        "r" => PieceKind::Rook,
                        "b" => PieceKind::Bishop,
                        "n" => PieceKind::Knight,
                        _ => return Err("Invalid promotion piece"),
                    })
                } else {
                    MoveMeta::None
                }
            } else {
                MoveMeta::None
            },
        })
    }
}
