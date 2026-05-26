

# 📅 History log from 2026-05-26 01:30:46 (Spawned at 2026-05-25T23:15:48+09:00)

# Completion Report: AI Secretary Premium UX Upgrade

We have upgraded the visual and usability aspects of the dashboard chat assistant to provide a premium, interactive personal secretary experience matching the Hermes Agent model.

## Accomplishments
1. **Interactive Markdown Parsing**: Built a fast, safe, dependency-free inline and block markdown parser in Dioxus to render headers, bullet lists, bold text, inline code, and clickable file pills natively.
2. **Suggested Quick Actions**: Integrated quick action chips (System Info, List Projects, Active Issues, Create Task) for streamlined JIT queries.
3. **Session Management**: Added a "Reset Session" button to clear backend session state and refresh UI history.
4. **Premium Aesthetics**: Upgraded styles with modern gradients, glowing SVG robot/user avatars, and smooth animations.

## Verification
- Code successfully passes Clippy warnings (`-D warnings`) and tests.



# 📅 History log from 2026-05-26 07:53:24 (Spawned at 2026-05-25T23:15:48+09:00)

# Completion Report: Chat Session Persistence & Tab-Switching Bug Fix

## Changes Implemented

1. **Frontend State Management (`src/frontend/app.rs`)**:
   - Lifted the chat history state (`chat_messages`) to the root `App` component so it is preserved when switching tabs.
   - Removed the wasteful 3-second background polling of chat history to eliminate race conditions and UI flickering.
   - Initialized history once on page mount and refreshed it on demand when clicking the "Chat Assistant" tab.

2. **History Parsing & Sanitization (`src/main.rs`)**:
   - Switched from `transcript.jsonl` to `transcript_full.jsonl` to prevent displaying truncated message content.
   - Corrected JSON key tracking from `timestamp` to `created_at` for accurate timing indicators.
   - Extracted user prompts from system prompts and metadata wrappers in transcript logs to keep chat bubbles clean.

3. **Compilation Enhancements (`src/backend/upgrade.rs`)**:
   - Fixed the `self-upgrade` tool to build native binaries using `--no-default-features --features server` to avoid runtime panics on non-wasm targets.

4. **Evolution Harness and Deployment**:
   - Verified changes via evolution-harness, auto-committed, and pushed to remote branch.
   - Successfully compiled the release binary, installed it, restarted the daemon service via systemd, and launched the upgraded dashboard (PID: 419244).



# 📅 History log from 2026-05-26 08:17:04 (Spawned at 2026-05-25T23:15:48+09:00)

# Completion Report: Multi-Room Chat Session Management

## Changes Implemented

1. **Backend API Features (`src/main.rs`)**:
   - Implemented helper functions `load_chat_sessions` and `save_chat_sessions` to store chat session metadata in `~/.agy_orchestrator/chat_sessions.json`.
   - Added `uuid_v4_fallback` function to generate unique session IDs without external dependencies.
   - Added `check_and_rename_session` to automatically rename a session from "New Chat" to a trimmed snippet of the user's first query message.
   - Refactored `get_chat_history` to retrieve a specific session's transcript from its folder under `~/.gemini/antigravity-cli/brain/<id>/.system_generated/logs/transcript_full.jsonl`.
   - Refactored `send_chat_message` to direct prompts to a specific session and support checking/renaming on the first message.
   - Added new server functions `get_chat_sessions`, `create_chat_session`, `delete_chat_session`, `get_active_session_id`, and `set_active_session_id` to fully expose session CRUD operations to the frontend.

2. **Frontend State & Signal Lifting (`src/frontend/app.rs`)**:
   - Lifted chat state variables (`active_session_id`, `chat_sessions`) to the `App` component level.
   - Configured `use_future` on mount to pull existing sessions and the active session ID, updating chat history dynamically.
   - Connected the "Chat Assistant" tab click event to fetch session states and load the active history.

3. **Frontend Component Redesign (`src/frontend/components/chat.rs`)**:
   - Transitioned `ChatTab` into a split-screen flex layout: a scrollable left sidebar for rooms list and room control (creation, selection, deletion) and a right pane for message stream.
   - Implemented "+ New Chat" button, selection highlights, date-time indicators, and room trash/deletion support (calling `delete_chat_session` and cleaning corresponding local brain folders).
   - Designed a friendly Empty State landing layout when no chat room is active.

## Verification
- Validated all compiler warning requirements and Clippy rules via `cargo clippy --all-targets -- -D warnings`.
- Verified and resolved collapsible matches warnings by restructuring destructuring patterns.
- Verified test suite passes successfully.
- Registered issue #33 on the evolution tracker, ran `evolution-harness` to commit and push changes, and marked it resolved.



# 📅 History log from 2026-05-26 08:58:33 (Spawned at 2026-05-25T23:15:48+09:00)

# Completion Report: Multi-Room Chat Session, Auto-Incrementing Dev Version & Dashboard Restart

## Description of Work
We have implemented and verified a highly robust set of features to align with the Hermes-Agent UX and fix operational upgrade issues:

1. **Multi-Room Chat Session Management**:
   - Implemented `create_chat_session`, `get_chat_sessions`, `delete_chat_session`, `get_active_session_id`, and `set_active_session_id` server functions.
   - Designed the **Draft-to-UUID Promotion** pattern: fresh chat sessions are generated as `draft-<timestamp>` on the frontend. Upon the first message, backend triggers `agy` without conversation flags. The newly created random UUID brain folder is auto-detected via sorting by recent modified directory, renaming the session ID to UUID in config files.
   - Refactored `src/frontend/components/chat.rs` with split flex-row layouts, a left sidebar for chat rooms, a Trash button for session deletion, and first-message auto-naming.
   - Programmatically validated this behavior via the integration test `test_multi_session_chat`.

2. **Auto-Incrementing Local Dev Version**:
   - Modified `build.rs` to track local dev compile count using the global `~/.agy_orchestrator/dev_build_number` file.
   - Appends `-dev<Count>` suffix to the version string (e.g. `v0.1.27-dev2`), enabling developers to distinguish consecutive local binary compilations.

3. **Automatic Dashboard Restart during Self-Upgrade**:
   - Added `restart_dashboard_process` in `src/backend/upgrade.rs` to scan `/proc` cmdlines for running `agy-orchestrator dashboard` processes.
   - Terminates old dashboard instances and spawns the upgraded dashboard on the previously used port with `setsid()` detached execution.

## Verification
- Run `cargo test --all-targets` -> 4 tests passed successfully, verifying the multi-session chat isolation.
- Completed evolution harness gates for Issue #33 and #34.
- Triggered `self-upgrade`, validating the dev counter increased to `dev2` and the old dashboard process was automatically killed and restarted on port 8080.



# 📅 History log from 2026-05-26 09:07:51 (Spawned at 2026-05-25T23:15:48+09:00)

# Completion Report: Fix missing transcript UUID session promotion bug

## Description of Work
We resolved a critical conversation failure bug that happened on existing UUID chat rooms that had empty/deleted transcript folders:

1. **Root Cause Analysis**:
   - When a user selected an existing chat session (with a UUID ID, thus `is_draft` is false) and sent a message, the system passed `--conversation <UUID>` to the `agy` CLI.
   - However, since the folder was empty or deleted, `agy` CLI rejected the pre-specified conversation ID, printed a warning, and generated a completely new random UUID folder.
   - Because `is_draft` was false, the backend skipped the UUID promotion/matching logic, trying to read the transcript file from the original requested folder.
   - This triggered the error: `Failed to retrieve agent response: Transcript file does not exist`.

2. **Resolution**:
   - Modified `send_chat_message` in `src/main.rs`.
   - Introduced `is_new_session = is_draft || !transcript_path.exists();`.
   - If a session is new or does not have a transcript file, we do not pass `--conversation` and enable the promotion logic.
   - After execution, backend reads the latest modified brain folder and renames the session ID mapping to match the newly generated UUID on the fly.
   - This keeps conversational flow working seamlessly even if folders are empty.

## Verification
- Run `cargo test --all-targets` -> 4 tests passed, confirming no regressions.
- Completed evolution harness and self-upgrade. Verified that the updated dashboard successfully restarted and is serving the fix on port 8080.



# 📅 History log from 2026-05-26 09:16:35 (Spawned at 2026-05-25T23:15:48+09:00)

# Completion Report: Isolate Loading State by Session & Prevents Cross-Session UI Updates

## Description of Work
We resolved a critical UX concurrency bug and fixed local dev version build tracking triggers:

1. **UX Concurrency Fix (HashMap-based loading & Active Session Filter)**:
   - Modified `src/frontend/components/chat.rs`.
   - Replaced the single-boolean `is_loading` state with a `HashMap<String, bool>` mapping each session ID to its loading state.
   - Now, if Room A is waiting for a response (loading), switching to Room B activates the send button instantly, as Room B is not loading.
   - Added an active-session match check (`if Some(active_id_spawn.clone()) == *active_session_id_ref.read()`) before pushing incoming AI responses to the current message stream signal. This prevents Room A's response from bleeding into Room B's view while Room B is active.

2. **Ensured build.rs rerun on source changes**:
   - Added `println!("cargo:rerun-if-changed=src/");` in `build.rs` to enforce recompilation of `build.rs` whenever any source file under `src/` changes.
   - Successfully verified that consecutive compilation commands now increment the dev version counters correctly (currently upgraded to `v0.1.27-dev6`).

## Verification
- Checked unit and integration tests -> all passed.
- Upgraded the binary and verified that dashboard (PID: 469340) restarted on port 8080.



# 📅 History log from 2026-05-26 09:28:16 (Spawned at 2026-05-25T23:15:48+09:00)

# Evolution Report: Chat Session Mapping and Simple Chat Latency Fixes

## Problem Solved
- Resolved the issue where the chat assistant failed to return responses due to a `Transcript file does not exist` error when creating/interacting with new rooms.
- Optimized response times for simple conversation queries (like hello, test) by stripping heavy system prompt wrappers.
- Fixed loading state isolation bugs where switching rooms left the send button disabled.

## Solution Architecture
1. **Unifying Promotion on Entry**: Introduced `promote_session_if_draft` to promote a temporary `draft-` session ID into a persistent `session-<TIMESTAMP>` ID immediately on entry of `send_chat_message`. This ensures both short-circuit custom commands and standard AI prompt executions use the same promoted ID.
2. **Excluding Drafts from Directory Resolution**: Updated the `ls -td` directory lookup parsing to apply `grep -v '/draft-'`, preventing it from incorrectly grabbing incomplete draft directories as active sessions.
3. **ChatResponse Return Struct**: Converted `send_chat_message` return type from `String` to `ChatResponse` containing the reply and `actual_session_id`.
4. **Active Session ID Sync**: Linked the frontend to immediately update `active_session_id` Signal with `response.actual_session_id` upon response, maintaining session highlight and continuity.
5. **Simple Chat Classifier & Fast-Path**: Strip down the 6KB+ system context wrapper for simple messages (<40 characters) lacking system execution keywords to improve TTFT and response times significantly.
6. **Clippy Warning Suppression**: Resolved target-dependent clippy warnings by appending `#[allow(dead_code)]` and fixing `to_string` formatting arg lints.

## Verification
- Unit test suite ran successfully (4 tests passed).
- CLI self-upgrade compiled in release and hot-reloaded the active systemd user service.
- Git auto-commit pushed to remote `main` branch under evolution issue #38.



# 📅 History log from 2026-05-26 09:42:06 (Spawned at 2026-05-25T23:15:48+09:00)

# Completion Report: Multi-Room Chat Session Management & TDD Verification

## Work Accomplished
1. **Resolved HashMap-based Message Caching Type Mismatches**: Fixed remaining type mismatch compilation issues in [chat.rs](file:///home/wimvm/works/agy_orchestrator/src/frontend/components/chat.rs) (line 421) by replacing the legacy `.set(Vec::new())` with the correct `.write().insert(new_id, Vec::new())` format.
2. **Fixed Borrow Checker & Lifetime Conflicts**: Resolved mutable double borrowing and lifetime issues within the HashMap migration block (around line 366) by extracting the draft removal into its own expression, releasing the write lock before modifying the HashMap entry.
3. **Cleaned Unused Variables and Mutability Warnings**:
   - Removed the unused `active_session_id_ref` variable in the reset session callback.
   - Removed the redundant `mut` keyword on `actual_session_id` in [src/main.rs](file:///home/wimvm/works/agy_orchestrator/src/main.rs).
4. **Verified via Evolution Harness**:
   - Ran `cargo clippy --all-targets -- -D warnings` and verified zero errors and warnings.
   - Ran `cargo test` and verified the concurrent `test_multi_session_chat` test passes successfully.
   - Executed `evolution-harness` to auto-commit and push the resolutions for issue #39 and issue #40 to GitHub.
5. **Deployed upgraded binary**: Ran `self-upgrade` to compile, package, and restart the systemd user service and dashboard process successfully on port 8080.



# 📅 History log from 2026-05-26 09:49:04 (Spawned at 2026-05-25T23:15:48+09:00)

# Completion Report: Resolved Session ID Duplicate Promotion Race Condition

## Work Accomplished
1. **Identified Race Condition Root Cause**: Found that when `agy` was executed for a new draft session, it generated a new random UUID. The orchestrator's directory scanner previously used `ls -td ... | head -n 1` to find the newly created directory. If another active session's folder had files written to it concurrently, its timestamp was updated, causing the scanner to return that folder instead. This led to multiple sessions sharing the same ID, causing them to select simultaneously and act as a single room.
2. **Directory Difference Mapping Implementation**:
   - Refactored `send_chat_message` in [src/main.rs](file:///home/wimvm/works/agy_orchestrator/src/main.rs) to capture the set of existing brain directories (`before_dirs`) *prior* to executing the CLI subprocess.
   - Captured the directories again (`after_dirs`) *after* completion.
   - Calculated the set difference (`after_dirs.difference(&before_dirs)`).
   - If there is a new directory, it resolves that specific UUID. If there are multiple new directories, it picks the latest modified among the difference.
   - Fell back to the `ls -td` logic if the difference set was empty (ensuring full backward compatibility).
3. **Backend Deduplication Safeguard**:
   - Updated `load_chat_sessions` in [src/main.rs](file:///home/wimvm/works/agy_orchestrator/src/main.rs) to automatically deduplicate the session list by their ID, keeping the first occurrence (active session with title) and discarding duplicate entries.
4. **Pruned JSON Duplicates**: Manually edited and cleaned up `chat_sessions.json` to remove the duplicate entries created during the race condition.
5. **OTA Upgraded & Deployed**: Rebuilt and deployed the updated binary on port `8080` via `self-upgrade`, confirming the daemon status remains active and healthy.



# 📅 History log from 2026-05-26 09:56:18 (Spawned at 2026-05-25T23:15:48+09:00)

# Evolution Report: Auto-Scroll to Bottom in Live Logs Tab

## Problem
In the live log viewer, when new events or updates arrive or when first entering the logs tab, the user had to manually scroll to the very bottom of the terminal viewer to see the latest log statements.

## Solution
1. **Added Element ID**: Assigned `id: "live-logs-container"` to the main log viewer terminal component in `src/frontend/components/logs.rs`.
2. **Added Scroll Effect**: Integrated a Dioxus `use_effect` hook inside the `LogsTab` component. The effect reads `logs` signal to trigger on new updates.
3. **Javascript Execution**: Runs a browser `eval` command with a slight delay (`setTimeout` of 50ms) to ensure Dioxus has updated the DOM, then scrolls `live-logs-container` to its maximum `scrollTop`.
4. **Validation**: Compiled and tested locally. Verified all test targets compile and pass successfully. Re-deployed the compiled release binary and restarted the local `agy-orchestrator` service daemon.



# 📅 History log from 2026-05-26 10:01:22 (Spawned at 2026-05-25T23:15:48+09:00)

# Evolution Report: Auto-Scroll to Bottom in Secretary Chat Tab

## Problem
1. The dashboard web page was not updating to the latest build after a refresh because the old dashboard process (PID 491151) remained running on port 8080 and served outdated assets, while the new daemon service was running but couldn't bind port 8080.
2. In the AI Secretary Chat tab, when first entering the tab, switching rooms, or when new messages were sent/received, the scroll container did not automatically scroll to the bottom, requiring manual scroll.

## Solution
1. **Chat Auto-Scroll ([chat.rs](file:///home/wimvm/works/agy_orchestrator/src/frontend/components/chat.rs))**:
   * Assigned `id: "chat-messages-container"` to the main message list div container.
   * Integrated a Dioxus `use_effect` hook inside the `ChatTab` component. The effect tracks changes to `active_session_id` and `messages` signals.
   * On any updates, it executes a browser `eval` command with a slight delay (`setTimeout` of 50ms) to ensure Dioxus updates the DOM, then scrolls the message container to the bottom.
2. **Dashboard Lifecycle Fix**:
   * Terminated the zombie dashboard process holding port 8080 (`kill -9 491151`).
   * Performed a complete Dioxus build (`dx build --release`) to generate both server binary and WASM/JS/CSS public assets.
   * Copied client public assets to `/home/wimvm/.local/bin/public` and the server binary to `/home/wimvm/.local/bin/agy-orchestrator`.
   * Restarted the systemd service and spawned the updated dashboard process on port 8080, successfully upgrading the running build version to `dev45`.
3. **Validation**: Verified build and tests pass successfully.



# 📅 History log from 2026-05-26 10:07:25 (Spawned at 2026-05-25T23:15:48+09:00)

# Evolution Report: Normalizing and Improving Chat Assistant UI/UX

## Problem
1. **Raw CLI Command Exposure**: The quick action buttons for JIT commands previously typed raw command strings like `env -u PORT -u ADDR -u IP agy-orchestrator list` or `/home/wimvm/.local/bin/agy-orchestrator info` directly into the chat prompt, which looked awkward and exposed command line details in the user's bubble.
2. **Environment Variable Panic Bug**: The server function `send_chat_message` set `PORT` to `"8080"` and `ADDR` to `"0.0.0.0"` in the child process's environment when executing subcommands. This caused subcommands (which are CLI binaries checking for `PORT` presence to determine if they are web servers) to think they were Dioxus web servers and panic with "Failed to bind to address 0.0.0.0:8080".
3. **Scrollbar discrepancy**: The Rooms list and Message Stream list containers did not have custom glassmorphic scrollbar styling, using the browser's default scrollbars instead.
4. **Synchronous Signal Read Race Condition**: Quick action buttons set `input_text` and then immediately called `send_message()`, which risked racing signal write propagation.

## Solution
1. **Subprocess Env Fix ([main.rs](file:///home/wimvm/works/agy_orchestrator/src/main.rs))**:
   * Removed setting `PORT` and `ADDR` in `send_chat_message` subcommand command execution.
   * Explicitly added `env_remove("PORT")`, `env_remove("ADDR")`, and `env_remove("IP")` so child CLI processes are cleanly isolated and run in standard CLI execution mode.
2. **Quick Action Prompt Cleanup & display formatting ([chat.rs](file:///home/wimvm/works/agy_orchestrator/src/frontend/components/chat.rs))**:
   * Refactored buttons to send simple, clean commands (e.g. `agy-orchestrator info`, `agy-orchestrator list`, `agy-orchestrator issue --list`).
   * Added rendering override logic in the message bubble list to parse user messages; if a user message matches these commands, it renders a beautiful human-readable text label (e.g. `📋 System Info`, `🔍 List Projects`, `🐛 Active Issues`) instead of raw text. This hides technical commands cleanly from the UI bubble log while preserving historical displays.
3. **Scrollbar styling**: Added `scrollbar-thin scrollbar-thumb-slate-800 scrollbar-track-transparent` classes to both Rooms List and Message list scrollable divs.
4. **Refactored send_message helper**: Created a `send_custom_message(text: String)` closure that accepts raw text, allowing quick action buttons to dispatch messages immediately without modifying/racing the input bar's signal.
5. **Validation**: Built the whole Dioxus bundle using `dx build --release`, deployed binary and assets, restarted the service daemon on port 8080, and verified with `cargo test`.



# 📅 History log from 2026-05-26 10:12:44 (Spawned at 2026-05-25T23:15:48+09:00)

# Evolution Report: Normalizing Chat Tab Layout and Interactions

## Problem
1. **Double Scrollbar and Hidden Input Bug**: The chat container had a fixed height of `h-[calc(100vh-12rem)]` and restricted width of `max-w-6xl mx-auto`. Because the parent container has vertical padding and scrolls independently, this caused double scrollbars and pushed the input bar off the bottom of the screen on desktop displays.
2. **Accidental Deletion Risk**: Clicking the trash icon on a room deleted the chat session instantly without any verification prompt, leading to risk of data loss.
3. **Friction in Chat Starting**: When switching rooms or entering the chat tab, the input text field did not auto-focus, forcing the user to manually click the input field every time.

## Solution
1. **Layout Normalization ([chat.rs](file:///home/wimvm/works/agy_orchestrator/src/frontend/components/chat.rs))**:
   * Removed `max-w-6xl mx-auto h-[calc(100vh-12rem)]` limits on the outer ChatTab div.
   * Applied `h-full w-full` so it fills the available viewport content area dynamically, matching the other full-width dashboard tabs. This aligns the input bar cleanly at the bottom and eliminates double scrollbar conflicts.
2. **Room Delete Confirmation**:
   * Added a client-side JS evaluation using `confirm('이 대화방을 정말로 삭제하시겠습니까?')`.
   * The spawn block waits for the dialog result asynchronously via `eval.recv::<bool>()`. The deletion is cancelled if the user declines.
3. **Auto-Focus input field**:
   * Assigned `id: "chat-input-field"` to the text input component.
   * Inside the existing messages/room change `use_effect` hook, added `document.getElementById('chat-input-field').focus()` alongside the bottom scroll logic. The cursor now automatically focuses the chat input on room changes and mounting.
4. **Validation**: Built and compiled via Dioxus CLI release build, deployed binary & assets, restarted the service daemon, and confirmed tests pass.



# 📅 History log from 2026-05-26 23:02:23 (Spawned at 2026-05-25T23:15:48+09:00)

# Desktop Layout & Scroll Normalization Report

## Changes Made
- **Main App Layout (`app.rs`)**: Modified the main tab content panel class to always be `flex-1 p-8 flex flex-col overflow-hidden h-full`. This removes the top-level scrollbar for the page, forcing each individual tab component to handle its own scrolling.
- **Projects Tab Layout (`projects.rs`)**: Modified the outer wrapper to use `h-full w-full overflow-hidden`. The page header is kept fixed using `shrink-0`, and the project card grid is wrapped in a `flex-1 overflow-y-auto` scrollable div.
- **Evolution Kanban Issues Layout (`issues.rs`)**: Adjusted the main page wrapper to `h-full w-full overflow-hidden` and column grid to `items-stretch flex-1 min-h-0 overflow-hidden`. Updated `KanbanColumn` to use `h-full min-h-0` flexbox with issues lists scrolling individually using `flex-1 overflow-y-auto` (replacing the hardcoded `max-h-[600px]`).
- **Knowledge Vault Layout (`vault.rs`)**: Resized the entire note explorer grid to use the full viewport (`h-full w-full overflow-hidden`). Set notes sidebar list and editor panel to `h-full min-h-0` flex layouts. The Markdown editor textarea now dynamically fills `flex-1` of the screen without clipping.
- **Live Logs Layout (`logs.rs`)**: Wrapped the component with `overflow-hidden` so that the log terminal stream accurately scrolls inside the terminal container box.

## Impact
- **No Double Scrollbars**: Desktop users will no longer experience vertical scrolling on both the main container and inner components.
- **Fixed Headers**: Component titles and action buttons (like "New Task", "New Issue", "Add Note") stay locked at the top of the viewport when scrolling lists.
- **True Desktop Feel**: The dashboard behaves like a native high-quality desktop web app, using screen height and width dynamically.



# 📅 History log from 2026-05-26 23:10:25 (Spawned at 2026-05-25T23:15:48+09:00)

# Completion Report: Desktop Layout Optimizations and Input Autofocus

## Completed Work
1. **App shell height normalization**: Set main wrapper to `h-full overflow-hidden` so that inner content fits the viewport.
2. **Projects Tab (`projects.rs`)**: Fixed header elements and placed projects grid inside scrollable `overflow-y-auto` container.
3. **Kanban Issues Tab (`issues.rs`)**: Column grid stretches to fill height (`items-stretch flex-1 min-h-0 overflow-hidden`), columns list scrolls independently, and column container heights adjust nicely.
4. **Knowledge Vault Tab (`vault.rs`)**: Layout set to `items-stretch h-full w-full overflow-hidden`, sidebar list is scrollable, and editor textarea occupies the full remaining space height (`flex-1`) dynamically.
5. **Live Logs Tab (`logs.rs`)**: Layout modified to `h-full w-full overflow-hidden` with scrollable terminal window height auto-scaling.
6. **Chat Assistant Tab (`chat.rs`)**: Fully polished with autofocus on chat input field and custom styled scrollbars.
7. **Connection Interruption Diagnosis**: Confirmed the temporary socket disconnection ("lock" issue) was caused by the self-upgrade daemon restart triggered during KST 23:01 to deploy the new build.

