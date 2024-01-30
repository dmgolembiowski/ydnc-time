// ydnc-time -- You Don't Need the Cloud to log your time!
// Copyright (C) 2023 Jonathan Ming
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// You can find this project at https://codeberg.org/jming422/ydnc-time
// You can find me at jming422@gmail.com

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    error::Error,
    io,
    sync::{Arc, Mutex},
};
use tracing::info;
use tracing_subscriber::{filter::LevelFilter, prelude::*, EnvFilter};

use ydnc_time::{bluetooth::BluetoothTask, App};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Need to hold on to this guard until the program exits
    let _appender_guard = {
        let file_appender =
            tracing_appender::rolling::hourly(std::env::temp_dir().join("ydnc"), "time.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        let sub = tracing_subscriber::fmt::layer().with_writer(non_blocking);
        tracing_subscriber::registry()
            .with(sub)
            .with(
                EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .from_env_lossy(),
            )
            .init();

        Some(guard)
    };

    info!("ydnc-time starting");

    // modeled after
    // https://github.com/fdehau/tui-rs/blob/master/examples/user_input.rs

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and wrap it so that our bluetooth and UI threads can share it
    // (bluetooth thread will only write to state; UI will both read and write
    // to it)
    let app_state = Arc::new(Mutex::new(App::load_or_default()));

    // start bluetooth handler in "the background" as a tokio task
    let btle_task = BluetoothTask::start(Arc::clone(&app_state));

    // Run the app -- it will return when the user exits the app
    let res = ydnc_time::run(app_state, &mut terminal).await;

    btle_task.stop().await;

    info!("ydnc-time stopped");

    // restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(res?)
}
