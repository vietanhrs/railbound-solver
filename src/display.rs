use crate::puzzle::{CellKind, Puzzle};
use crate::solver::Solution;
use crate::types::TrackType;

/// Print the solved puzzle to stdout using Unicode box-drawing characters.
pub fn print_solution(puzzle: &Puzzle, solution: &Solution) {
    println!();
    println!("╔═══ Solution ═══╗");
    println!();

    // Build a display grid
    let mut display: Vec<Vec<String>> = (0..puzzle.rows)
        .map(|_| (0..puzzle.cols).map(|_| "·".to_owned()).collect())
        .collect();

    // 1. Fixed tracks
    for r in 0..puzzle.rows {
        for c in 0..puzzle.cols {
            match &puzzle.grid[r][c] {
                CellKind::Wall      => display[r][c] = " ".to_owned(),
                CellKind::Placeable => display[r][c] = "░".to_owned(),
                CellKind::Fixed(t)  => display[r][c] = t.display_char().to_owned(),
                CellKind::Tunnel(l) => display[r][c] = format!("T{}", l),
                CellKind::Terminal  => {} // overwritten below
            }
        }
    }

    // 2. Placed (solved) tracks
    for (&(r, c), track) in solution {
        display[r][c] = track.display_char().to_owned();
    }

    // 3. Cabooses
    for cab in &puzzle.cabooses {
        display[cab.row][cab.col] = format!("C{}", cab.color);
    }

    // 4. Train cars (shown at starting positions)
    for car in &puzzle.cars {
        let arrow = match car.dir {
            crate::types::Direction::North => "↑",
            crate::types::Direction::South => "↓",
            crate::types::Direction::East  => "→",
            crate::types::Direction::West  => "←",
        };
        display[car.row][car.col] = format!("{}{}", arrow, car.id);
    }

    // ── Determine column widths ──────────────────────────────────────────────
    let col_widths: Vec<usize> = (0..puzzle.cols)
        .map(|c| {
            (0..puzzle.rows)
                .map(|r| visible_len(&display[r][c]))
                .max()
                .unwrap_or(1)
        })
        .collect();

    // ── Print top border ─────────────────────────────────────────────────────
    let total_width: usize = col_widths.iter().sum::<usize>() + col_widths.len() * 3 + 1;
    println!("┌{}┐", "─".repeat(total_width - 2));

    // ── Print rows ───────────────────────────────────────────────────────────
    for r in 0..puzzle.rows {
        print!("│");
        for c in 0..puzzle.cols {
            let cell = &display[r][c];
            let pad = col_widths[c].saturating_sub(visible_len(cell));
            print!(" {}{} │", cell, " ".repeat(pad));
        }
        println!();

        if r < puzzle.rows - 1 {
            print!("├");
            for (ci, &w) in col_widths.iter().enumerate() {
                print!("{}", "─".repeat(w + 2));
                if ci < puzzle.cols - 1 {
                    print!("┼");
                }
            }
            println!("┤");
        }
    }

    // ── Print bottom border ──────────────────────────────────────────────────
    println!("└{}┘", "─".repeat(total_width - 2));

    // ── Legend ───────────────────────────────────────────────────────────────
    println!();
    println!("Legend:");
    println!("  ↑/↓/→/←  Train car starting position & direction");
    println!("  C<n>      Caboose for color group <n>");
    println!("  T<l>      Tunnel entrance/exit (label)");
    println!("  ░         Empty placeable cell (no piece used here)");
    println!("  ·         Empty wall cell");
    println!("  ═ ║ ╚ ╝ ╔ ╗ ╬ ╦ ╩ ╠ ╣   Track pieces");
    println!();

    // ── Placed pieces summary ────────────────────────────────────────────────
    if solution.is_empty() {
        println!("No pieces were needed.");
    } else {
        println!("Placed {} piece(s):", solution.len());
        let mut sorted: Vec<_> = solution.iter().collect();
        sorted.sort_by_key(|&(&(r, c), _)| (r, c));
        for (&(r, c), track) in sorted {
            println!("  ({r},{c}): {}", track_name(*track));
        }
    }
    println!();
}

/// Print an unsolved puzzle (for inspection / debug).
pub fn print_puzzle(puzzle: &Puzzle) {
    println!();
    println!("Puzzle  ({} cols × {} rows)", puzzle.cols, puzzle.rows);

    let mut display: Vec<Vec<String>> = (0..puzzle.rows)
        .map(|_| (0..puzzle.cols).map(|_| "·".to_owned()).collect())
        .collect();

    for r in 0..puzzle.rows {
        for c in 0..puzzle.cols {
            match &puzzle.grid[r][c] {
                CellKind::Wall      => display[r][c] = " ".to_owned(),
                CellKind::Placeable => display[r][c] = "░".to_owned(),
                CellKind::Fixed(t)  => display[r][c] = t.display_char().to_owned(),
                CellKind::Tunnel(l) => display[r][c] = format!("T{}", l),
                CellKind::Terminal  => {}
            }
        }
    }
    for cab in &puzzle.cabooses {
        display[cab.row][cab.col] = format!("C{}", cab.color);
    }
    for car in &puzzle.cars {
        let arrow = match car.dir {
            crate::types::Direction::North => "↑",
            crate::types::Direction::South => "↓",
            crate::types::Direction::East  => "→",
            crate::types::Direction::West  => "←",
        };
        display[car.row][car.col] = format!("{}{}", arrow, car.id);
    }

    let col_widths: Vec<usize> = (0..puzzle.cols)
        .map(|c| (0..puzzle.rows).map(|r| visible_len(&display[r][c])).max().unwrap_or(1))
        .collect();
    let total_width: usize = col_widths.iter().sum::<usize>() + col_widths.len() * 3 + 1;

    println!("┌{}┐", "─".repeat(total_width - 2));
    for r in 0..puzzle.rows {
        print!("│");
        for c in 0..puzzle.cols {
            let cell = &display[r][c];
            let pad = col_widths[c].saturating_sub(visible_len(cell));
            print!(" {}{} │", cell, " ".repeat(pad));
        }
        println!();
        if r < puzzle.rows - 1 {
            print!("├");
            for (ci, &w) in col_widths.iter().enumerate() {
                print!("{}", "─".repeat(w + 2));
                if ci < puzzle.cols - 1 { print!("┼"); }
            }
            println!("┤");
        }
    }
    println!("└{}┘", "─".repeat(total_width - 2));

    // Inventory
    let inv = &puzzle.inventory;
    println!();
    println!(
        "Inventory: {} straight, {} curve, {} crossing, {} switch",
        inv.straight, inv.curve, inv.crossing, inv.switch
    );
    println!("Cars: {} | Cabooses: {} | Tunnels: {}",
        puzzle.cars.len(), puzzle.cabooses.len(), puzzle.tunnels.len());
    println!();
}

fn track_name(t: TrackType) -> &'static str {
    match t {
        TrackType::StraightH  => "StraightH (═)",
        TrackType::StraightV  => "StraightV (║)",
        TrackType::CurveNE    => "CurveNE (╚)",
        TrackType::CurveNW    => "CurveNW (╝)",
        TrackType::CurveSE    => "CurveSE (╔)",
        TrackType::CurveSW    => "CurveSW (╗)",
        TrackType::Crossing   => "Crossing (╬)",
        TrackType::Switch(_)  => "Switch",
    }
}

/// Return the display width of a string, accounting for multi-byte Unicode
/// (each Unicode scalar = 1 display column here; box-drawing chars are 1-wide in most terminals).
fn visible_len(s: &str) -> usize {
    s.chars().count()
}
