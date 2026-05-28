# Refactoring Plan: Log Compression & Delegation Logic

This document details the refactoring plan for `src/backend/commands/utils.rs` to improve its readability, maintainability, and testability.

---

## 1. Objectives & Rationale

- **Decouple I/O from Logic**: Separate file reading/writing operations from core text manipulation logic. This enables unit testing without disk access.
- **Improve Readability**: Replace low-level nested loop index tracking and state variables with clear, helper-driven state-machine-like parsing, or structured state objects.
- **Eliminate Magic Numbers & Stop Words**: Extract hardcoded thresholds (e.g., lines limit, Cargo block skip size) and stopwords into structured constants.
- **Enhance Modularity in Delegation**: Separate sub-agent CLI preparation, prompt rendering, and state storage into cleaner functions or structural helpers.

---

## 2. Log Compression Refactoring (`compress_log_file`)

### Current Issues:
- The function `compress_log_file` accepts a file path, loads the entire content into memory, performs inline processing using a manual `while i < lines.len()` loop with inline lookaheads, and overwrites the file.
- The stateful logic (`in_long_block`, `block_lines`, nested loop scanning for Cargo compiler dependencies) is hard to follow and modify.
- Unit testing is difficult because it requires mocking or creating physical files on disk.

### Proposed Changes:

1. **Extract Core Processing**:
   Create a pure function that does not touch the filesystem:
   ```rust
   pub fn compress_log_content(content: &str) -> String;
   ```
   The original `compress_log_file` will simply perform the file read, call `compress_log_content`, and write back if changes were made.

2. **Introduce Configurations / Constants**:
   Define a local config struct or modular constants for thresholds:
   ```rust
   const LOG_LINE_COMPRESSION_THRESHOLD: usize = 300;
   const MIN_CARGO_SKIP_COUNT: usize = 3;
   const MAX_TOOL_OUTPUT_LINES: usize = 60;
   const TOOL_OUTPUT_BOUNDARY_LINES: usize = 15;
   ```

3. **Restructure Sequential Parser**:
   - Extract the detection/handling of Rust Cargo compilation logs into:
     ```rust
     fn skip_cargo_logs(lines: &[&str], current_index: usize) -> (usize, Option<&'static str>);
     ```
   - Extract the detection/handling of tool output blocks (`[diff_block_start]`, file viewer blocks) into:
     ```rust
     fn compress_tool_block(lines: &[&str], current_index: usize) -> (usize, Vec<String>);
     ```
   - This cleans up the main loop of `compress_log_content` and isolates the concerns of the two compression algorithms.

4. **Add Unit Tests**:
   Create a comprehensive unit test suite in `src/backend/commands/utils.rs` (under a `#[cfg(test)]` module) validating:
   - Early return for files with < 300 lines.
   - Correct compression of long sequential `Compiling ` / `Checking ` dependency messages.
   - Truncation of tool output blocks exceeding 60 lines, ensuring boundary lines are preserved correctly.

---

## 3. Delegation Logic Refactoring (`execute_delegate`)

### Current Issues:
- `get_skills_injection` reads a directory, extracts keywords by splitting strings, manually filters stop words, parses files line-by-line looking for `name:` and `description:`, and matches them.
- Keyword extraction, file reading, parsing, and filtering are all inlined in a single deeply nested loop.
- Stop words (`this`, `that`, `with`, `from`) are hardcoded inside the logic.

### Proposed Changes:

1. **Structured Constant for Stop Words**:
   Define a static slice of stop words:
   ```rust
   const DELEGATE_STOP_WORDS: &[&str] = &["this", "that", "with", "from", "for", "and", "the"];
   ```

2. **Goal Keyword Extraction**:
   Extract keyword calculation logic from `get_skills_injection`:
   ```rust
   fn extract_goal_keywords(goal: &str) -> std::collections::HashSet<String>;
   ```

3. **Skill Metadata Parser**:
   Extract parsing logic from skill file contents:
   ```rust
   fn parse_skill_metadata(content: &str) -> Option<(String, String)>;
   ```

4. **Structured Prompt Builder**:
   Group prompt component generation into a struct `SubagentPromptBuilder` or clean helper functions that take explicit configurations, reducing the visual clutter in `execute_delegate`.

5. **Isolate State Updating**:
   Clearly separate the side-effect of CLI execution `spawn_agy_background` from database state management (`save_state`), ensuring clear boundary between execution success and state updates.

---

## 4. Verification and Evolution Safe Gate Plan

1. **Compile & Lint Check**: Confirm no compilation errors or warnings.
2. **Evolution Harness**: Ensure changes compile clean under `-- -D warnings` and tests pass using `agy-orchestrator evolution-harness`.
3. **Comment/Documentation Retention**: Maintain all existing documentation, system rule notices, and formatting comments as mandated by the `AGENTS.md` protocol.
