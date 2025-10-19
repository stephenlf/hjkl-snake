use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::{HashSet, VecDeque};

/// Integer coordinate type for grid cells (not pixels)
pub type Coord = i32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point {
    pub x: Coord,
    pub y: Coord,
}

impl Point {
    #[inline]
    pub const fn new(x: Coord, y: Coord) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    #[inline]
    pub fn dx_dy(self) -> (Coord, Coord) {
        match self {
            Self::Up => (0, -1),
            Self::Down => (0, 1),
            Self::Left => (-1, 0),
            Self::Right => (1, 0),
        }
    }

    #[inline]
    pub fn is_opposite(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Up, Self::Down)
                | (Self::Down, Self::Up)
                | (Self::Right, Self::Left)
                | (Self::Left, Self::Right)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus {
    Running,
    Dead,
}

#[derive(Debug, Clone)]
pub struct GameConfig {
    pub width: Coord,
    pub height: Coord,
    pub wrap_edges: bool,
    /// Initial snake length (>= 1)
    pub initial_len: usize,
    /// If true, ensure an odd aspect for Braille rasterization later (2x4 cell mapping)
    pub braille_friendly: bool,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            width: 40,
            height: 24,
            wrap_edges: false,
            initial_len: 4,
            braille_friendly: true,
        }
    }
}

/// UI-agnostic result of a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickResult {
    pub ate_food: bool,
    pub status: GameStatus,
    pub score: u32,
}

#[derive(Debug)]
pub struct GameState {
    cfg: GameConfig,
    snake: VecDeque<Point>,
    dir: Direction,
    /// Applied at the start of the next tick if it's not a 180* turn.
    pending_dir: Option<Direction>,
    food: HashSet<Point>, // Supports multiple foods on the board
    rng: ChaCha8Rng,
    status: GameStatus,
    score: u32,
}

impl GameState {
    pub fn with_seed(cfg: GameConfig, seed: u64) -> Self {
        Self::with_rng(cfg, ChaCha8Rng::seed_from_u64(seed))
    }

    /// Create a new game with deterministic RNG from `seed`.
    pub fn with_rng(cfg: GameConfig, rng: ChaCha8Rng) -> Self {
        let mut game = Self {
            cfg,
            snake: VecDeque::new(),
            dir: Direction::Right,
            pending_dir: None,
            food: HashSet::new(),
            rng: rng,
            status: GameStatus::Running,
            score: 0,
        };
        game.reset();
        game
    }

    /// Create a new game with non-deterministic seed
    pub fn new(cfg: GameConfig) -> Self {
        Self::with_rng(cfg, ChaCha8Rng::from_os_rng())
    }

    pub fn config(&self) -> &GameConfig {
        &self.cfg
    }

    pub fn status(&self) -> GameStatus {
        self.status
    }

    pub fn score(&self) -> u32 {
        self.score
    }

    pub fn snake_segments(&self) -> impl Iterator<Item = &Point> {
        self.snake.iter()
    }

    pub fn food_positions(&self) -> impl Iterator<Item = &Point> {
        self.food.iter()
    }

    pub fn head(&self) -> Point {
        *self.snake.front().expect("snake is non-empty")
    }

    /// Request a direction change, applied on the next tick if valid.
    /// (Prevents instantaneous 180° reversal.)
    pub fn queue_direction(&mut self, dir: Direction) {
        self.pending_dir = Some(dir);
    }

    /// Resets snake, direction, food, status, and score.
    pub fn reset(&mut self) {
        self.status = GameStatus::Running;
        self.score = 0;
        self.snake.clear();
        self.food.clear();
        self.dir = Direction::Right;
        self.pending_dir = None;

        // Center the snake horizontally, start heading right.
        let cx = self.cfg.width / 2;
        let cy = self.cfg.height / 2;

        let init_len = self.cfg.initial_len.max(1);
        for i in 0..init_len as i32 {
            self.snake
                .push_back(Point::new(cx - i, cy));
        }

        // Spawn one piece of food for now.
        self.spawn_food();
    }

    /// Advance the game by one tick.
    pub fn tick(&mut self) -> TickResult {
        if self.status == GameStatus::Dead {
            return TickResult {
                ate_food: false,
                status: self.status,
                score: self.score,
            };
        }

        // Apply pending direction (if not 180*)
        if let Some(next) = self.pending_dir.take() {
            if !next.is_opposite(self.dir) {
                self.dir = next;
            }
        }

        let next_head = self.next_head_position();

        if !self.cfg.wrap_edges && self.out_of_bounds(next_head) {
            self.status = GameStatus::Dead;
            return TickResult {
                ate_food: false,
                status: self.status,
                score: self.score,
            };
        }

        let next_head = if self.cfg.wrap_edges {
            self.wrap(next_head)
        } else {
            next_head
        };

        // Self collision: allow moving onto the tail if it will move off (unless eating)
        let is_eating = self.food.contains(&next_head);
        let tail_will_move_off = !is_eating;
        if self.collides_with_body(next_head, tail_will_move_off) {
            self.status = GameStatus::Running;
            return TickResult {
                ate_food: false,
                status: self.status,
                score: self.score,
            };
        }

        // Move head
        self.snake.push_front(next_head);

        let ate_food = if is_eating {
            self.food.remove(&next_head);
            self.score += 1;
            self.spawn_food();
            true
        } else {
            self.snake.pop_back();
            false
        };

        TickResult {
            ate_food,
            status: self.status,
            score: self.score,
        }
    }

    fn next_head_position(&self) -> Point {
        let (dx, dy) = self.dir.dx_dy();
        let h = self.head();
        Point::new(h.x + dx, h.y + dy)
    }

    fn out_of_bounds(&self, p: Point) -> bool {
        p.x < 0 || p.x >= self.cfg.width || p.y < 0 || p.y >= self.cfg.height
    }

    fn wrap(&self, p: Point) -> Point {
        let mut x = p.x;
        let mut y = p.y;
        if x < 0 {
            x = self.cfg.width - 1;
        } else if x >= self.cfg.width {
            x = 0;
        }
        if y < 0 {
            y = self.cfg.height - 1;
        } else if y >= self.cfg.height {
            y = 0;
        }
        Point::new(x, y)
    }

    fn collides_with_body(&self, p: Point, tail_will_move_off: bool) -> bool {
        // If tail will move, ignore the last segment during collision check.
        if tail_will_move_off && !self.snake.is_empty() {
            self.snake
                .iter()
                .take(self.snake.len() - 1)
                .any(|&s| s == p)
        } else {
            self.snake.iter().any(|&s| s == p)
        }
    }

    fn spawn_food(&mut self) {
        // Very small grids could be full--avoid inifint loops.
        let max_attempts = (self.cfg.width as usize)
            .saturating_mul(self.cfg.height as usize)
            .saturating_mul(2)
            .max(8);
        let snake_set: HashSet<Point> = self.snake.iter().copied().collect();

        for _ in 0..max_attempts {
            let x = self.rng.random_range(0..self.cfg.width) as Coord;
            let y = self.rng.random_range(0..self.cfg.height) as Coord;
            let p = Point::new(x, y);
            if !snake_set.contains(&p) && !self.food.contains(&p) {
                self.food.insert(p);
                return;
            }
        }
        // If we fail to find a spot, do nothing (grid is effectively full).
    }
}

/// A lightweight "raster" to help the renderer later.
/// Not used by the core tick logic, but makes it trivial to convert to Braille.
#[derive(Debug, Clone)]
pub struct Raster2D {
    pub width: Coord,
    pub height: Coord,
    pub cells: Vec<bool>,
}

impl Raster2D {
    pub fn new(width: Coord, height: Coord) -> Self {
        let size = (width.max(0) * height.max(0)) as usize;
        Self {
            width,
            height,
            cells: vec![false; size],
        }
    }

    #[inline]
    fn idx(&self, x: Coord, y: Coord) -> Option<usize> {
        if x < 0 || y < 0 || x > self.width || y > self.height {
            None
        } else {
            Some((y * self.width + x) as usize)
        }
    }

    pub fn set(&mut self, x: Coord, y: Coord, on: bool) {
        if let Some(i) = self.idx(x, y) {
            self.cells[i] = on;
        }
    }

    pub fn get(&self, x: Coord, y: Coord) -> Option<bool> {
        match self.idx(x, y) {
            Some(idx) => Some(self.cells[idx]),
            None => None,
        }
    }

    /// Print raster in simple ascii
    fn strmap(&self) -> String {
        (0..self.height)
            .map(|y| {
                let to_row = |x| {
                    if let Some(true) = self.get(x, y) {
                        '8'
                    } else {
                        '.'
                    }
                };
                (0..self.width).map(to_row).collect::<String>()
            })
            .collect::<Vec<String>>()
            .join("\n")
    }
}

impl std::fmt::Display for Raster2D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.strmap())
    }
}

pub fn rasterize_game(state: &GameState) -> Raster2D {
    let mut r = Raster2D::new(state.cfg.width, state.cfg.height);
    // Draw snake
    for p in state.snake_segments() {
        r.set(p.x, p.y, true);
    }
    for p in state.food_positions() {
        r.set(p.x, p.y, true);
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_game() -> GameState {
        let cfg = GameConfig {
            width: 10,
            height: 8,
            wrap_edges: false,
            initial_len: 3,
            braille_friendly: true,
        };
        GameState::with_rng(cfg, ChaCha8Rng::seed_from_u64(42))
    }

    #[test]
    fn initial_state_is_running() {
        let g = base_game();
        assert_eq!(g.status(), GameStatus::Running);
        assert!(g.snake_segments().count() >= 1);
        assert!(g.food_positions().count() >= 1);
    }

    #[test]
    fn queue_direction_blocks_180() {
        let mut g = base_game();
        // Start going Right
        g.queue_direction(Direction::Left); // 180° turn; should ignore
        let before = g.dir;
        g.tick();
        assert_eq!(g.status(), GameStatus::Running);
        assert_eq!(g.dir, before);
    }

    #[test]
    fn head_moves() {
        let mut g = base_game();
        let head_1 = g.head().clone();
        g.tick();
        let head_2 = g.head().clone();
        assert_ne!(head_1, head_2);
    }

    #[test]
    fn eating_increases_score_and_length() {
        let mut g = base_game();
        // Place food directly in front of the head.
        let (dx, dy) = g.dir.dx_dy();
        let head = g.head();
        let food_pos = Point::new(head.x + dx, head.y + dy);
        // Clear and insert deterministic food.
        g.food.clear();
        g.food.insert(food_pos);
        let len_before = g.snake_segments().count();
        let res = g.tick();
        assert_eq!(g.head(), food_pos, "Head advanced onto food position");
        assert!(res.ate_food, "Ate food");
        assert_eq!(g.score(), 1, "Score incremented");
        assert_eq!(
            g.snake_segments().count(),
            len_before + 1,
            "Snake length grew"
        );
    }

    #[test]
    fn wall_collision_kills() {
        let mut g = GameState::with_rng(
            GameConfig {
                width: 3,
                height: 3,
                wrap_edges: false,
                initial_len: 1,
                braille_friendly: true,
            },
            ChaCha8Rng::seed_from_u64(1),
        );
        // Put head at right edge, moving right
        g.snake.clear();
        g.snake.push_front(Point::new(2, 1));
        g.dir = Direction::Right;
        let res = g.tick();
        assert_eq!(res.status, GameStatus::Dead);
    }
}
