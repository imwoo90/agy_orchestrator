# Refactoring Plan: Project Command Modernization

This document outlines the detailed refactoring plan for `src/backend/commands/project.rs` to extract presentation/formatting helpers, replace `std::process::exit(1)` code smells with standard Rust error propagation (`std::io::Result`), and plan for comprehensive unit tests.

---

## 1. Objectives & Architectural Decisions

* **Extract Presentation & Formatting Helpers**: Decouple command orchestration logic from raw terminal rendering/printing. This ensures that formatting and layout changes do not affect core project management logic.
* **Injectable Generic Writers**: Modernize all printing helpers to accept a generic writer parameter `w: &mut W` (where `W: std::io::Write`), rendering the presentation layer fully unit-testable via memory buffers (e.g., `Vec<u8>`).
* **Replace `std::process::exit(1)`**: Eliminate direct process exits to allow upstream callers (like CLI entrypoint, future dashboard APIs, or parent orchestrator routines) to handle errors cleanly. Failures will propagate as structured `std::io::Error` instances.
* **Enable Pure Logical Testing**: Extract file-parsing logic and project data construction into pure functional cores, facilitating unit testing without direct disk dependency.
* **Mock State in Tests**: Establish isolated test-homes in target directories for filesystem side-effect testing, preventing pollution of real developer databases.

---

## 2. Refactoring Outline & Helper Signatures

### 2.1. Formatting & Presentation Helpers

We will extract output formatting and terminal-printing layouts into dedicated presentation helpers:

```rust
/// Formats the RFC3339 timestamp of project spawning for CLI tabular display.
///
/// Converts "YYYY-MM-DDT..." to "YYYY-MM-DD HH:MM:SS" (or similar prefix).
pub fn format_spawned_at(spawned_at: &str) -> String {
    spawned_at.get(..19).unwrap_or(spawned_at).replace('T', " ")
}

/// Renders the projects list table to the specified writer.
pub fn render_projects_table<W: std::io::Write>(
    w: &mut W,
    state: &mut std::collections::HashMap<String, ProjectInfo>,
) -> io::Result<()> {
    if state.is_empty() {
        writeln!(w, "No projects registered.")?;
        return Ok(());
    }

    writeln!(
        w,
        "{:<15} | {:<6} | {:<10} | {:<20} | Path",
        "Project Name", "PID", "Status", "Spawned At"
    )?;
    writeln!(w, "{}", "-".repeat(80))?;

    for (name, info) in state.iter_mut() {
        let status = check_project_status(name, info);
        let spawned = format_spawned_at(&info.spawned_at);
        writeln!(
            w,
            "{:<15} | {:<6} | {:<10} | {:<20} | {}",
            name, info.pid, status, spawned, info.path
        )?;
    }
    Ok(())
}

/// Renders detailed project status report to the specified writer.
pub fn render_project_status<W: std::io::Write>(
    w: &mut W,
    details: &ProjectStatusDetails,
) -> io::Result<()> {
    writeln!(w, "Project: {}", details.name)?;
    writeln!(w, "Path: {}", details.path)?;
    writeln!(w, "Status: {}", details.status)?;
    writeln!(w, "PID: {}", details.pid)?;
    writeln!(w, "Spawned At: {}", details.spawned_at)?;
    writeln!(w, "Goal: {}", details.goal)?;

    if let Some(ref report_content) = details.report_content {
        writeln!(w, "\n--- [report.md Content] ---")?;
        writeln!(w, "{}", report_content)?;
    } else {
        writeln!(w, "\nReport file not found at: {}/report.md", details.path)?;
        if details.status == "failed" {
            if let Some(ref log_path) = details.log_suggestion {
                writeln!(w, "Note: Project failed. Check logs for details: {}", log_path.display())?;
            }
        }
    }
    Ok(())
}

/// Renders detailed project context (playbook, hot memory, cold memory size) to the specified writer.
pub fn render_project_context<W: std::io::Write>(
    w: &mut W,
    details: &ProjectContextDetails,
) -> io::Result<()> {
    writeln!(w, "Project: {}", details.name)?;
    writeln!(w, "Path: {}", details.path)?;
    writeln!(w, "Status: {}", details.status)?;

    if let Some(ref playbook) = details.playbook {
        writeln!(w, "\n--- [AGENTS.md Content (Project Playbook)] ---")?;
        writeln!(w, "{}", playbook)?;
    } else {
        writeln!(w, "\nNo AGENTS.md (Project Playbook) file exists yet in the project directory.")?;
    }

    if let Some(ref hot_memory) = details.hot_memory {
        writeln!(w, "\n--- [context.md Content (Hot Memory)] ---")?;
        writeln!(w, "{}", hot_memory)?;
    } else {
        writeln!(w, "\nNo context.md (Hot Memory) file exists yet in the project directory.")?;
    }

    writeln!(w, "\n--- [context_history.md Status (Cold Memory)] ---")?;
    if details.cold_memory_exists {
        if let Some(size) = details.cold_memory_size {
            writeln!(w, "Archive file exists. Size: {} bytes", size)?;
        } else {
            writeln!(w, "Archive file exists.")?;
        }
    } else {
        writeln!(w, "No context_history.md (Cold Memory) file exists yet.")?;
    }
    Ok(())
}

/// Renders health check results to the specified writer.
pub fn render_health_checks<W: std::io::Write>(
    w: &mut W,
    results: &[HealthCheckResult],
) -> io::Result<()> {
    writeln!(w, "{:<25} | {:<8} | {:<20} | Message", "Target", "Status", "Checked At")?;
    writeln!(w, "{}", "-".repeat(90))?;
    for r in results {
        let status = if r.healthy { "✅ OK" } else { "❌ FAIL" };
        writeln!(w, "{:<25} | {:<8} | {:<20} | {}", r.target, status, r.checked_at, r.message)?;
    }
    let healthy_count = results.iter().filter(|r| r.healthy).count();
    let failed_count = results.len() - healthy_count;
    writeln!(w, "\nSummary: {} passed, {} failed.", healthy_count, failed_count)?;
    Ok(())
}
```

### 2.2. Pure Logical Core Helpers

We will extract the logic routines so that execution flows do not read files inline or print directly:

```rust
pub struct ProjectStatusDetails {
    pub name: String,
    pub path: String,
    pub status: String,
    pub pid: u32,
    pub spawned_at: String,
    pub goal: String,
    pub report_content: Option<String>,
    pub log_suggestion: Option<std::path::PathBuf>,
}

pub struct ProjectContextDetails {
    pub name: String,
    pub path: String,
    pub status: String,
    pub playbook: Option<String>,
    pub hot_memory: Option<String>,
    pub cold_memory_size: Option<u64>,
    pub cold_memory_exists: bool,
}

/// Parses the "Lessons Learned" section out of report.md.
///
/// Looks for headings matching "Lessons Learned", "교훈", or "지식".
pub fn parse_lessons_learned(content: &str) -> String {
    let mut lessons_content = String::new();
    let mut in_lessons = false;
    
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let header_title = trimmed.trim_start_matches('#').trim().to_lowercase();
            if header_title.contains("lessons learned") || header_title == "교훈" || header_title == "지식" {
                in_lessons = true;
                continue;
            } else {
                in_lessons = false;
            }
        }
        
        if in_lessons {
            lessons_content.push_str(line);
            lessons_content.push('\n');
        }
    }
    lessons_content.trim().to_string()
}

/// Evaluates sub-agent naming to automatically append completed reports to parent contexts.
pub fn handle_parent_feedback_loop(
    name: &str,
    report_content: &str,
    state: &std::collections::HashMap<String, ProjectInfo>,
) -> io::Result<Option<String>> {
    if !name.contains("_sub_") {
        return Ok(None);
    }
    if let Some(sub_idx) = name.rfind("_sub_") {
        let parent_name = &name[..sub_idx];
        let subtask_name = &name[sub_idx + 5..];
        
        if let Some(parent_info) = state.get(parent_name) {
            let parent_context_path = Path::new(&parent_info.path).join("context.md");
            let mut parent_context_file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&parent_context_path)?;
                
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            writeln!(
                parent_context_file,
                "\n\n==================================================\n\
                 # 📢 Subtask Completed: '{}' at {}\n\
                 - Sub-agent Name: {}\n\
                 - Completed Report:\n\n\
                 {}\n\
                 ==================================================\n",
                subtask_name, timestamp, name, report_content.trim()
            )?;
            return Ok(Some(parent_name.to_string()));
        }
    }
    Ok(None)
}
```

---

## 3. Modernized Execute Functions

By utilizing the extracted formatting and logic helpers, we replace `std::process::exit(1)` with `io::Result`:

### 3.1. `execute_list()`
```rust
pub fn execute_list() -> io::Result<CliResult> {
    let mut state = load_state();
    render_projects_table(&mut io::stdout(), &mut state)?;
    save_state(&state)?;
    Ok(CliResult::Exit)
}
```

### 3.2. `execute_status(name: String)`
```rust
pub fn execute_status(name: String) -> io::Result<CliResult> {
    let mut state = load_state();
    let base_dir = get_base_dir();
    
    let info = state.get_mut(&name).ok_or_else(|| {
        eprintln!("Error: Project '{}' not found.", name);
        io::Error::new(io::ErrorKind::NotFound, format!("Project '{}' not found.", name))
    })?;
    
    let status = check_project_status(&name, info);
    let path_str = info.path.clone();
    let report_path = Path::new(&path_str).join("report.md");
    
    let report_content = if report_path.exists() {
        let mut content = String::new();
        File::open(report_path)?.read_to_string(&mut content)?;
        Some(content)
    } else {
        None
    };

    let details = ProjectStatusDetails {
        name: name.clone(),
        path: path_str,
        status: status.clone(),
        pid: info.pid,
        spawned_at: info.spawned_at.clone(),
        goal: info.goal.clone(),
        report_content,
        log_suggestion: Some(base_dir.join("logs").join(format!("{}.log", name))),
    };
    
    save_state(&state)?;
    render_project_status(&mut io::stdout(), &details)?;
    Ok(CliResult::Exit)
}
```

### 3.3. `execute_get_context(name: String)`
```rust
pub fn execute_get_context(name: String) -> io::Result<CliResult> {
    let mut state = load_state();
    
    let info = state.get_mut(&name).ok_or_else(|| {
        eprintln!("Error: Project '{}' not found.", name);
        io::Error::new(io::ErrorKind::NotFound, format!("Project '{}' not found.", name))
    })?;
    
    let status = check_project_status(&name, info);
    let path_str = info.path.clone();
    
    let agents_path = Path::new(&path_str).join("AGENTS.md");
    let playbook = if agents_path.exists() {
        let mut content = String::new();
        File::open(&agents_path).and_then(|mut f| f.read_to_string(&mut content)).ok().map(|_| content)
    } else {
        None
    };

    let context_path = Path::new(&path_str).join("context.md");
    let hot_memory = if context_path.exists() {
        let mut content = String::new();
        File::open(context_path)?.read_to_string(&mut content)?;
        Some(content)
    } else {
        None
    };

    let history_path = Path::new(&path_str).join("context_history.md");
    let (cold_memory_exists, cold_memory_size) = if history_path.exists() {
        let size = fs::metadata(&history_path).ok().map(|m| m.len());
        (true, size)
    } else {
        (false, None)
    };

    let details = ProjectContextDetails {
        name,
        path: path_str,
        status,
        playbook,
        hot_memory,
        cold_memory_size,
        cold_memory_exists,
    };
    
    save_state(&state)?;
    render_project_context(&mut io::stdout(), &details)?;
    Ok(CliResult::Exit)
}
```

### 3.4. `execute_consolidate(name: String)`
```rust
pub fn execute_consolidate(name: String) -> io::Result<CliResult> {
    let mut state = load_state();
    let base_dir = get_base_dir();
    
    let info = state.get_mut(&name).ok_or_else(|| {
        eprintln!("Error: Project '{}' not found.", name);
        io::Error::new(io::ErrorKind::NotFound, format!("Project '{}' not found.", name))
    })?;

    let status = check_project_status(&name, info);
    if status == "running" {
        eprintln!("Error: Cannot consolidate project '{}' while it is still running.", name);
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("Cannot consolidate project '{}' while it is still running.", name)
        ));
    }

    info.status = "completed".to_string();
    let path_str = info.path.clone();
    let spawned_at = info.spawned_at.clone();

    let report_path = Path::new(&path_str).join("report.md");
    if !report_path.exists() {
        eprintln!("Error: report.md not found at {}. Cannot consolidate.", report_path.display());
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("report.md not found at {}.", report_path.display())
        ));
    }

    let mut report_content = String::new();
    File::open(&report_path)?.read_to_string(&mut report_content)?;

    let lessons_trimmed = parse_lessons_learned(&report_content);
    if !lessons_trimmed.is_empty() {
        let vault_dir = base_dir.join("memory/vault");
        fs::create_dir_all(&vault_dir)?;
        let lessons_file_path = vault_dir.join(format!("{}_lessons.md", name));
        let mut file = File::create(&lessons_file_path)?;
        writeln!(
            file,
            "# 🧠 Lessons Learned from Project '{}'\n\n*Saved on: {}*\n\n{}",
            name,
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            lessons_trimmed
        )?;
        println!("Extracted lessons learned and saved to {}", lessons_file_path.display());

        if let Ok(mut log_file) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(base_dir.join("notifications.log"))
        {
            let _ = writeln!(
                log_file,
                "[{}] INFO: Extracted lessons learned from '{}' and saved to vault.",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                name
            );
        }
    }

    let history_path = Path::new(&path_str).join("context_history.md");
    let mut history_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&history_path)?;

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    writeln!(
        history_file,
        "\n\n# 📅 History log from {} (Spawned at {})\n\n{}",
        timestamp, spawned_at, report_content
    )?;

    let context_path = Path::new(&path_str).join("context.md");
    if !context_path.exists() {
        let mut context_file = File::create(&context_path)?;
        writeln!(
            context_file,
            "# Active Project Context\n\n\
             ## Project Name: {}\n\
             ## Status: Completed (Initialized from fallback at {})\n\n\
             ### Last Task Summary\n\
             {}",
            name, timestamp, report_content
        )?;
        println!("Hot Memory: Initialized context.md from report fallback.");
    }

    if let Err(e) = fs::remove_file(&report_path) {
        eprintln!("Warning: Failed to remove report.md at {}: {}", report_path.display(), e);
    } else {
        println!("Cleaned up report.md after consolidation.");
    }

    handle_parent_feedback_loop(&name, &report_content, &state)?;
    save_state(&state)?;

    println!("Successfully consolidated report.md into context_history.md for project '{}'.", name);
    println!("Updated status to 'completed' in projects.json.");
    Ok(CliResult::Exit)
}
```

### 3.5. `execute_health_check()`
```rust
pub fn execute_health_check() -> io::Result<CliResult> {
    println!("Running health checks on all registered targets...\n");
    let results = run_health_checks().map_err(|e| {
        eprintln!("Health check error: {}", e);
        io::Error::new(io::ErrorKind::Other, format!("Health check error: {}", e))
    })?;
    
    render_health_checks(&mut io::stdout(), &results)?;
    Ok(CliResult::Exit)
}
```

---

## 4. Unit Testing Strategy

We will introduce a unit test suite under `#[cfg(test)]` in `src/backend/commands/project.rs`:

1. **`test_format_spawned_at`**:
   - Asserts timestamp slicing and T-separator replacement behavior for RFC3339 formatted timestamps.
   - Asserts handling of malformed input (gracefully falling back to original string).

2. **`test_parse_lessons_learned`**:
   - Asserts header extraction for various headings: `# Lessons Learned`, `## 교훈`, `### 지식`.
   - Asserts that text under other headers is ignored.
   - Asserts correct formatting of multi-line content inside lessons learned sections.

3. **`test_handle_parent_feedback_loop`**:
   - Verifies logic mapping between sub-agent names (e.g. `parent_sub_task`) and parent directories.
   - Sets up a temporary workspace and checks if reports are properly appended to parent's `context.md` file.

4. **`test_render_helpers`**:
   - Tests `render_projects_table`, `render_project_status`, `render_project_context`, and `render_health_checks` by passing a dummy `Vec<u8>` writer, verifying the output contains correct formatting, tabular columns, headings, and data fields.

5. **Error handling tests**:
   - Asserts that `execute_status` and `execute_get_context` return `ErrorKind::NotFound` if the requested project is absent from state, verifying the propagated `io::Error`.
   - Asserts that `execute_consolidate` returns `ErrorKind::PermissionDenied` if the targeted project is still running, and `ErrorKind::NotFound` if `report.md` is missing.
