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

    #[tokio::test]
    async fn test_multi_session_chat() {
        #[cfg(feature = "server")]
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
