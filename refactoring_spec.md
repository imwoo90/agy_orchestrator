# Refactoring Specification: Error Propagation instead of process::exit(1)

This specification details the changes to be applied to remove `std::process::exit(1)` and return standard Rust `std::io::Error` values instead.

## 📂 Target Changes

### 1. `src/backend/commands/skill.rs`
- **Line 12**:
  - Target:
    ```rust
    if !skills_dir.exists() {
        eprintln!("Error: Skills registry not found.");
        std::process::exit(1);
    }
    ```
  - Replacement:
    ```rust
    if !skills_dir.exists() {
        eprintln!("Error: Skills registry not found.");
        return Err(io::Error::new(io::ErrorKind::NotFound, "Error: Skills registry not found."));
    }
    ```
- **Line 18**:
  - Target:
    ```rust
    if !file_path.exists() {
        eprintln!("Error: Skill '{}' not found.", name);
        std::process::exit(1);
    }
    ```
  - Replacement:
    ```rust
    if !file_path.exists() {
        eprintln!("Error: Skill '{}' not found.", name);
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("Error: Skill '{}' not found.", name)));
    }
    ```
- **Line 40**:
  - Target:
    ```rust
    if sanitized_name.is_empty() {
        eprintln!("Error: Invalid skill name.");
        std::process::exit(1);
    }
    ```
  - Replacement:
    ```rust
    if sanitized_name.is_empty() {
        eprintln!("Error: Invalid skill name.");
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Error: Invalid skill name."));
    }
    ```

---

### 2. `src/backend/commands/spawn.rs`
- **Line 25**:
  - Target:
    ```rust
    if let Some(info) = state.get_mut(&name) {
        if check_project_status(&name, info) == "running" {
            eprintln!("Error: Project '{}' is already running with PID {}.", name, info.pid);
            std::process::exit(1);
        }
    }
    ```
  - Replacement:
    ```rust
    if let Some(info) = state.get_mut(&name) {
        if check_project_status(&name, info) == "running" {
            eprintln!("Error: Project '{}' is already running with PID {}.", name, info.pid);
            return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("Error: Project '{}' is already running with PID {}.", name, info.pid)));
        }
    }
    ```
- **Line 303**:
  - Target:
    ```rust
        Err(e) => {
            eprintln!("Failed to spawn agy command: {}", e);
            std::process::exit(1);
        }
    ```
  - Replacement:
    ```rust
        Err(e) => {
            eprintln!("Failed to spawn agy command: {}", e);
            return Err(e);
        }
    ```

---

### 3. `src/backend/commands/utils.rs`
- **Line 26**:
  - Target:
    ```rust
    if !log_file_path.exists() {
        eprintln!("Error: Log file for project '{}' not found.", name);
        std::process::exit(1);
    }
    ```
  - Replacement:
    ```rust
    if !log_file_path.exists() {
        eprintln!("Error: Log file for project '{}' not found.", name);
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("Error: Log file for project '{}' not found.", name)));
    }
    ```
- **Line 49**:
  - Target:
    ```rust
        None => {
            eprintln!("Error: Project '{}' not found.", name);
            std::process::exit(1);
        }
    ```
  - Replacement:
    ```rust
        None => {
            eprintln!("Error: Project '{}' not found.", name);
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("Error: Project '{}' not found.", name)));
        }
    ```

---

### 4. `src/backend/commands/memory.rs`
- **Line 66**:
  - Target:
    ```rust
    if sanitized_topic.is_empty() {
        eprintln!("Error: Invalid topic name.");
        std::process::exit(1);
    }
    ```
  - Replacement:
    ```rust
    if sanitized_topic.is_empty() {
        eprintln!("Error: Invalid topic name.");
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Error: Invalid topic name."));
    }
    ```
- **Line 84**:
  - Target:
    ```rust
        None => {
            eprintln!("Error: Project '{}' not found in projects.json.", project);
            std::process::exit(1);
        }
    ```
  - Replacement:
    ```rust
        None => {
            eprintln!("Error: Project '{}' not found in projects.json.", project);
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("Error: Project '{}' not found in projects.json.", project)));
        }
    ```
- **Line 91**:
  - Target:
    ```rust
    let vault_dir = base_dir.join("memory/vault");
    if !vault_dir.exists() {
        eprintln!("Error: Memory vault directory not found.");
        std::process::exit(1);
    }
    ```
  - Replacement:
    ```rust
    let vault_dir = base_dir.join("memory/vault");
    if !vault_dir.exists() {
        eprintln!("Error: Memory vault directory not found.");
        return Err(io::Error::new(io::ErrorKind::NotFound, "Error: Memory vault directory not found."));
    }
    ```

---

### 5. `src/backend/commands/upgrade.rs`
- **Line 19**:
  - Target:
    ```rust
            Err(e) => {
                eprintln!("Error checking latest release: {}", e);
                std::process::exit(1);
            }
    ```
  - Replacement:
    ```rust
            Err(e) => {
                eprintln!("Error checking latest release: {}", e);
                return Err(e);
            }
    ```
