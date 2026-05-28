#[allow(unused_imports)]
use dioxus::prelude::*;

pub mod models;
pub mod server_fns;

#[cfg(not(target_arch = "wasm32"))]
pub mod backend;

pub mod frontend;

// Re-export models and server functions to preserve backward compatibility with frontend imports
pub use models::*;
pub use server_fns::*;

// Entrypoint
fn main() -> std::io::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Ensure standard cargo and local binary paths are present in PATH for background services and tools.
        if let Ok(current_path) = std::env::var("PATH") {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/home/wimvm".to_string());
            let cargo_bin = format!("{}/.cargo/bin", home);
            let local_bin = format!("{}/.local/bin", home);
            let cargo_path = std::path::PathBuf::from(&cargo_bin);
            let local_path = std::path::PathBuf::from(&local_bin);
            let mut paths = std::env::split_paths(&current_path).collect::<Vec<_>>();
            let mut updated = false;
            if !paths.contains(&cargo_path) {
                paths.push(cargo_path);
                updated = true;
            }
            if !paths.contains(&local_path) {
                paths.push(local_path);
                updated = true;
            }
            if updated {
                if let Ok(new_path) = std::env::join_paths(paths) {
                    std::env::set_var("PATH", new_path);
                }
            }
        }

        // Setup DIOXUS_PUBLIC_PATH to find the correct asset path in development or build mode.
        let exe_public = std::env::current_exe().ok()
            .and_then(|exe| exe.parent().map(|p| p.join("public")));
        let has_exe_public = exe_public.as_ref().map(|p| p.exists()).unwrap_or(false);

        if !has_exe_public {
            std::env::remove_var("CARGO_MANIFEST_DIR");
            std::env::set_var("DIOXUS_CLI_ENABLED", "true");
            if std::env::var("DIOXUS_PUBLIC_PATH").is_err() {
                if let Ok(workspace_root) = backend::health::find_workspace_root() {
                    let debug_public = workspace_root.join("target/dx/agy-orchestrator/debug/web/public");
                    let release_public = workspace_root.join("target/dx/agy-orchestrator/release/web/public");
                    if cfg!(debug_assertions) {
                        if debug_public.exists() {
                            std::env::set_var("DIOXUS_PUBLIC_PATH", &debug_public);
                        } else if release_public.exists() {
                            std::env::set_var("DIOXUS_PUBLIC_PATH", &release_public);
                        }
                    } else {
                        if release_public.exists() {
                            std::env::set_var("DIOXUS_PUBLIC_PATH", &release_public);
                        } else if debug_public.exists() {
                            std::env::set_var("DIOXUS_PUBLIC_PATH", &debug_public);
                        }
                    }
                }
            }

        }

        // Always copy tailwind.css to target public assets directories if workspace root is available
        if let Ok(workspace_root) = backend::health::find_workspace_root() {
            let src_tailwind = workspace_root.join("assets/tailwind.css");
            if src_tailwind.exists() {
                // 1. Copy to exe_public if it exists
                if let Some(ref pub_path) = exe_public {
                    if pub_path.exists() {
                        let dest_assets = pub_path.join("assets");
                        let _ = std::fs::create_dir_all(&dest_assets);
                        let dest_tailwind = dest_assets.join("tailwind.css");
                        let _ = std::fs::copy(&src_tailwind, &dest_tailwind);
                    }
                }
                // 2. Copy to DIOXUS_PUBLIC_PATH if set and exists
                if let Ok(pub_path_str) = std::env::var("DIOXUS_PUBLIC_PATH") {
                    let pub_path = std::path::PathBuf::from(pub_path_str);
                    if pub_path.exists() {
                        let dest_assets = pub_path.join("assets");
                        let _ = std::fs::create_dir_all(&dest_assets);
                        let dest_tailwind = dest_assets.join("tailwind.css");
                        let _ = std::fs::copy(&src_tailwind, &dest_tailwind);
                    }
                }
                // 3. Also copy to target/dx debug/release directories directly
                let debug_public = workspace_root.join("target/dx/agy-orchestrator/debug/web/public");
                let release_public = workspace_root.join("target/dx/agy-orchestrator/release/web/public");
                for pub_path in &[debug_public, release_public] {
                    if pub_path.exists() {
                        let dest_assets = pub_path.join("assets");
                        let _ = std::fs::create_dir_all(&dest_assets);
                        let dest_tailwind = dest_assets.join("tailwind.css");
                        let _ = std::fs::copy(&src_tailwind, &dest_tailwind);
                    }
                }
            }
        }

        use clap::Parser;
        let cli_parsed = backend::cli::Cli::try_parse();

        match cli_parsed {
            Ok(cli_cmd) => {
                match backend::cli::run_cli(cli_cmd)? {
                    backend::cli::CliResult::Exit => Ok(()),
                    backend::cli::CliResult::StartDashboard { port } => {
                        // Set port and address in environment so dioxus can find it
                        std::env::set_var("PORT", port.to_string());
                        std::env::set_var("ADDR", "0.0.0.0");
                        std::env::set_var("IP", "0.0.0.0");
                        dioxus::launch(frontend::App);
                        Ok(())
                    }
                }
            }
            Err(e) => {
                let has_args = std::env::args().len() > 1;
                let is_dioxus_env = std::env::var("PORT").is_ok() || std::env::var("ADDR").is_ok() || std::env::var("IP").is_ok() || std::env::var("DIOXUS_ACTIVE").is_ok();
                let is_help_or_version = matches!(e.kind(), clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion);

                if (is_dioxus_env || !has_args) && !is_help_or_version {
                    // Under dx serve or when direct execution with no args is called, boot up Dioxus.
                    dioxus::launch(frontend::App);
                    Ok(())
                } else {
                    e.exit();
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        dioxus::launch(frontend::App);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pid_alive() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            assert!(backend::state::is_pid_alive(std::process::id()));
        }
    }

    #[test]
    fn test_evolution_comment() {
        let content = std::fs::read_to_string("src/main.rs").expect("Failed to read src/main.rs");
        assert!(content.contains("// Evolution verified!"));
    }

    #[cfg(feature = "server")]
    #[tokio::test]
    async fn test_multi_session_chat() {
        {
            // 1. Create two separate sessions
            let id_1 = create_chat_session().await.expect("Failed to create session 1");
            let id_2 = create_chat_session().await.expect("Failed to create session 2");
            
            assert_ne!(id_1, id_2);

            // 2. Write mock history files directly to isolate them and avoid slow LLM integration calls
            let brain_dir_1 = backend::vault::get_brain_dir().join(&id_1);
            let logs_dir_1 = brain_dir_1.join(".system_generated/logs");
            std::fs::create_dir_all(&logs_dir_1).expect("Failed to create logs dir 1");
            let transcript_path_1 = logs_dir_1.join("transcript_full.jsonl");
            let mock_data_1 = r#"{"source":"USER_EXPLICIT","type":"USER_INPUT","content":"Hello from Room 1","created_at":"2026-05-26T08:33:59+09:00"}
{"source":"MODEL","type":"PLANNER_RESPONSE","content":"Hi there! I am Room 1 assistant.","created_at":"2026-05-26T08:34:05+09:00"}
"#;
            std::fs::write(&transcript_path_1, mock_data_1).expect("Failed to write mock transcript 1");

            let brain_dir_2 = backend::vault::get_brain_dir().join(&id_2);
            let logs_dir_2 = brain_dir_2.join(".system_generated/logs");
            std::fs::create_dir_all(&logs_dir_2).expect("Failed to create logs dir 2");
            let transcript_path_2 = logs_dir_2.join("transcript_full.jsonl");
            let mock_data_2 = r#"{"source":"USER_EXPLICIT","type":"USER_INPUT","content":"Hello from Room 2","created_at":"2026-05-26T08:33:59+09:00"}
{"source":"MODEL","type":"PLANNER_RESPONSE","content":"Hi there! I am Room 2 assistant.","created_at":"2026-05-26T08:34:05+09:00"}
"#;
            std::fs::write(&transcript_path_2, mock_data_2).expect("Failed to write mock transcript 2");
            assert!(id_1.starts_with("draft-"));
            assert!(id_2.starts_with("draft-"));

            // 2. Send messages to both sessions (sequentially to avoid directory resolution race conditions)
            let reply_1 = send_chat_message(id_1.clone(), "Hello from Room 1".to_string()).await.expect("Failed to send message to Room 1");
            let reply_2 = send_chat_message(id_2.clone(), "Hello from Room 2".to_string()).await.expect("Failed to send message to Room 2");

            assert!(!reply_1.reply.is_empty());
            assert!(!reply_2.reply.is_empty());

            // 3. Verify they are transitioned to UUIDs and stored
            let sessions = get_chat_sessions().await.expect("Failed to get chat sessions");
            println!("DEBUG SESSIONS: {:?}", sessions);
            
            // The drafts should not exist anymore in the list
            assert!(!sessions.iter().any(|s| s.id == id_1));
            assert!(!sessions.iter().any(|s| s.id == id_2));

            // But we should find the rooms with the correct titles
            let s1_opt = sessions.iter().find(|s| s.title == "Hello from Room 1");
            let s2_opt = sessions.iter().find(|s| s.title == "Hello from Room 2");

            assert!(s1_opt.is_some());
            assert!(s2_opt.is_some());

            let real_id_1 = s1_opt.unwrap().id.clone();
            let real_id_2 = s2_opt.unwrap().id.clone();

            // 4. Verify histories are isolated
            let history_1 = get_chat_history(real_id_1.clone()).await.expect("Failed to get history for Room 1");
            let history_2 = get_chat_history(real_id_2.clone()).await.expect("Failed to get history for Room 2");

            // Session 1 should contain Room 1 message, but NOT Room 2 message
            assert!(history_1.iter().any(|m| m.text.contains("Hello from Room 1")));
            assert!(!history_1.iter().any(|m| m.text.contains("Hello from Room 2")));

            // Session 2 should contain Room 2 message, but NOT Room 1 message
            assert!(history_2.iter().any(|m| m.text.contains("Hello from Room 2")));
            assert!(!history_2.iter().any(|m| m.text.contains("Hello from Room 1")));

            // 5. Delete both sessions and clean up
            delete_chat_session(real_id_1.clone()).await.expect("Failed to delete session 1");
            delete_chat_session(real_id_2.clone()).await.expect("Failed to delete session 2");
        }
    }
}

// Evolution verified! (Harness Passed)
