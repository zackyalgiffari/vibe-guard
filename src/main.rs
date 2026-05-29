use std::process::ExitCode;

fn main() -> ExitCode {
    match vibe_guard::cli::run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
