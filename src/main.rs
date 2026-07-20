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

use anyhow::{Result, bail};
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
    let restore_result = restore_terminal(&mut terminal);

    result?;
    restore_result
}

fn run(terminal: &mut Tui, config: Config) -> Result<()> {
    let interval = Duration::from_millis(config.interval_ms);
    let (sender, receiver) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let collector = spawn_collector(sender, Arc::clone(&stop), interval);
    let mut app = App::new(config.history_points);

    let loop_result = (|| -> Result<()> {
        loop {
            for snapshot in receiver.try_iter() {
                app.apply_snapshot(snapshot);
            }

            terminal.draw(|frame| ui::draw(frame, &mut app))?;

            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key) if key.kind != KeyEventKind::Release => {
                        if app.handle_key(key) {
                            break;
                        }
                    }
                    Event::Mouse(mouse) => app.handle_mouse(mouse),
                    _ => {}
                }
            }
        }
        Ok(())
    })();

    stop.store(true, Ordering::Relaxed);
    let collector_result = collector.join();

    loop_result?;
    if collector_result.is_err() {
        bail!("metric collector thread panicked");
    }
    Ok(())
}

fn init_terminal() -> Result<Tui> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    if let Err(error) = execute!(stdout, EnterAlternateScreen, EnableMouseCapture) {
        let _ = disable_raw_mode();
        return Err(error.into());
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(terminal) => terminal,
        Err(error) => {
            let _ = disable_raw_mode();
            let mut stdout = io::stdout();
            let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture, Show);
            return Err(error.into());
        }
    };

    if let Err(error) = terminal.clear() {
        let _ = restore_terminal(&mut terminal);
        return Err(error.into());
    }

    Ok(terminal)
}

fn restore_terminal(terminal: &mut Tui) -> Result<()> {
    let raw_mode_result = disable_raw_mode();
    let screen_result = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        Show
    );
    let cursor_result = terminal.show_cursor();

    raw_mode_result?;
    screen_result?;
    cursor_result?;
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
