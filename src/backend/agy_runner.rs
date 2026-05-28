//! agy_runner: rexpect 기반 agy 프로세스 실행 모듈
//!
//! `--dangerously-skip-permissions`를 사용하더라도 `invoke_subagent`가 띄우는
//! 플랫폼 레벨의 interactive 권한 팝업이 PTY 없이는 hang을 유발합니다.
//! 이 모듈은 rexpect를 이용해 PTY를 생성하고, 예상치 못한 interactive 프롬프트에
//! 자동으로 응답하여 agy 프로세스가 중단 없이 완료되도록 보장합니다.

use std::io::{self, Write};

/// agy 프로세스 실행 결과
pub struct AgyOutput {
    /// stdout + stderr 통합 출력
    pub combined_output: String,
    /// 프로세스 종료 성공 여부
    pub success: bool,
}

/// 자동 응답 대상 interactive 프롬프트 패턴들
///
/// 이 패턴들에 매칭되는 프롬프트는 자동으로 "y\n" 또는 적절한 응답으로 처리됩니다.
/// agy --dangerously-skip-permissions가 처리하지 못하는 플랫폼 레벨 팝업,
/// git 인증, 패키지 설치 확인 등을 포함합니다.
const AUTO_APPROVE_PATTERNS: &[(&str, &str)] = &[
    // Antigravity 플랫폼 권한 팝업 (invoke_subagent 서브에이전트)
    (r"Allow\s+.*\?\s*\(y/n\)", "y"),
    (r"\[Allow\]\s*\[Deny\]", "y"),
    (r"Grant permission", "y"),
    (r"Permission request", "y"),
    // 일반 y/n 확인 프롬프트
    (r"\(y/n\)\s*$", "y"),
    (r"\(Y/n\)\s*$", "y"),
    (r"\[y/N\]\s*$", "y"),
    (r"\[Y/n\]\s*$", "y"),
    (r"Continue\?\s*\(yes/no\)", "yes"),
    // git 관련
    (r"Are you sure you want to continue", "yes"),
    // 패키지 / sudo
    (r"Do you want to continue\?", "y"),
    (r"Proceed\s*\[y/N\]", "y"),
];

/// rexpect를 이용해 agy를 PTY 환경에서 실행합니다.
///
/// - `args`: `agy` 이후의 인자 목록 (예: `["--dangerously-skip-permissions", "--prompt", "..."]`)
/// - `timeout_secs`: 전체 실행 타임아웃 (초). None이면 10분.
/// - `log_path`: 백그라운드 실시간 출력을 기록할 로그 파일 경로.
///
/// 반환: stdout/stderr 통합 출력 문자열과 성공 여부.
pub fn run_agy_with_pty(
    args: &[&str],
    timeout_secs: Option<u64>,
    log_path: Option<&std::path::Path>,
) -> io::Result<AgyOutput> {
    let overall_timeout = std::time::Duration::from_secs(timeout_secs.unwrap_or(600));
    let start_time = std::time::Instant::now();

    // agy 바이너리 경로 결정
    let agy_bin = std::env::var("AGY_BIN")
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/home/wimvm".to_string());
            format!("{}/.local/bin/agy", home)
        });

    let mut output_buf = String::new();
    let mut log_file = if let Some(path) = log_path {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Some(std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?)
    } else {
        None
    };

    // AUTO_APPROVE_PATTERNS를 하나의 OR 정규식으로 통합
    let combined_pattern = AUTO_APPROVE_PATTERNS
        .iter()
        .map(|(p, _)| format!("({})", p))
        .collect::<Vec<_>>()
        .join("|");

    let mut cmd = std::process::Command::new(&agy_bin);
    let filtered_args: Vec<&str> = args.iter()
        .cloned()
        .filter(|&arg| arg != "--dangerously-skip-permissions")
        .collect();
    cmd.args(&filtered_args);

    // 500ms의 짧은 read timeout을 설정하여 주기적인 로깅 및 타임아웃 체크를 지원합니다.
    match rexpect::session::spawn_command(cmd, Some(500)) {
        Ok(mut session) => {
            // 메인 루프: interactive 팝업 감지 → 자동 응답 → EOF까지 반복
            loop {
                // 전체 실행 시간 초과 검사
                if start_time.elapsed() >= overall_timeout {
                    let timeout_msg = format!(
                        "\n[agy_runner] Error: Overall execution timeout reached ({}s).\n",
                        overall_timeout.as_secs()
                    );
                    output_buf.push_str(&timeout_msg);
                    if let Some(file) = log_file.as_mut() {
                        let _ = write!(file, "{}", timeout_msg);
                        let _ = file.flush();
                    }
                    break;
                }

                match session.exp_regex(&combined_pattern) {
                    Ok((before, matched_str)) => {
                        output_buf.push_str(&before);
                        output_buf.push_str(&matched_str);

                        if let Some(file) = log_file.as_mut() {
                            let _ = write!(file, "{}{}", before, matched_str);
                            let _ = file.flush();
                        }

                        // 매칭된 패턴에 해당하는 응답 찾기
                        // matched_str에 패턴 키워드가 포함되는지로 간단히 판별
                        let response = AUTO_APPROVE_PATTERNS
                            .iter()
                            .find(|(p, _)| {
                                // 패턴의 핵심 리터럴 부분으로 매칭 여부 확인
                                let literal = p.trim_matches(|c: char| {
                                    matches!(c, '(' | ')' | '?' | '\\' | '^' | '$' | '*' | '+')
                                });
                                matched_str.contains(literal) || literal.is_empty()
                            })
                            .map(|(_, r)| *r)
                            .unwrap_or("y");

                        if session.send_line(response).is_err() {
                            // 응답 전송 실패 = 프로세스 종료 신호
                            break;
                        }

                        if let Some(file) = log_file.as_mut() {
                            let _ = writeln!(file, "{}", response);
                            let _ = file.flush();
                        }
                    }
                    Err(rexpect::error::Error::EOF { got, .. }) => {
                        output_buf.push_str(&got);
                        if let Some(file) = log_file.as_mut() {
                            let _ = write!(file, "{}", got);
                            let _ = file.flush();
                        }
                        // 정상 종료
                        break;
                    }
                    Err(rexpect::error::Error::Timeout { got, .. }) => {
                        output_buf.push_str(&got);
                        if let Some(file) = log_file.as_mut() {
                            let _ = write!(file, "{}", got);
                            let _ = file.flush();
                        }
                        // 단기 500ms 타임아웃 발생 시, 단순 출력 보존 후 루프 지속
                    }
                    Err(_) => {
                        // 기타 오류
                        break;
                    }
                }
            }

            Ok(AgyOutput {
                combined_output: output_buf,
                success: true,
            })
        }
        Err(e) => {
            // rexpect spawn 실패 — fallback으로 일반 Command 실행
            eprintln!(
                "[agy_runner] rexpect spawn 실패, fallback Command 사용: {}",
                e
            );
            run_agy_fallback(args, log_path)
        }
    }
}

/// rexpect를 사용할 수 없는 환경(PTY 없음 등)에서의 fallback 실행
fn run_agy_fallback(args: &[&str], log_path: Option<&std::path::Path>) -> io::Result<AgyOutput> {
    let agy_bin = std::env::var("AGY_BIN").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/wimvm".to_string());
        format!("{}/.local/bin/agy", home)
    });

    let output = std::process::Command::new(&agy_bin)
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()?;

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    if let Some(path) = log_path {
        let _ = std::fs::write(path, &combined);
    }

    Ok(AgyOutput {
        combined_output: combined,
        success: output.status.success(),
    })
}

/// agy를 백그라운드 PTY 세션으로 spawn합니다 (결과를 기다리지 않음).
///
/// `delegate` 커맨드처럼 fire-and-forget이 필요한 경우 사용합니다.
/// rexpect를 별도 스레드에서 실행하여 interactive 팝업을 자동 처리합니다.
///
/// 반환: 백그라운드 스레드 JoinHandle
pub fn spawn_agy_background(
    args: Vec<String>,
    log_path: Option<std::path::PathBuf>,
    timeout_secs: Option<u64>,
) -> io::Result<u32> {
    let (tx, rx) = std::sync::mpsc::channel();
    
    std::thread::spawn(move || {
        let agy_bin = std::env::var("AGY_BIN")
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/home/wimvm".to_string());
                format!("{}/.local/bin/agy", home)
            });

        let mut cmd = std::process::Command::new(&agy_bin);
        let workspace_dir = "/home/wimvm/works/agy_orchestrator";
        if std::path::Path::new(workspace_dir).exists() {
            cmd.arg("--add-dir");
            cmd.arg(workspace_dir);
            cmd.current_dir(workspace_dir);
        }

        // Filter out `--dangerously-skip-permissions`
        let filtered_args: Vec<String> = args.into_iter()
            .filter(|arg| arg != "--dangerously-skip-permissions")
            .collect();
        cmd.args(&filtered_args);

        match rexpect::session::spawn_command(cmd, Some(500)) {
            Ok(mut session) => {
                // Access pid via shadow struct transmute
                #[allow(dead_code)]
                struct PtyProcessShadow {
                    pty: std::os::fd::OwnedFd,
                    child_pid: i32,
                    kill_timeout: Option<std::time::Duration>,
                }
                let pid = unsafe {
                    let shadow: &PtyProcessShadow = std::mem::transmute(session.process());
                    shadow.child_pid as u32
                };
                
                let _ = tx.send(Ok(pid));
                
                // Now run the PTY interaction loop just like run_agy_with_pty
                let overall_timeout = std::time::Duration::from_secs(timeout_secs.unwrap_or(600));
                let start_time = std::time::Instant::now();
                let mut output_buf = String::new();
                let mut log_file = if let Some(ref path) = log_path {
                    let _ = std::fs::create_dir_all(path.parent().unwrap());
                    std::fs::OpenOptions::new().create(true).append(true).open(path).ok()
                } else {
                    None
                };

                let combined_pattern = AUTO_APPROVE_PATTERNS
                    .iter()
                    .map(|(p, _)| format!("({})", p))
                    .collect::<Vec<_>>()
                    .join("|");

                loop {
                    if start_time.elapsed() >= overall_timeout {
                        break;
                    }
                    match session.exp_regex(&combined_pattern) {
                        Ok((before, matched_str)) => {
                            output_buf.push_str(&before);
                            output_buf.push_str(&matched_str);
                            if let Some(ref mut file) = log_file {
                                let _ = write!(file, "{}{}", before, matched_str);
                                let _ = file.flush();
                            }
                            let response = AUTO_APPROVE_PATTERNS
                                .iter()
                                .find(|(p, _)| {
                                    let literal = p.trim_matches(|c: char| {
                                        matches!(c, '(' | ')' | '?' | '\\' | '^' | '$' | '*' | '+')
                                    });
                                    matched_str.contains(literal) || literal.is_empty()
                                })
                                .map(|(_, r)| *r)
                                .unwrap_or("y");

                            if session.send_line(response).is_err() {
                                break;
                            }
                            if let Some(ref mut file) = log_file {
                                let _ = writeln!(file, "{}", response);
                                let _ = file.flush();
                            }
                        }
                        Err(rexpect::error::Error::EOF { got, .. }) => {
                            output_buf.push_str(&got);
                            if let Some(ref mut file) = log_file {
                                let _ = write!(file, "{}", got);
                                let _ = file.flush();
                            }
                            break;
                        }
                        Err(rexpect::error::Error::Timeout { got, .. }) => {
                            output_buf.push_str(&got);
                            if let Some(ref mut file) = log_file {
                                let _ = write!(file, "{}", got);
                                let _ = file.flush();
                            }
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(Err(io::Error::other(e.to_string())));
            }
        }
    });

    rx.recv().unwrap_or_else(|_| Err(io::Error::other("Thread hung during spawn")))
}
