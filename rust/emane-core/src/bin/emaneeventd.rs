mod support;

use std::process::ExitCode;

fn main() -> ExitCode {
    match support::run_event_application("eventdaemon", "agent") {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("emaneeventd: {error}");
            ExitCode::FAILURE
        }
    }
}
