#[allow(
    clippy::missing_const_for_fn,
    reason = "the binary entry point should remain an ordinary fn main"
)]
fn main() -> std::process::ExitCode {
    sqlcomp_cli::run()
}
