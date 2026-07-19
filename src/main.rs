mod app;
mod collector;
mod config;
mod model;
mod ui;
mod utils;

use std::{
    io::{self, Stdout},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

use anyhow::Result;
use crossterm::{
    cursor::Show,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{
    app::App,
    collector::spawn_collector,
    config::{Config, help_text},
};

type Tui = Terminal<CrosstermBackend<Stdout>>;

fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print!("{}", help_text());
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("btop-win {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let config = Config::from_env()?;
    install_panic_hook();
    let mut terminal = init_terminal()?;
    let result = run(&mut terminal, config);
    restore_terminal(&mut terminal)?;
    result
}

fn run(terminal: &mut Tui, config: Config) -> Result<()> {
    let interval = Duration::from_millis(config.interval_ms);
    let (sender, receiver) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let collector = spawn_collector(sender, Arc::clone(&stop), interval);
    let mut app = App::new(config.history_points);
    let mut should_quit = false;

    while !should_quit {
        for snapshot in receiver.try_iter() {
            app.apply_snapshot(snapshot);
        }

        terminal.draw(|frame| ui::draw(frame, &mut app))?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    should_quit = app.handle_key(key);
                }
                Event::Mouse(mouse) => app.handle_mouse(mouse),
                _ => {}
            }
        }
    }

    stop.store(true, Ordering::Relaxed);
    let _ = collector.join();
    Ok(())
}

fn init_terminal() -> Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Tui) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture, Show)?;
    terminal.show_cursor()?;
    Ok(())
}

fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture, Show);
        previous(panic_info);
    }));
}
