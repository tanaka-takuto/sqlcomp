use std::process::Command;

#[test]
fn sqlcomp_binary_exits_successfully() {
    let status = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .status()
        .expect("sqlcomp binary should run");

    assert!(status.success());
}
