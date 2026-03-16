use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

impl Direction {
    pub fn opposite(self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::East => Direction::West,
            Direction::West => Direction::East,
        }
    }

    /// Row/col delta when moving in this direction. Row increases downward.
    pub fn delta(self) -> (i32, i32) {
        match self {
            Direction::North => (-1, 0),
            Direction::South => (1, 0),
            Direction::East => (0, 1),
            Direction::West => (0, -1),
        }
    }

    pub fn from_char(c: char) -> Option<Direction> {
        match c.to_ascii_uppercase() {
            'N' => Some(Direction::North),
            'S' => Some(Direction::South),
            'E' => Some(Direction::East),
            'W' => Some(Direction::West),
            _ => None,
        }
    }

    pub fn from_str(s: &str) -> Option<Direction> {
        if s.len() == 1 {
            s.chars().next().and_then(Direction::from_char)
        } else {
            match s.to_uppercase().as_str() {
                "NORTH" => Some(Direction::North),
                "SOUTH" => Some(Direction::South),
                "EAST" => Some(Direction::East),
                "WEST" => Some(Direction::West),
                _ => None,
            }
        }
    }

    pub fn all() -> [Direction; 4] {
        [Direction::North, Direction::South, Direction::East, Direction::West]
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Direction::North => write!(f, "N"),
            Direction::South => write!(f, "S"),
            Direction::East => write!(f, "E"),
            Direction::West => write!(f, "W"),
        }
    }
}

/// Switch configuration.
/// `trunk`: the single-end side where trains diverge (flip between straight and branch).
/// `straight`: the through path (when not flipped, trunk→straight).
/// `branch`: the diverging path (when flipped, trunk→branch).
/// Trains entering from `straight` or `branch` always exit via `trunk`, and toggle the switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SwitchConfig {
    pub trunk: Direction,
    pub straight: Direction,
    pub branch: Direction,
}

impl SwitchConfig {
    /// All 8 valid switch configurations (trunk + branch, straight is implied as opposite of trunk).
    pub fn all() -> [SwitchConfig; 8] {
        [
            SwitchConfig { trunk: Direction::West,  straight: Direction::East,  branch: Direction::North },
            SwitchConfig { trunk: Direction::West,  straight: Direction::East,  branch: Direction::South },
            SwitchConfig { trunk: Direction::East,  straight: Direction::West,  branch: Direction::North },
            SwitchConfig { trunk: Direction::East,  straight: Direction::West,  branch: Direction::South },
            SwitchConfig { trunk: Direction::North, straight: Direction::South, branch: Direction::East  },
            SwitchConfig { trunk: Direction::North, straight: Direction::South, branch: Direction::West  },
            SwitchConfig { trunk: Direction::South, straight: Direction::North, branch: Direction::East  },
            SwitchConfig { trunk: Direction::South, straight: Direction::North, branch: Direction::West  },
        ]
    }

    /// Route a train entering via `entry_side`.
    /// Returns `(exit_side, should_toggle_switch)` or `None` if the train crashes.
    /// `flipped`: true means trunk→branch is active, false means trunk→straight.
    pub fn route(self, entry_side: Direction, flipped: bool) -> Option<(Direction, bool)> {
        if entry_side == self.trunk {
            let exit = if flipped { self.branch } else { self.straight };
            Some((exit, true))
        } else if entry_side == self.straight || entry_side == self.branch {
            Some((self.trunk, true))
        } else {
            None
        }
    }

    pub fn open_sides(self) -> [Direction; 3] {
        [self.trunk, self.straight, self.branch]
    }
}

/// A track piece type with its routing behavior.
/// Convention: `entry_side` is the side of the cell the train enters FROM.
/// A train moving East enters the West side of the next cell (entry_side = West).
/// The returned exit_side is the side through which the train leaves,
/// and becomes the train's new direction of travel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TrackType {
    StraightH,        // West ↔ East
    StraightV,        // North ↔ South
    CurveNE,          // North ↔ East  (╚ shape)
    CurveNW,          // North ↔ West  (╝ shape)
    CurveSE,          // South ↔ East  (╔ shape)
    CurveSW,          // South ↔ West  (╗ shape)
    Crossing,         // West↔East AND North↔South independently
    Switch(SwitchConfig),
}

impl TrackType {
    /// Route a train entering via `entry_side`.
    /// Returns `(exit_side, toggle_switch)` or `None` if the train crashes.
    pub fn route(self, entry_side: Direction, switch_flipped: bool) -> Option<(Direction, bool)> {
        match self {
            TrackType::StraightH => match entry_side {
                Direction::West => Some((Direction::East, false)),
                Direction::East => Some((Direction::West, false)),
                _ => None,
            },
            TrackType::StraightV => match entry_side {
                Direction::North => Some((Direction::South, false)),
                Direction::South => Some((Direction::North, false)),
                _ => None,
            },
            TrackType::CurveNE => match entry_side {
                Direction::North => Some((Direction::East, false)),
                Direction::East => Some((Direction::North, false)),
                _ => None,
            },
            TrackType::CurveNW => match entry_side {
                Direction::North => Some((Direction::West, false)),
                Direction::West => Some((Direction::North, false)),
                _ => None,
            },
            TrackType::CurveSE => match entry_side {
                Direction::South => Some((Direction::East, false)),
                Direction::East => Some((Direction::South, false)),
                _ => None,
            },
            TrackType::CurveSW => match entry_side {
                Direction::South => Some((Direction::West, false)),
                Direction::West => Some((Direction::South, false)),
                _ => None,
            },
            TrackType::Crossing => {
                // Both H and V pass through independently
                match entry_side {
                    Direction::West  => Some((Direction::East,  false)),
                    Direction::East  => Some((Direction::West,  false)),
                    Direction::North => Some((Direction::South, false)),
                    Direction::South => Some((Direction::North, false)),
                }
            }
            TrackType::Switch(cfg) => cfg.route(entry_side, switch_flipped),
        }
    }

    /// Sides that are open (connected) on this track type.
    pub fn open_sides(self) -> Vec<Direction> {
        match self {
            TrackType::StraightH  => vec![Direction::West,  Direction::East],
            TrackType::StraightV  => vec![Direction::North, Direction::South],
            TrackType::CurveNE    => vec![Direction::North, Direction::East],
            TrackType::CurveNW    => vec![Direction::North, Direction::West],
            TrackType::CurveSE    => vec![Direction::South, Direction::East],
            TrackType::CurveSW    => vec![Direction::South, Direction::West],
            TrackType::Crossing   => vec![Direction::North, Direction::South, Direction::East, Direction::West],
            TrackType::Switch(cfg) => cfg.open_sides().to_vec(),
        }
    }

    /// Unicode display character.
    pub fn display_char(self) -> &'static str {
        match self {
            TrackType::StraightH  => "═",
            TrackType::StraightV  => "║",
            TrackType::CurveNE    => "╚",
            TrackType::CurveNW    => "╝",
            TrackType::CurveSE    => "╔",
            TrackType::CurveSW    => "╗",
            TrackType::Crossing   => "╬",
            TrackType::Switch(cfg) => match (cfg.trunk, cfg.branch) {
                (Direction::West,  Direction::North) |
                (Direction::East,  Direction::North) => "╦",
                (Direction::West,  Direction::South) |
                (Direction::East,  Direction::South) => "╩",
                (Direction::North, Direction::East)  |
                (Direction::South, Direction::East)  => "╠",
                (Direction::North, Direction::West)  |
                (Direction::South, Direction::West)  => "╣",
                _ => "┼",
            },
        }
    }

    /// Parse a track type from the grid cell code used in puzzle files.
    pub fn from_code(s: &str) -> Option<TrackType> {
        match s {
            "H"  => Some(TrackType::StraightH),
            "V"  => Some(TrackType::StraightV),
            "NE" => Some(TrackType::CurveNE),
            "NW" => Some(TrackType::CurveNW),
            "SE" => Some(TrackType::CurveSE),
            "SW" => Some(TrackType::CurveSW),
            "X"  => Some(TrackType::Crossing),
            _ if s.starts_with('Y') && s.len() == 3 => {
                // Format: Y<trunk><branch>, e.g., YWN = trunk=W, branch=N
                let mut chars = s.chars().skip(1);
                let trunk  = chars.next().and_then(Direction::from_char)?;
                let branch = chars.next().and_then(Direction::from_char)?;
                if trunk == branch || trunk == branch.opposite() {
                    return None; // invalid
                }
                let straight = trunk.opposite();
                Some(TrackType::Switch(SwitchConfig { trunk, straight, branch }))
            }
            _ => None,
        }
    }

    /// All orientations for a "straight" inventory piece.
    pub fn straight_orientations() -> &'static [TrackType] {
        &[TrackType::StraightH, TrackType::StraightV]
    }

    /// All orientations for a "curve" inventory piece.
    pub fn curve_orientations() -> &'static [TrackType] {
        &[TrackType::CurveNE, TrackType::CurveNW, TrackType::CurveSE, TrackType::CurveSW]
    }

    /// All orientations for a "crossing" inventory piece.
    pub fn crossing_orientations() -> &'static [TrackType] {
        &[TrackType::Crossing]
    }

    /// All orientations for a "switch" inventory piece.
    pub fn switch_orientations() -> Vec<TrackType> {
        SwitchConfig::all().iter().map(|&cfg| TrackType::Switch(cfg)).collect()
    }
}

/// How many of each placeable piece type remain in the player's inventory.
#[derive(Clone, Debug, Default)]
pub struct Inventory {
    pub straight: usize,
    pub curve: usize,
    pub crossing: usize,
    pub switch: usize,
}

impl Inventory {
    pub fn is_empty(&self) -> bool {
        self.straight == 0 && self.curve == 0 && self.crossing == 0 && self.switch == 0
    }

    pub fn total(&self) -> usize {
        self.straight + self.curve + self.crossing + self.switch
    }
}
