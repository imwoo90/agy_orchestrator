# Refactoring Plan: Modularizing `execute_delegate` in `src/backend/commands/utils.rs`

## 1. Executive Summary
This document provides a detailed plan to refactor the `execute_delegate` function in `/home/wimvm/works/agy_orchestrator/src/backend/commands/utils.rs`.
Currently, `execute_delegate` spans over 190 lines of code, mixing validation, multiple disk reads (JIT injections for playbooks, context, and skills), system CLI parameter formatting, PTY background execution, and state persistence.

Refactoring this function into smaller, well-defined helper functions will:
- **Improve Readability:** Keep the main command handler brief, self-documenting, and focused on control flow.
- **Enhance Testability:** Allow isolated unit-testing of key features (such as skill extraction matching logic).
- **Reduce Maintenance Overhead:** Enable safe modifications to individual steps without risking side effects in others.
- **Maintain Full Compatibility:** Preserve all existing behaviors, terminal outputs, error exits, and file/folder structures.

---

## 2. Current Implementation Analysis
The current `execute_delegate` handles the following consecutive phases:
1. **State Loading & Validation:** Checks if the parent project exists and if the child sub-agent is already running. Exits with code `1` on failure.
2. **Workspace Authorization:** Calls `vault::authorize_workspace` for the project.
3. **Playbook Inheritance Injection:** Checks for `AGENTS.md` in the parent directory and reads it.
4. **Context Injection:** Checks for `context.md` in the parent directory and reads it.
5. **Completion Report Instruction:** Prepares system completion guidelines.
6. **Procedural Skills Injection (JIT Matching):** Scans `memory/skills`, extracts skill keywords matching the subtask goal, and constructs the skills list.
7. **Tool Call Formatting Instruction:** Declares formatting rules to avoid double-quoted string timeout errors.
8. **Sub-Agent Background Spawning:** Spawns a PTY background runner via `spawn_agy_background`.
9. **State Updating & Persistence:** Inserts sub-agent info into state and saves it.
10. **Log Output Reporting:** Prints progress messages to the console.

---

## 3. Proposed Refactoring Architecture

### 3.1 Added Imports & Constants
We introduce standard `HashMap` import for clear type annotations:
```rust
use std::collections::HashMap;
```

We extract the static critical tool call formatting rules into a module-level constant:
```rust
const TOOL_FORMAT_INSTRUCTION: &str = "\n\n==================================================\n\
     CRITICAL TOOL CALL FORMATTING RULES:\n\
     When calling platform tools (e.g., view_file, list_dir, grep_search, write_to_file, replace_file_content):\n\
     - Do NOT wrap string arguments (like paths or queries) in nested or escaped double quotes.\n\
     - Correct: \"AbsolutePath\": \"/path/to/file\"\n\
     - Incorrect: \"AbsolutePath\": \"\\\"/path/to/file\\\"\"\n\
     Failure to follow this will cause sandbox permission validation to time out and fail!\n\
     ==================================================\n";
```

### 3.2 Helper Functions Design

#### Helper 1: `get_and_validate_projects`
- **Role:** Handles parent project validation and child status verification.
- **Signature:**
  ```rust
  fn get_and_validate_projects(
      state: &mut HashMap<String, ProjectInfo>,
      parent: &str,
      subtask: &str,
  ) -> (ProjectInfo, String)
  ```
- **Error Handling:** Exits standard CLI using `std::process::exit(1)` upon validation failures (preserving exact original flow).

#### Helper 2: `get_parent_agents_injection`
- **Role:** Checks for and formats `AGENTS.md` contents.
- **Signature:**
  ```rust
  fn get_parent_agents_injection(parent: &str, project_path: &Path) -> String
  ```

#### Helper 3: `get_parent_context_injection`
- **Role:** Checks for and formats `context.md` active context.
- **Signature:**
  ```rust
  fn get_parent_context_injection(project_path: &Path) -> String
  ```

#### Helper 4: `get_report_instruction`
- **Role:** Formats standard instructions for completing subtasks (generating `report.md`).
- **Signature:**
  ```rust
  fn get_report_instruction(project_path_str: &str) -> String
  ```

#### Helper 5: `get_skills_injection`
- **Role:** Scans the skills registry directory, tokenizes goals, finds matching skills, and forms instructions.
- **Signature:**
  ```rust
  fn get_skills_injection(base_dir: &Path, goal: &str) -> String
  ```

#### Helper 6: `spawn_subagent_and_update_state`
- **Role:** Prepares the runner arguments, executes in PTY mode in background, updates internal state maps, and persists to disk.
- **Signature:**
  ```rust
  fn spawn_subagent_and_update_state(
      state: &mut HashMap<String, ProjectInfo>,
      child_name: &str,
      project_path: &str,
      goal: &str,
      final_prompt: &str,
      log_file_path: &Path,
  ) -> io::Result<u32>
  ```

---

## 4. Concrete Code Changes (Diff)
Here is the proposed patch to `src/backend/commands/utils.rs`:

```diff
diff --git a/src/backend/commands/utils.rs b/src/backend/commands/utils.rs
index e0657ff..b1b2cd8 100644
--- a/src/backend/commands/utils.rs
+++ b/src/backend/commands/utils.rs
@@ -1,13 +1,24 @@
 use std::fs::{self, File};
 use std::io::{self, Read, Write};
 use std::path::Path;
+use std::collections::HashMap;
 
 use chrono::Local;
 
 use crate::models::ProjectInfo;
 use crate::backend::vault::get_base_dir;
 use crate::backend::state::{load_state, save_state, check_project_status};
 use crate::backend::daemon::{is_daemon_running, get_daemon_pid};
 use crate::backend::cli::CliResult;
 
+const TOOL_FORMAT_INSTRUCTION: &str = "\n\n==================================================\n\
+     CRITICAL TOOL CALL FORMATTING RULES:\n\
+     When calling platform tools (e.g., view_file, list_dir, grep_search, write_to_file, replace_file_content):\n\
+     - Do NOT wrap string arguments (like paths or queries) in nested or escaped double quotes.\n\
+     - Correct: \"AbsolutePath\": \"/path/to/file\"\n\
+     - Incorrect: \"AbsolutePath\": \"\\\"/path/to/file\\\"\"\n\
+     Failure to follow this will cause sandbox permission validation to time out and fail!\n\
+     ==================================================\n";
+
 pub fn execute_compress(name: String) -> io::Result<CliResult> {
     let base_dir = get_base_dir();
@@ -89,191 +100,195 @@ pub fn execute_search_history(name: String, query: String) -> io::Result<CliResu
-pub fn execute_delegate(parent: String, subtask: String, goal: String) -> io::Result<CliResult> {
-    let mut state = load_state();
-    let base_dir = get_base_dir();
-    let parent_info = match state.get(&parent) {
-        Some(info) => info.clone(),
-        None => {
-            eprintln!("Error: Parent project '{}' not found in projects.json.", parent);
-            std::process::exit(1);
-        }
-    };
-
-    let child_name = format!("{}_sub_{}", parent, subtask);
-    
-    if let Some(info) = state.get_mut(&child_name) {
-        if check_project_status(&child_name, info) == "running" {
-            eprintln!("Error: Sub-agent '{}' is already running with PID {}.", child_name, info.pid);
-            std::process::exit(1);
-        }
-    }
-
-    let project_path_str = parent_info.path.clone();
-
-    // Automatically authorize the subagent workspace path
-    let _ = crate::backend::vault::authorize_workspace(&project_path_str);
-    
-    // AGENTS.md inheritance
-    let parent_agents_path = Path::new(&project_path_str).join("AGENTS.md");
-    let mut agents_inject = String::new();
-    if parent_agents_path.exists() {
-        if let Ok(content) = fs::read_to_string(&parent_agents_path) {
-            agents_inject = format!(
-                "\n\n==================================================\n\
-                 [PARENT PROJECT PLAYBOOK - AGENTS.MD]\n\
-                 (This subtask belongs to parent project '{}'. Follow these guidelines!)\n\n\
-                 {}\n\
-                 ==================================================\n\n",
-                parent, content.trim()
-            );
-        }
-    }
-
-    // Parent context.md JIT inject
-    let parent_context_path = Path::new(&project_path_str).join("context.md");
-    let mut parent_context_inject = String::new();
-    if parent_context_path.exists() {
-        if let Ok(content) = fs::read_to_string(&parent_context_path) {
-            parent_context_inject = format!(
-                "\n\n==================================================\n\
-                 [PARENT ACTIVE CONTEXT - HOT MEMORY]\n\
-                 (Current parent project state for your reference):\n\n\
-                 {}\n\
-                 ==================================================\n\n",
-                content.trim()
-            );
-        }
-    }
-
-    let report_instruction = format!(
-        "\n\n==================================================\n\
-         SYSTEM INSTRUCTIONS FOR COMPLETION:\n\
-         Once you complete this subtask, you MUST generate a 'report.md' file in the root of the project directory ({})\n\
-         This report must contain:\n\
-         1. A summary of completed tasks.\n\
-         2. Crucial design/architectural choices made.\n\
-         3. Minor choices resolved autonomously.\n\
-         4. A section 'CRITICAL ITEMS FOR REVIEW' containing only items that require manual review or escalation. If none, clearly state 'None'.\n\n\
-         Ensure this report is written before you finish. The orchestrator will automatically consolidate this subtask report back into the parent project context.",
-        project_path_str
-    );
-
-    // JIT Skills Catalog Auto-Injection
-    let skills_dir = base_dir.join("memory/skills");
-    let mut skills_inject = String::new();
-    if skills_dir.exists() {
-        let mut matched_skills = Vec::new();
-        let goal_lower = goal.to_lowercase();
-        let keywords: std::collections::HashSet<String> = goal_lower
-            .split(|c: char| !c.is_alphanumeric())
-            .filter(|s| s.len() >= 4)
-            .map(|s| s.to_string())
-            .collect();
-
-        if let Ok(entries) = fs::read_dir(&skills_dir) {
-            for entry in entries.flatten() {
-                let path = entry.path();
-                if path.extension().is_some_and(|ext| ext == "md") {
-                    if let Ok(content) = fs::read_to_string(&path) {
-                        let mut skill_name = String::new();
-                        let mut skill_desc = String::new();
-                        for line in content.lines() {
-                            let trimmed = line.trim();
-                            if trimmed.starts_with("name:") {
-                                skill_name = trimmed.trim_start_matches("name:").trim().to_string();
-                            } else if trimmed.starts_with("description:") {
-                                skill_desc = trimmed.trim_start_matches("description:").trim().to_string();
-                            }
-                        }
-                        if !skill_name.is_empty() {
-                            let skill_name_lower = skill_name.to_lowercase();
-                            let skill_desc_lower = skill_desc.to_lowercase();
-                            let is_match = keywords.iter().any(|kw| {
-                                kw != "this" && kw != "that" && kw != "with" && kw != "from" &&
-                                (skill_name_lower.contains(kw) || skill_desc_lower.contains(kw))
-                            });
-                            if is_match || goal_lower.contains(&skill_name_lower) {
-                                matched_skills.push((skill_name, skill_desc));
-                            }
-                        }
-                    }
-                }
-            }
-        }
-        if !matched_skills.is_empty() {
-            let mut skills_list = String::new();
-            for (s_name, s_desc) in matched_skills {
-                skills_list.push_str(&format!("- name: {}\n  description: {}\n", s_name, s_desc));
-            }
-            let current_exe = std::env::current_exe()
-                .map(|p| p.to_string_lossy().to_string())
-                .unwrap_or_else(|_| "agy-orchestrator".to_string());
-            skills_inject = format!(
-                "\n\n==================================================\n\
-                 [AVAILABLE PROCEDURAL SKILLS (Level 0 Index)]\n\
-                 (To load full instructions, execute: `{} load-skill --name <skill_name>`)\n\n\
-                 {}\
-                 ==================================================\n\n",
-                current_exe,
-                skills_list
-            );
-        }
-    }
-
-    let tool_format_instruction = "\n\n==================================================\n\
-         CRITICAL TOOL CALL FORMATTING RULES:\n\
-         When calling platform tools (e.g., view_file, list_dir, grep_search, write_to_file, replace_file_content):\n\
-         - Do NOT wrap string arguments (like paths or queries) in nested or escaped double quotes.\n\
-         - Correct: \"AbsolutePath\": \"/path/to/file\"\n\
-         - Incorrect: \"AbsolutePath\": \"\\\"/path/to/file\\\"\"\n\
-         Failure to follow this will cause sandbox permission validation to time out and fail!\n\
-         ==================================================\n";
-
-    let final_prompt = format!(
-        "{}{}{}{}{}{}",
-        agents_inject,
-        parent_context_inject,
-        skills_inject,
-        goal,
-        report_instruction,
-        tool_format_instruction
-    );
-
-    // agy_runner를 통해 PTY 백그라운드로 실행.
-    // rexpect가 invoke_subagent 서브에이전트 권한 팝업 등 unexpected interactive
-    // 프롬프트를 자동 응답하여 hang 없이 완료되도록 보장합니다.
-    let log_file_path = base_dir.join("logs").join(format!("{}.log", child_name));
-
-    let agy_args = vec![
-        "--add-dir".to_string(),
-        project_path_str.clone(),
-        "--dangerously-skip-permissions".to_string(),
-        "--print".to_string(),
-        final_prompt.clone(),
-    ];
-
-    let child_pid = crate::backend::agy_runner::spawn_agy_background(
-        agy_args,
-        Some(log_file_path.clone()),
-        None, // 기본 타임아웃 10분
-    )?;
-
-
-    state.insert(
-        child_name.clone(),
-        ProjectInfo {
-            path: project_path_str.clone(),
-            goal: goal.clone(),
-            pid: child_pid,
-            status: "running".to_string(),
-            spawned_at: Local::now().to_rfc3339(),
-        },
-    );
-    save_state(&state)?;
-
-    let log_display = log_file_path.canonicalize()
-        .map(|p| p.to_string_lossy().into_owned())
-        .unwrap_or_else(|_| log_file_path.to_string_lossy().into_owned());
-
-    println!("Successfully spawned sub-agent '{}' in background (PTY mode).", child_name);
-    println!("Logs: {}", log_display);
-
-    Ok(CliResult::Exit)
-}
+pub fn execute_delegate(parent: String, subtask: String, goal: String) -> io::Result<CliResult> {
+    let mut state = load_state();
+    let base_dir = get_base_dir();
+
+    // 1. Resolve and validate parent/child projects
+    let (parent_info, child_name) = get_and_validate_projects(&mut state, &parent, &subtask);
+    let project_path_str = parent_info.path.clone();
+
+    // Automatically authorize the subagent workspace path
+    let _ = crate::backend::vault::authorize_workspace(&project_path_str);
+
+    // 2. Generate Prompt Components
+    let agents_inject = get_parent_agents_injection(&parent, Path::new(&project_path_str));
+    let parent_context_inject = get_parent_context_injection(Path::new(&project_path_str));
+    let report_instruction = get_report_instruction(&project_path_str);
+    let skills_inject = get_skills_injection(&base_dir, &goal);
+
+    let final_prompt = format!(
+        "{}{}{}{}{}{}",
+        agents_inject,
+        parent_context_inject,
+        skills_inject,
+        goal,
+        report_instruction,
+        TOOL_FORMAT_INSTRUCTION
+    );
+
+    // 3. Spawn Subagent & Update State
+    let log_file_path = base_dir.join("logs").join(format!("{}.log", child_name));
+    let _child_pid = spawn_subagent_and_update_state(
+        &mut state,
+        &child_name,
+        &project_path_str,
+        &goal,
+        &final_prompt,
+        &log_file_path,
+    )?;
+
+    let log_display = log_file_path.canonicalize()
+        .map(|p| p.to_string_lossy().into_owned())
+        .unwrap_or_else(|_| log_file_path.to_string_lossy().into_owned());
+
+    println!("Successfully spawned sub-agent '{}' in background (PTY mode).", child_name);
+    println!("Logs: {}", log_display);
+
+    Ok(CliResult::Exit)
+}
+
+/// Validates and retrieves the parent project's details, and checks that the
+/// child/sub-agent process is not already running.
+/// If validation fails, prints an error message and exits with status 1.
+fn get_and_validate_projects(
+    state: &mut HashMap<String, ProjectInfo>,
+    parent: &str,
+    subtask: &str,
+) -> (ProjectInfo, String) {
+    let parent_info = match state.get(parent) {
+        Some(info) => info.clone(),
+        None => {
+            eprintln!("Error: Parent project '{}' not found in projects.json.", parent);
+            std::process::exit(1);
+        }
+    };
+
+    let child_name = format!("{}_sub_{}", parent, subtask);
+
+    if let Some(info) = state.get_mut(&child_name) {
+        if check_project_status(&child_name, info) == "running" {
+            eprintln!("Error: Sub-agent '{}' is already running with PID {}.", child_name, info.pid);
+            std::process::exit(1);
+        }
+    }
+
+    (parent_info, child_name)
+}
+
+/// Reads the parent's `AGENTS.md` file (if it exists) and formats it as system instruction injection.
+fn get_parent_agents_injection(parent: &str, project_path: &Path) -> String {
+    let parent_agents_path = project_path.join("AGENTS.md");
+    if parent_agents_path.exists() {
+        if let Ok(content) = fs::read_to_string(&parent_agents_path) {
+            return format!(
+                "\n\n==================================================\n\
+                 [PARENT PROJECT PLAYBOOK - AGENTS.MD]\n\
+                 (This subtask belongs to parent project '{}'. Follow these guidelines!)\n\n\
+                 {}\n\
+                 ==================================================\n\n",
+                parent, content.trim()
+            );
+        }
+    }
+    String::new()
+}
+
+/// Reads the parent's `context.md` file (if it exists) and formats it as system instruction injection.
+fn get_parent_context_injection(project_path: &Path) -> String {
+    let parent_context_path = project_path.join("context.md");
+    if parent_context_path.exists() {
+        if let Ok(content) = fs::read_to_string(&parent_context_path) {
+            return format!(
+                "\n\n==================================================\n\
+                 [PARENT ACTIVE CONTEXT - HOT MEMORY]\n\
+                 (Current parent project state for your reference):\n\n\
+                 {}\n\
+                 ==================================================\n\n",
+                content.trim()
+            );
+        }
+    }
+    String::new()
+}
+
+/// Formats the system instructions that prompt the sub-agent to generate a `report.md` on completion.
+fn get_report_instruction(project_path_str: &str) -> String {
+    format!(
+        "\n\n==================================================\n\
+         SYSTEM INSTRUCTIONS FOR COMPLETION:\n\
+         Once you complete this subtask, you MUST generate a 'report.md' file in the root of the project directory ({})\n\
+         This report must contain:\n\
+         1. A summary of completed tasks.\n\
+         2. Crucial design/architectural choices made.\n\
+         3. Minor choices resolved autonomously.\n\
+         4. A section 'CRITICAL ITEMS FOR REVIEW' containing only items that require manual review or escalation. If none, clearly state 'None'.\n\n\
+         Ensure this report is written before you finish. The orchestrator will automatically consolidate this subtask report back into the parent project context.",
+        project_path_str
+    )
+}
+
+/// Searches the global skills directory for any skills matching keywords derived from the subtask goal.
+/// If matches are found, compiles them into a system instruction block.
+fn get_skills_injection(base_dir: &Path, goal: &str) -> String {
+    let skills_dir = base_dir.join("memory/skills");
+    if !skills_dir.exists() {
+        return String::new();
+    }
+
+    let mut matched_skills = Vec::new();
+    let goal_lower = goal.to_lowercase();
+    let keywords: std::collections::HashSet<String> = goal_lower
+        .split(|c: char| !c.is_alphanumeric())
+        .filter(|s| s.len() >= 4)
+        .map(|s| s.to_string())
+        .collect();
+
+    if let Ok(entries) = fs::read_dir(&skills_dir) {
+        for entry in entries.flatten() {
+            let path = entry.path();
+            if path.extension().is_some_and(|ext| ext == "md") {
+                if let Ok(content) = fs::read_to_string(&path) {
+                    let mut skill_name = String::new();
+                    let mut skill_desc = String::new();
+                    for line in content.lines() {
+                        let trimmed = line.trim();
+                        if trimmed.starts_with("name:") {
+                            skill_name = trimmed.trim_start_matches("name:").trim().to_string();
+                        } else if trimmed.starts_with("description:") {
+                            skill_desc = trimmed.trim_start_matches("description:").trim().to_string();
+                        }
+                    }
+                    if !skill_name.is_empty() {
+                        let skill_name_lower = skill_name.to_lowercase();
+                        let skill_desc_lower = skill_desc.to_lowercase();
+                        let is_match = keywords.iter().any(|kw| {
+                            kw != "this" && kw != "that" && kw != "with" && kw != "from" &&
+                            (skill_name_lower.contains(kw) || skill_desc_lower.contains(kw))
+                        });
+                        if is_match || goal_lower.contains(&skill_name_lower) {
+                            matched_skills.push((skill_name, skill_desc));
+                        }
+                    }
+                }
+            }
+        }
+    }
+
+    if !matched_skills.is_empty() {
+        let mut skills_list = String::new();
+        for (s_name, s_desc) in matched_skills {
+            skills_list.push_str(&format!("- name: {}\n  description: {}\n", s_name, s_desc));
+        }
+        let current_exe = std::env::current_exe()
+            .map(|p| p.to_string_lossy().to_string())
+            .unwrap_or_else(|_| "agy-orchestrator".to_string());
+        format!(
+            "\n\n==================================================\n\
+             [AVAILABLE PROCEDURAL SKILLS (Level 0 Index)]\n\
+             (To load full instructions, execute: `{} load-skill --name <skill_name>`)\n\n\
+             {}\
+             ==================================================\n\n",
+            current_exe,
+            skills_list
+        )
+    } else {
+        String::new()
+    }
+}
+
+/// Spawns the sub-agent process in the background using `spawn_agy_background` and records the status in state.
+fn spawn_subagent_and_update_state(
+    state: &mut HashMap<String, ProjectInfo>,
+    child_name: &str,
+    project_path: &str,
+    goal: &str,
+    final_prompt: &str,
+    log_file_path: &Path,
+) -> io::Result<u32> {
+    let agy_args = vec![
+        "--add-dir".to_string(),
+        project_path.to_string(),
+        "--dangerously-skip-permissions".to_string(),
+        "--print".to_string(),
+        final_prompt.to_string(),
+    ];
+
+    let child_pid = crate::backend::agy_runner::spawn_agy_background(
+        agy_args,
+        Some(log_file_path.to_path_buf()),
+        None, // Default timeout 10 minutes
+    )?;
+
+    state.insert(
+        child_name.to_string(),
+        ProjectInfo {
+            path: project_path.to_string(),
+            goal: goal.to_string(),
+            pid: child_pid,
+            status: "running".to_string(),
+            spawned_at: Local::now().to_rfc3339(),
+        },
+    );
+    save_state(state)?;
+
+    Ok(child_pid)
+}
```

---

## 5. Verification & Testing Plan
After applying the refactoring changes:

1. **Syntax & Compilation Check:**
   Run `cargo check` to ensure that all imports, types, signatures, and module linkages are correct and compile without warnings.
   ```bash
   cargo check
   ```

2. **Integration Verification:**
   Launch a subtask delegation via the CLI or UI (e.g. `agy-orchestrator delegate --parent <parent> --subtask <subtask> --goal <goal>`) and verify that:
   - Appropriate workspace path is authorized.
   - Parent guidelines (`AGENTS.md` and `context.md`) are successfully loaded and injected into the prompt.
   - Associated skill catalogs are matched and loaded under `AVAILABLE PROCEDURAL SKILLS`.
   - The subtask agent is spawned successfully in the background and a PTY log is created in the target base path directory.
   - Internal CLI project status transitions to `running`.
