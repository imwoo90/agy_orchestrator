use std::fs::{self, File};
use std::io::{self, Write};
use std::process::{Command, Stdio};

use crate::backend::vault::get_base_dir;
use crate::backend::daemon::{is_daemon_running, get_daemon_pid, run_daemon_loop};
use crate::backend::cli::CliResult;

pub fn execute(start: bool, stop: bool, status: bool, run: bool) -> io::Result<CliResult> {
    let base_dir = get_base_dir();
    let pid_path = base_dir.join("daemon.pid");

    if run {
        run_daemon_loop()?;
    } else if start {
        if is_daemon_running() {
            let pid = get_daemon_pid().unwrap();
            println!("Daemon is already running with PID: {}", pid);
            return Ok(CliResult::Exit);
        }

        let current_exe = std::env::current_exe()?;
        let mut cmd = Command::new(&current_exe);
        cmd.arg("daemon").arg("--run");

        let daemon_log_path = base_dir.join("daemon.log");
        let daemon_log_file = File::create(&daemon_log_path)?;
        cmd.stdout(Stdio::from(daemon_log_file.try_clone()?));
        cmd.stderr(Stdio::from(daemon_log_file));
        cmd.stdin(Stdio::null());

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            extern "C" {
                fn setsid() -> i32;
            }
            unsafe {
                cmd.pre_exec(|| {
                    setsid();
                    Ok(())
                });
            }
        }

        let child = cmd.spawn()?;
        let pid = child.id();

        let mut file = File::create(&pid_path)?;
        write!(file, "{}", pid)?;

        println!("Orchestrator daemon started in background (PID: {}).", pid);
    } else if stop {
        if !is_daemon_running() {
            println!("Daemon is not running.");
            return Ok(CliResult::Exit);
        }

        let pid = get_daemon_pid().unwrap();
        println!("Stopping daemon (PID: {})...", pid);

        let _ = Command::new("kill").arg(pid.to_string()).status();
        let _ = fs::remove_file(&pid_path);

        println!("Daemon stopped successfully.");
    } else if status {
        if is_daemon_running() {
            let pid = get_daemon_pid().unwrap();
            println!("Daemon is RUNNING (PID: {}).", pid);
        } else {
            println!("Daemon is STOPPED.");
        }
    } else {
        println!("Please specify --start, --stop, or --status.");
    }
    Ok(CliResult::Exit)
}
