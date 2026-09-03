#[cfg(windows)]
fn main() {
    let exit_code = if std::env::args_os().nth(1).is_none() {
        ldtg_lib::run_firewall_cleanup_helper()
    } else {
        2
    };
    std::process::exit(exit_code);
}

#[cfg(not(windows))]
fn main() {
    std::process::exit(1);
}
