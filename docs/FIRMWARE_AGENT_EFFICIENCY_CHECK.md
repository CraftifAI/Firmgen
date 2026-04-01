# Firmware Agent Efficiency Check

This document summarizes an efficiency review of the firmware/ESP32 agent: unnecessary LLM round-trips, use of wrapper vs direct CLI for ESP32 commands, and where time may be wasted.

---

## 1. ESP32 tools: wrapper vs direct CLI

**Finding: ESP32 tools are not "wrapper commands" that add meaningful extra time.**

- **Implementation**: All ESP32 tools use `IdfCommand` in `refact-agent/engine/src/tools/esp32_tools/idf_command.rs`, which:
  - Runs **direct CLI**:
    - `idf.py <subcommand>` (e.g. `idf.py build`, `idf.py reconfigure`, `idf.py flash`)
    - `python3 -m esptool <subcommand>` for device operations (e.g. `chip_id`, `flash_id`)
  - Uses `tokio::process::Command` (no shell, no extra process chain).
  - Sets only `IDF_PATH` (and optional env like `IDF_TARGET`); the rest of the environment is inherited (user is expected to have run `source $IDF_PATH/export.sh`).
- **Overhead**: Timeouts, project path, error parsing (ErrorParser), and suggested_actions. No extra "wrapper" process or repeated CLI invocations per tool call.

**Conclusion**: Keeping the current design is appropriate. Replacing these with a generic "run command" tool would lose structured diagnostics and suggested fixes and would push env/project handling onto the LLM, increasing prompt size and error-prone behavior.

---

## 2. Unnecessary or redundant LLM round-trips

### 2.1 ESP32-specific tools (main agent)

- **ESP32Analyze**: **Disabled** in `tools_list.rs` with an explicit comment that it used `subchat_single` and caused UI flicker; the main agent is intended to do build-fix directly. So there is **no** extra nested LLM call for build failure analysis. Good.
- **esp32_project, esp32_build, esp32_device, esp32_config, esp32_component**: None of these invoke subchat or any LLM; they only run CLI and return structured output. No redundant to-and-fro LLM from the tools themselves.

### 2.2 `parallel_execute` (parallel_execution_exp only)

`parallel_execute` is only present in `parallel_execution_exp`, not in the main refact-agent engine.

For a **single** `parallel_execute` invocation, the flow is:

| Phase            | LLM usage |
|------------------|-----------|
| 1. Decompose     | 1 call (`parallel_subtasks` tool) |
| 2. Research      | N research agents, each a **full subchat** (up to 6 tool-call rounds) |
| 3. Assemble      | 1 assembler **subchat** (up to 16 tool-call rounds) |
| 4. Build-fix      | Up to 3 build-fix **subchats** (each up to 6 rounds) |

So for one "non-trivial" feature we get: **1 + N×(multi-round) + 1×(multi-round) + up to 3** = many LLM round-trips. This is by design for complex, multi-part features.

**Mitigation already in place**: In `parallel_execution_exp/src/yaml_configs/customization_compiled_in.yaml`, the prompt restricts when to use it:

- "For **non-trivial multi-part** firmware features (e.g. BLE provisioning, multi-protocol stacks), call `parallel_execute` …"
- "For simple example/template, small localized change" the agent should **not** use `parallel_execute` and should implement directly with the main agent.

**Recommendation**: Keep this wording strong so the agent does not use `parallel_execute` for simple tasks (e.g. "add a button", "change a pin"). If you observe over-use, add an explicit line such as: "Do NOT use parallel_execute for single-file changes, pin/config tweaks, or adding one component."

### 2.3 Other tools that use subchat

- **search_semantic**: Uses `subchat_single` to summarize retrieved chunks before returning to the main agent. That is one extra LLM call per search. The tradeoff is smaller context in the main agent; it's a design choice rather than clear waste.
- **locate** (tool_locate_search): Uses `subchat` for multi-round planning; same tradeoff.
- **strategic_planning**, **create_memory_bank**, **generate_commit_message**, etc.: Subchat is used for dedicated tasks; not part of the normal ESP32 firmware loop.

---

## 3. Redundant CLI work (idf.py)

**Possible double reconfigure**:

- **esp32_config** (operations that touch sdkconfig.defaults) can run `idf.py reconfigure` when `reconfigure=true`.
- **esp32_build** (operation `build`) checks if `sdkconfig.defaults` is newer than `sdkconfig` and, if so, runs `idf.py reconfigure` **again** before `idf.py build`.

So in a flow like: "set CONFIG_X via esp32_config with reconfigure=true" → "esp32_build build", we can get **two** reconfigures. Reconfigure is typically on the order of tens of seconds; the second one is redundant.

**Recommendation**: In `esp32_build`, before auto-running reconfigure, optionally skip it if `sdkconfig` was modified very recently (e.g. within the last 60–120 seconds), indicating that a reconfigure was likely just run by esp32_config. This is an optional micro-optimization; the rest of the firmware agent does not show other redundant CLI patterns.

---

## 4. Summary table

| Area                         | Status / finding |
|-----------------------------|------------------|
| ESP32 tools vs direct CLI   | Tools run `idf.py` / `esptool` directly; no wasteful wrapper. |
| Extra LLM in ESP32 tools    | None; ESP32Analyze disabled to avoid nested subchat. |
| parallel_execute over-use   | Prompt limits to "non-trivial multi-part"; reinforce if needed. |
| Double reconfigure          | Can happen when esp32_config(reconfigure=true) then esp32_build(build); optional skip in esp32_build if sdkconfig just updated. |
| search_semantic subchat     | One extra LLM per search for summarization; tradeoff for context size. |

---

## 5. Why ESP32 build, flash, monitor feel slower than direct idf.py

Even though the tools run the same (or equivalent) commands, several things add **real or perceived** delay compared to running `idf.py` in your terminal.

### 5.1 Config load on first use (real delay)

- **Every first** ESP32 tool call in a session calls `get_config()`, which (1) tries **HTTP GET** to `REFACT_ESP32_CONFIG_URL` (default `http://localhost:8002/v1/esp32-config`) with 5s connect / 15s total timeout, then (2) on failure falls back to file `~/.cache/refact/esp32_tools.yaml`.
- So the **first** build/flash/monitor can add **up to 5–15 seconds** if the config API is slow or unreachable. After that, config is cached.
- **Fix**: Try file first when API URL is unset or default localhost (see §6).

### 5.2 Monitor: fixed duration + no streaming (real and perceived)

- **Direct CLI**: You run `idf.py monitor`, see output in real time, Ctrl+C when done (e.g. 5 seconds).
- **Tool today**: It does **not** use `idf.py monitor`. It runs a **custom pyserial script** that reads for a **fixed duration** (default **30 seconds**) and uses `cmd.output()` so you see **nothing** until the process exits.
- So monitor always waits the full duration (default 30s) and returns one blob. That is why it "takes more time."
- **Fix**: Use `IdfCommand::new("monitor")` with a timeout and a lower default duration (e.g. 10s) (see §6).

### 5.3 Agent round-trip (perceived delay)

- For build/flash, the actual `idf.py` process is the same as CLI. But the flow is: ask agent → LLM → tool runs → result → LLM responds. So there is an extra round-trip **before** the command starts and **after** it finishes, which makes it feel slower than typing `idf.py build` in the terminal.

---

## 6. Implemented / suggested code changes

1. **Skip redundant reconfigure in esp32_build** (implemented): If `sdkconfig` was modified in the last 90 seconds, skip auto-reconfigure before build.

2. **Config: try file first** (recommended): When `REFACT_ESP32_CONFIG_URL` is unset or default localhost, try `~/.cache/refact/esp32_tools.yaml` first; only call the API if the file is missing or invalid.

3. **Monitor: use idf.py monitor with timeout** (recommended): Replace the custom pyserial script with `IdfCommand::new("monitor")` and `timeout_secs(duration + 5)`, and lower the default duration (e.g. 10s).
