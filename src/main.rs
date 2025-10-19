use std::io;
use std::time::{Duration, Instant};

use hjkl_snake::render::render_braille;
use hjkl_snake::{Direction, GameConfig, GameState, rasterize_game};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::Alignment,
    style::Stylize,
    widgets::{Block, Borders, Paragraph, Wrap},
};

fn main() -> io::Result<()> {
    // --- Init terminal ---
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let res = run(&mut terminal);

    // --- Restore terminal even on error ---
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = res {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
    Ok(())
}

const INIT_TICK_MILLIS: u64 = 70;

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    // --- Game setup ---
    let cfg = GameConfig {
        width: 100,  // grid cells (not characters)
        height: 40, // choose even/4-friendly for Braille density
        wrap_edges: true,
        initial_len: 6,
        braille_friendly: true,
    };
    let mut game = GameState::new(cfg);

    // Timing
    let mut tick_rate = Duration::from_millis(INIT_TICK_MILLIS); 
    let mut last_tick = Instant::now();

    // UI state
    let mut running = true;

    while running {
        // --- Input (non-blocking) ---
        let now = Instant::now();
        let timeout = tick_rate
            .checked_sub(now.saturating_duration_since(last_tick))
            .unwrap_or(Duration::from_millis(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if handle_key(&mut game, key) {
                        running = false; // requested quit
                    }
                }
            }
        }

        // --- Tick ---
        if last_tick.elapsed() >= tick_rate {
            game.tick();
            last_tick = Instant::now();
        }

        tick_rate = Duration::from_millis(std::cmp::max(INIT_TICK_MILLIS - game.score() as u64, 1));

        // --- Render ---
        terminal.draw(|f| {
            let area = f.area();

            // Compose title/status
            let status = match game.status() {
                hjkl_snake::GameStatus::Running => "󱔎  ",
                hjkl_snake::GameStatus::Dead =>    "    (press q to quit) ",
            };
            let title = format!(
                " hjkl Snake — score: {}  •  {}",
                game.score(),
                status,
            );

            // Convert to Braille string (each line is Braille cells)
            let braille = render_braille(&rasterize_game(&game));

            let block = Block::default().borders(Borders::ALL).title(title.bold());

            // --- Fit frame to game board (Braille dims) ---
            // Braille packs 2x4 cells per char. We compute the exact size.
            let gw = game.config().width;
            let gh = game.config().height;
            let braille_cols = (gw + 1) / 2; // ceil(width/2)
            let braille_rows = (gh + 3) / 4; // ceil(height/4)

            // Paragraph area should be exactly content size; Block adds a 1-char border around it.
            let outer_w = braille_cols.saturating_add(2) as u16;
            let outer_h = braille_rows.saturating_add(2) as u16;

            // Center the frame within the terminal area.
            let x = area.x.saturating_add(area.width.saturating_sub(outer_w) / 2);
            let y = area.y.saturating_add(area.height.saturating_sub(outer_h) / 2);
            let frame_area = ratatui::layout::Rect::new(x, y, outer_w, outer_h);

            let para = Paragraph::new(braille)
                .block(block)
                .alignment(Alignment::Left)
                // Avoid wrapping; Braille lines should display as provided
                .wrap(Wrap { trim: false });

            f.render_widget(para, frame_area);
        })?;
    }

    Ok(())
}

/// Returns true if the caller should quit.
fn handle_key(game: &mut GameState, key: KeyEvent) -> bool {
    match key.code {
        // Quit keys
        KeyCode::Char('q') => return true,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return true,

        // Vim movement (k/j/h/l) → Up/Down/Left/Right
        KeyCode::Char('k') => game.queue_direction(Direction::Up),
        KeyCode::Char('j') => game.queue_direction(Direction::Down),
        KeyCode::Char('h') => game.queue_direction(Direction::Left),
        KeyCode::Char('l') => game.queue_direction(Direction::Right),

        // Nice-to-have: also support arrow keys
        KeyCode::Up => game.queue_direction(Direction::Up),
        KeyCode::Down => game.queue_direction(Direction::Down),
        KeyCode::Left => game.queue_direction(Direction::Left),
        KeyCode::Right => game.queue_direction(Direction::Right),

        // Reset after death
        KeyCode::Char('r') => {
            if game.status() == hjkl_snake::GameStatus::Dead {
                game.reset();
            }
        }

        _ => {}
    }
    false
}
