use tracing::error;

use crate::app_state::AppState;
use crate::menu_entry::{MenuEntry, parse_line};

pub fn start_async_stdin_reader(app_state: AppState) {
    let (sender, receiver) = async_channel::unbounded::<String>();

    glib::spawn_future_local(async move {
        use smol::io::{AsyncBufReadExt, BufReader};

        let stdin = smol::Async::new(std::io::stdin()).expect("Failed to create async stdin");
        let mut reader = BufReader::new(stdin);
        let mut buffer = String::new();

        loop {
            buffer.clear();
            match reader.read_line(&mut buffer).await {
                Ok(0) => break,
                Ok(_) => {
                    let line = buffer.trim().to_string();
                    if !line.is_empty() && sender.send(line).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    error!("Error reading from stdin: {}", e);
                    break;
                }
            }
        }
    });

    glib::spawn_future_local(async move {
        while let Ok(line) = receiver.recv().await {
            if let Some((label, icon, value)) = parse_line(&line) {
                let entry = MenuEntry::new(label, icon, value);
                app_state.add_entry(entry);
            }
        }
    });
}
