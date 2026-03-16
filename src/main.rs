mod display;
mod puzzle;
mod simulation;
mod solver;
mod types;

use std::env;
use std::fs;
use std::process;

use display::{print_puzzle, print_solution};
use puzzle::parse_puzzle;
use solver::solve;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: railbound-solver <puzzle.rail>");
        eprintln!();
        eprintln!("Puzzle file format:");
        eprintln!("  SIZE <cols> <rows>");
        eprintln!();
        eprintln!("  GRID");
        eprintln!("  <cell> ...   # one row per line");
        eprintln!();
        eprintln!("  Cell codes:");
        eprintln!("    .          wall (impassable)");
        eprintln!("    _          empty placeable cell");
        eprintln!("    H V        fixed straight (horizontal / vertical)");
        eprintln!("    NE NW SE SW  fixed curves");
        eprintln!("    X          fixed crossing");
        eprintln!("    Y<t><b>    fixed switch (trunk=t, branch=b), e.g. YWN");
        eprintln!("    T<l>       tunnel entrance (label a–z, defined in TUNNELS)");
        eprintln!();
        eprintln!("  TRAINS");
        eprintln!("  <id> <row> <col> <dir> <color>   # dir: N S E W; color: integer");
        eprintln!();
        eprintln!("  CABOOSES");
        eprintln!("  <color> <row> <col> <entry_dir>");
        eprintln!();
        eprintln!("  TUNNELS");
        eprintln!("  <label> <er> <ec> <ef> <xr> <xc> <xf>");
        eprintln!();
        eprintln!("  PIECES");
        eprintln!("  straight <n>");
        eprintln!("  curve    <n>");
        eprintln!("  crossing <n>");
        eprintln!("  switch   <n>");
        process::exit(1);
    }

    let path = &args[1];
    let content = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Error reading '{}': {}", path, e);
        process::exit(1);
    });

    let puzzle = parse_puzzle(&content).unwrap_or_else(|e| {
        eprintln!("{}", e);
        process::exit(1);
    });

    println!("Loaded puzzle from '{}'", path);
    print_puzzle(&puzzle);

    println!("Solving…");
    match solve(&puzzle) {
        Some(solution) => {
            println!("Solution found!");
            print_solution(&puzzle, &solution);
        }
        None => {
            println!("No solution exists for this puzzle.");
            process::exit(2);
        }
    }
}
