mod support;

use std::process::ExitCode;

fn main() -> ExitCode {
    match support::run_event_application("eventservice", "generator") {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("emaneeventservice: {error}");
            ExitCode::FAILURE
        }
    }
}
