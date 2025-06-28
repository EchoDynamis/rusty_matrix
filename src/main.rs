use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    style::{Color, Print, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::Rng;
use std::io::{stdout, Write};
use std::time::Duration;

// --- Character Generation ---
static CHARACTERS: once_cell::sync::Lazy<Vec<char>> = once_cell::sync::Lazy::new(|| {
    (0x4E00..=0x9FA5)
        .filter_map(std::char::from_u32)
        .collect()
});

fn get_random_char() -> char {
    let mut rng = rand::thread_rng();
    CHARACTERS[rng.gen_range(0..CHARACTERS.len())]
}

// --- Configuration & State ---
#[derive(Clone, Copy)]
struct ColorScheme {
    head: Color,
    trail: Color,
    fade: Color,
}

const THEMES: [ColorScheme; 4] = [
    ColorScheme { head: Color::White,   trail: Color::Green,      fade: Color::DarkGreen },
    ColorScheme { head: Color::White,   trail: Color::Blue,       fade: Color::DarkBlue },
    ColorScheme { head: Color::White,   trail: Color::Red,        fade: Color::DarkRed },
    ColorScheme { head: Color::Cyan,    trail: Color::Magenta,    fade: Color::DarkMagenta },
];

struct Config {
    theme_index: usize,
    speed_level: usize, // 1-10
}

const SPEED_DURATIONS: [u64; 10] = [100, 88, 76, 64, 52, 40, 33, 28, 24, 20];

enum AppState {
    Matrix,
    Paused,
    Config,
}

// --- Cell & Column Structures ---
#[derive(Clone)]
struct Cell {
    char: char,
    color: Color,
    lifetime: i16,
}

impl Default for Cell {
    fn default() -> Self {
        Self { char: ' ', color: Color::Black, lifetime: 0 }
    }
}

struct Column {
    x: u16,
    cells: Vec<Cell>,
    head: i16,
    len: i16,
    speed: i16,
    counter: i16,
}

impl Column {
    fn new(x: u16, height: u16) -> Self {
        let mut rng = rand::thread_rng();
        Self {
            x,
            cells: vec![Cell::default(); height as usize],
            head: -1,
            len: rng.gen_range(5..=height as i16 / 2),
            speed: rng.gen_range(1..=4),
            counter: 0,
        }
    }

    fn reset(&mut self) {
        let mut rng = rand::thread_rng();
        let height = self.cells.len() as i16;
        self.head = -1;
        self.len = rng.gen_range(5..=height / 2);
        self.speed = rng.gen_range(1..=4);
        self.counter = 0;
    }

    fn update(&mut self, colors: &ColorScheme) {
        self.counter += 1;
        if self.counter < self.speed {
            return;
        }
        self.counter = 0;

        self.head += 1;

        for cell in self.cells.iter_mut() {
            if cell.lifetime > 0 {
                cell.lifetime -= 1;
                if cell.lifetime == 0 {
                    cell.char = ' ';
                }
            }
        }

        for i in 0..self.cells.len() {
            if self.cells[i].lifetime > self.len - 3 {
                self.cells[i].color = colors.trail;
            } else {
                self.cells[i].color = colors.fade;
            }
        }

        if self.head >= 0 && self.head < self.cells.len() as i16 {
            let head_idx = self.head as usize;
            self.cells[head_idx] = Cell {
                char: get_random_char(),
                color: colors.head,
                lifetime: self.len,
            };
        }

        if self.head >= self.cells.len() as i16 + self.len {
            self.reset();
        }
    }

    fn draw(&self, stdout: &mut std::io::Stdout) {
        for (y, cell) in self.cells.iter().enumerate() {
            if cell.lifetime > 0 {
                execute!(
                    stdout,
                    cursor::MoveTo(self.x * 2, y as u16),
                    SetForegroundColor(cell.color),
                    Print(cell.char)
                )
                .unwrap();
            }
        }
    }
}

// --- UI Drawing ---
fn draw_ui(text: &str, stdout: &mut std::io::Stdout) -> std::io::Result<()> {
    execute!(
        stdout,
        Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        SetForegroundColor(Color::White),
        Print(text)
    )?;
    stdout.flush()
}

// --- Main Application ---
fn main() -> std::io::Result<()> {
    let mut stdout = stdout();
    let (width, height) = terminal::size()?;

    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;
    terminal::enable_raw_mode()?;

    let mut columns: Vec<Column> = (0..width / 2).map(|x| Column::new(x, height)).collect();
    let mut app_state = AppState::Matrix;
    let mut config = Config { theme_index: 0, speed_level: 5 };

    loop {
        match app_state {
            AppState::Matrix => {
                if event::poll(Duration::from_millis(SPEED_DURATIONS[config.speed_level - 1]))? {
                    if let Event::Key(key) = event::read()? {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => break,
                            KeyCode::Char('p') => app_state = AppState::Paused,
                            KeyCode::Char('c') => app_state = AppState::Config,
                            _ => {},
                        }
                    }
                }

                execute!(stdout, Clear(ClearType::All))?;
                let colors = &THEMES[config.theme_index];
                for col in columns.iter_mut() {
                    col.update(colors);
                    col.draw(&mut stdout);
                }
                stdout.flush()?;
            }
            AppState::Paused => {
                draw_ui("Paused - Press 'p' to resume or 'q' to quit", &mut stdout)?;
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char('p') => app_state = AppState::Matrix,
                        _ => {},
                    }
                }
            }
            AppState::Config => {
                let theme_name = match config.theme_index {
                    0 => "Classic Green",
                    1 => "Ocean Blue",
                    2 => "Crimson Red",
                    3 => "Cyberpunk",
                    _ => "Unknown",
                };
                let menu_text = format!(
                    "Configuration Menu\n\nSpeed: {} (use +/- to change)\nTheme: {} (use left/right arrows to change)\n\nPress 'c' or 'Esc': Return to matrix",
                    config.speed_level,
                    theme_name
                );
                draw_ui(&menu_text, &mut stdout)?;

                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('c') | KeyCode::Esc => app_state = AppState::Matrix,
                        KeyCode::Char('+') | KeyCode::Char('=') => {
                            config.speed_level = (config.speed_level + 1).min(10);
                        }
                        KeyCode::Char('-') => {
                            config.speed_level = (config.speed_level - 1).max(1);
                        }
                        KeyCode::Right => {
                            config.theme_index = (config.theme_index + 1) % THEMES.len();
                        }
                        KeyCode::Left => {
                            config.theme_index = if config.theme_index == 0 {
                                THEMES.len() - 1
                            } else {
                                config.theme_index - 1
                            };
                        }
                        _ => {},
                    }
                }
            }
        }
    }

    // Cleanup
    execute!(stdout, cursor::Show, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(())
}