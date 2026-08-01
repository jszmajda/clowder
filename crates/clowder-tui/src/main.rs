//! Entry point. Everything lives in the library; this is the shim that starts
//! a session and hands its exit code back to the shell.

fn main() -> std::process::ExitCode {
    clowder_tui::runtime::run()
}
