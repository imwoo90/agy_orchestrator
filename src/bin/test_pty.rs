#[cfg(feature = "server")]
use std::process::Command;
#[cfg(feature = "server")]
use std::time::{Duration, Instant};
#[cfg(feature = "server")]
use std::os::fd::AsRawFd;

#[cfg(feature = "server")]
#[repr(C)]
struct Winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

#[cfg(feature = "server")]
extern "C" {
    fn ioctl(fd: std::os::raw::c_int, request: std::os::raw::c_ulong, ...) -> std::os::raw::c_int;
}

#[cfg(feature = "server")]
fn main() {
    let mut cmd = Command::new("/home/wimvm/.local/bin/agy");
    cmd.arg("--conversation");
    cmd.arg("test-session-12345");
    cmd.env("TERM", "xterm-256color");

    println!("Spawning agy via rexpect...");
    match rexpect::session::spawn_command(cmd, Some(10000)) {
        Ok(mut session) => {
            println!("Spawned successfully. Setting window size to 80x24...");
            
            #[allow(dead_code)]
            struct PtyProcessShadow {
                pty: std::os::fd::OwnedFd,
                child_pid: i32,
                kill_timeout: Option<std::time::Duration>,
            }
            
            let pty_fd = unsafe {
                let shadow: &PtyProcessShadow = std::mem::transmute(session.process());
                shadow.pty.as_raw_fd()
            };
            
            let w = Winsize {
                ws_row: 24,
                ws_col: 80,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            
            unsafe {
                ioctl(pty_fd, 0x5414, &w);
            }
            println!("Window size set. Starting match loop...");
            
            const REPL_PROMPT_PATTERN: &str = r"(\x1b\[94m>\x1b\[m|(?:\r\n|\n)>\s*)";
            const CAP_QUERY_2026: &str = r"\x1b\[\?2026\$p";
            const CAP_QUERY_2027: &str = r"\x1b\[\?2027\$p";
            const CAP_QUERY_KITTY: &str = r"\x1b\[\?u";
            
            let combined_pattern = format!(
                "({})|({})|({})|({})",
                REPL_PROMPT_PATTERN,
                CAP_QUERY_2026,
                CAP_QUERY_2027,
                CAP_QUERY_KITTY
            );
            
            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(15) {
                match session.exp_regex(&combined_pattern) {
                    Ok((_, matched)) => {
                        println!("[PTY Output] Matched: {:?}", matched);
                        if matched.contains("\x1b[?2026$p") {
                            println!("[PTY Mock] Got 2026 query, sending response...");
                            let _ = session.send("\x1b[?2026;0$y");
                        } else if matched.contains("\x1b[?2027$p") {
                            println!("[PTY Mock] Got 2027 query, sending response...");
                            let _ = session.send("\x1b[?2027;0$y");
                        } else if matched.contains("\x1b[?u") {
                            println!("[PTY Mock] Got Kitty keyboard query, sending response...");
                            let _ = session.send("\x1b[?0u");
                        } else {
                            // Must be the REPL prompt or permission prompt!
                            let is_repl_prompt = matched.contains('>') && (matched.contains('\n') || matched.contains("\x1b[94m"));
                            if is_repl_prompt {
                                println!("[Success] Reached REPL prompt!");
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        println!("Match error: {:?}", e);
                        break;
                    }
                }
            }
            
            println!("Child status: {:?}", session.process().status());
        }
        Err(e) => {
            println!("Failed to spawn: {:?}", e);
        }
    }
}

#[cfg(not(feature = "server"))]
fn main() {
    println!("Server feature not enabled; test_pty is a no-op.");
}
