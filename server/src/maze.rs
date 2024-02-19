use std::collections::{HashSet, VecDeque};

use crate::{BREAKABLE, EMPTY, MAZE_HEIGHT, MAZE_WIDTH, WALL};
use rand::{thread_rng, Rng};

pub fn select_maze(level: usize) -> Vec<u8> {
    let num_removed_bricks = (MAZE_WIDTH / 5) - level;
    generate_maze(MAZE_WIDTH, MAZE_HEIGHT, num_removed_bricks)
        .iter()
        .flatten()
        .cloned()
        .collect()
}

fn adjacent_is(cell: u8, x: usize, y: usize, maze: &[Vec<u8>]) -> bool {
    maze[y][x + 1] == cell && maze[y][x - 1] == cell
}

fn generic_maze() -> Vec<Vec<u8>> {
    vec![
        vec![
            2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
        ],
        vec![
            2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
        ],
        vec![
            2, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 0, 2,
        ],
        vec![
            2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
        ],
        vec![
            2, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 0, 2,
        ],
        vec![
            2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
        ],
        vec![
            2, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 0, 2,
        ],
        vec![
            2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
        ],
        vec![
            2, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 0, 2,
        ],
        vec![
            2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
        ],
        vec![
            2, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 0, 2,
        ],
        vec![
            2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
        ],
        vec![
            2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
        ],
        vec![
            2, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 0, 2,
        ],
        vec![
            2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
        ],
        vec![
            2, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 0, 2,
        ],
        vec![
            2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
        ],
        vec![
            2, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 0, 2,
        ],
        vec![
            2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
        ],
        vec![
            2, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 0, 2,
        ],
        vec![
            2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
        ],
        vec![
            2, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 0, 2,
        ],
        vec![
            2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
        ],
        vec![
            2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
        ],
    ]
}

pub fn generate_maze(width: usize, height: usize, num_removed_bricks: usize) -> Vec<Vec<u8>> {
    let mut rng = rand::thread_rng();

    // Initialize the maze with all walls
    let mut maze = generic_maze();

    // Remove x amount of random bricks from each wall
    for row in (2..11).step_by(2) {
        let mut removed_bricks = 0;
        while removed_bricks < num_removed_bricks {
            let random_index = rng.gen_range(2..width - 2);
            if adjacent_is(WALL, random_index, row, &maze) {
                maze[row][random_index] = EMPTY;
                removed_bricks += 1;
            }
        }
    }

    // Remove x amount of random bricks from each wall
    for row in (13..height - 2).step_by(2) {
        let mut removed_bricks = 0;
        while removed_bricks < num_removed_bricks {
            let random_index = rng.gen_range(2..width - 2);
            if adjacent_is(WALL, random_index, row, &maze) {
                maze[row][random_index] = EMPTY;
                removed_bricks += 1;
            }
        }
    }

    // Add x amount of bricks to each empty row
    for i in (3..11).step_by(2) {
        let mut added_bricks = 0;
        while added_bricks < (width + num_removed_bricks) / 8 {
            let random_index = rng.gen_range(2..width - 2);
            if adjacent_is(EMPTY, random_index, i, &maze) {
                maze[i][random_index] = WALL;
                // Add some random breakables
                if rng.gen_range(0..10) > 3 {
                    maze[i][random_index] = BREAKABLE;
                }
                added_bricks += 1;
            }
        }
    }

    // Add x amount of bricks to each empty row
    for i in (12..height - 3).step_by(2) {
        let mut added_bricks = 0;
        while added_bricks < (width + num_removed_bricks) / 8 {
            let random_index = rng.gen_range(2..width - 2);
            if adjacent_is(EMPTY, random_index, i, &maze) {
                maze[i][random_index] = WALL;
                // Add some random breakables
                if rng.gen_range(0..10) > 3 {
                    maze[i][random_index] = BREAKABLE;
                }
                added_bricks += 1;
            }
        }
    }

    fix_enclosed_areas(&mut maze);
    add_breakable_walls(&mut maze, num_removed_bricks);
    maze
}

fn is_reachable_from_start(maze: &[Vec<u8>], start_row: usize, start_col: usize) -> bool {
    let mut visited: HashSet<(usize, usize)> = HashSet::new();
    let mut queue: VecDeque<(usize, usize)> = VecDeque::new();

    queue.push_back((start_row, start_col));

    while let Some((row, col)) = queue.pop_front() {
        if row == 1 && col == 1 {
            return true; // Reached the start cell, path is reachable
        }

        let moves = [(0, 1), (0, -1), (1, 0), (-1, 0)];

        for (dx, dy) in moves.iter() {
            let new_row = row as i32 + dy;
            let new_col = col as i32 + dx;

            if new_row >= 0
                && new_row < maze.len() as i32
                && new_col >= 0
                && new_col < maze[0].len() as i32
                && maze[new_row as usize][new_col as usize] == EMPTY
                && !visited.contains(&(new_row as usize, new_col as usize))
            {
                visited.insert((new_row as usize, new_col as usize));
                queue.push_back((new_row as usize, new_col as usize));
            }
        }
    }

    false // Path is not reachable from the start
}

fn fix_enclosed_areas(maze: &mut Vec<Vec<u8>>) {
    // Iterate through each path cell in the maze
    for i in 1..maze.len() - 1 {
        for j in 1..maze[0].len() - 1 {
            if maze[i][j] == EMPTY {
                // Temporarily change the path to a wall

                // Check if the path is reachable from the start
                if !is_reachable_from_start(maze, i, j) {
                    // If not reachable, replace any adjacent wall with a path
                    for (dx, dy) in &[(0, 1), (0, -1), (1, 0), (-1, 0)] {
                        let new_row = (i as i32 + dy) as usize;
                        let new_col = (j as i32 + dx) as usize;
                        if maze[new_row][new_col] == WALL {
                            maze[new_row][new_col] = EMPTY;
                            break;
                        }
                    }
                }
            }
        }
    }
}

fn add_breakable_walls(maze: &mut [Vec<u8>], num_removed_bricks: usize) {
    let mut rng = thread_rng();
    let mut total_added = num_removed_bricks * 2;
    while total_added > 0 {
        let rand_row = rng.gen_range(2..maze.len() - 2);
        let rand_col = rng.gen_range(2..maze[0].len() - 2);
        if maze[rand_row][rand_col] == WALL {
            maze[rand_row][rand_col] = BREAKABLE;
            total_added -= 1;
        }
    }
}
