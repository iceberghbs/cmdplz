# AI Terminal Command Translator — Design Note

> **Concept**: Type natural language in the terminal prompt, press a hotkey,
> and the LLM replaces it with the corresponding shell command. No ghost text,
> no passive suggestions — on-demand translation only.

---

## 1. Motivation

Existing AI terminal tools (Copilot CLI, Warp, shell_gpt) require the user to
open a separate chat panel, type `?` or `#` prefixes, or launch a subprocess.
They break the terminal flow.

This tool works **inline in the prompt**: the user types what they want in
plain English, hits a hotkey, and the text is replaced by an executable
command. Review it, then press Enter.

---

## 2. User Flow

```
User types NL text             User presses Ctrl+Space
      │                                │
      ▼                                ▼
┌──────────────┐  ┌───────────┐  ┌─────────────────┐
│  Raw input   │──│Hook       │──│Show spinner     │
│  on prompt   │  │captures   │  │⏳ translating... │
└──────────────┘  │line+ctx   │  └────────┬────────┘
                  └───────────┘           │
                                          ▼
                                   ┌──────────────┐
                                   │ HTTP POST    │
                                   │ to daemon    │
                                   └──────┬───────┘
                                          │
                               ┌──────────┴──────────┐
                               │                     │
                               ▼                     ▼
                        ┌──────────────┐    ┌──────────────┐
                        │LLM returns   │    │LLM errors    │
                        │raw command   │    │(timeout/400) │
                        └──────┬───────┘    └──────┬───────┘
                               │                    │
                               ▼                    ▼
                        ┌──────────────┐    ┌──────────────┐
                        │Strip fences  │    │Show [ERR]    │
                        │Escape shell  │    │in buffer     │
                        └──────┬───────┘    └──────────────┘
                               │
                               ▼
                        ┌──────────────┐
                        │Replace buffer│
                        │with command  │
                        └──────┬───────┘
                               │
                               ▼
                        ┌──────────────┐
                        │ User reviews │
                        │ command,     │
                        │ hits Enter   │
                        └──────────────┘
```

---

## 3. Architecture

```
┌─────────────────────────────────────────────────────────┐
│                  TERMINAL EMULATOR                      │
│  (Windows Terminal / Alacritty / VS Code Terminal)      │
│                                                         │
│  ┌───────────────────────────────────────────────────┐  │
│  │                    SHELL                           │  │
│  │  (PowerShell / bash / zsh / fish)                  │  │
│  │                                                    │  │
│  │  $ find all files named config in this folder      │  │
│  │           ▲ user presses Ctrl+Space                │  │
│  │           │                                        │  │
│  │  ┌────────┴──────────────┐                        │  │
│  │  │   HOOK MODULE         │                        │  │
│  │  │  (PSReadLine plugin   │                        │  │
│  │  │   / bash widget)      │                        │  │
│  │  │                       │                        │  │
│  │  │  ┌──────────────────┐ │                        │  │
│  │  │  │ Output Handler   │ │                        │  │
│  │  │  │ - show spinner   │ │                        │  │
│  │  │  │   while waiting  │ │                        │  │
│  │  │  │ - shell-escape   │ │                        │  │
│  │  │  │   returned cmd   │ │                        │  │
│  │  │  │ - replace buffer │ │                        │  │
│  │  │  │   with command   │ │                        │  │
│  │  │  │ - or show [ERR]  │ │                        │  │
│  │  │  └──────────────────┘ │                        │  │
│  │  └───────────────────────┘                        │  │
│  │                                                    │  │
│  │  $ find . -name "config*"        ◄── replaced!     │  │
│  │              user reviews, presses Enter            │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
                         │
                         │ HTTP / gRPC
                         ▼
┌─────────────────────────────────────────────────────────┐
│               AI COMMAND ENGINE (daemon)                │
│                                                         │
│  ┌───────────────────────────────────────────────────┐  │
│  │  POST /translate                                   │  │
│  │                                                    │  │
│  │  Request:                                          │  │
│  │  {                                                 │  │
│  │    "input": "find all files named foo in bar",     │  │
│  │    "shell": "bash",                                │  │
│  │    "cwd": "/home/user/projects",                   │  │
│  │    "os": "linux",                                  │  │
│  │    "history": ["cd bar", "ls -la"]                 │  │
│  │  }                                                 │  │
│  │                                                    │  │
│  │  ┌─────────────┐    ┌───────────────────────┐      │  │
│  │  │ Prompt      │───▶│ LLM Provider          │      │  │
│  │  │ Builder     │    │                       │      │  │
│  │  │             │    │ ┌───────────────────┐ │      │  │
│  │  │ System:     │    │ │ OpenAI-compatible │ │      │  │
│  │  │ "You are a  │    │ │ /v1/chat/complete │ │      │  │
│  │  │  shell CLI  │    │ │ (Ollama, LMStudio,│ │      │  │
│  │  │  translator.│    │ │  OpenAI, etc.)    │ │      │  │
│  │  │  Output ONLY│    │ └───────────────────┘ │      │  │
│  │  │  the command│    │ ┌───────────────────┐ │      │  │
│  │  │  No markdown│    │ │ Anthropic         │ │      │  │
│  │  │  No explain"│    │ │ /v1/messages      │ │      │  │
│  │  └─────────────┘    │ └───────────────────┘ │      │  │
│  │                      └───────────┬───────────┘      │  │
│  │                                  │                  │  │
│  │                      ┌───────────┴───────────┐      │  │
│  │                      │ Post-processor         │      │  │
│  │                      │ - strip quotes/fences  │      │  │
│  │                      │ - shell-escape         │      │  │
│  │                      │ - validate single line │      │  │
│  │                      └───────────┬───────────┘      │  │
│  │                                  │                  │  │
│  │  Response:                       │                  │  │
│  │  { "command": "find ./bar -name 'foo*'" }          │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

---

## 4. Components

### 4.1 Shell Module (per-shell plugin)

Responsible for intercepting the hotkey, gathering context, calling the daemon,
and replacing the input buffer.

| Shell | Implementation |
|-------|---------------|
| **PowerShell** | PSReadLine plugin — `Set-PSReadLineKeyHandler Ctrl+SpaceBar { ... }` |
| **bash** | `bind -x '"\C- ": my_widget'` in `.bashrc` |
| **zsh** | `zle -N` widget registered via `bindkey` |
| **fish** | Custom `fish_key_reader` binding |

#### Hotkey trigger flow

1. User presses `Ctrl+Space`
2. Hook module replaces the buffer with `⏳ translating...` and fires HTTP request
   to the daemon in the background
3. On response:
   - **Success**: shell-escape the returned command (strip backtick fences,
     escape any `$()` that would be interpreted by the shell), replace buffer
     with the sanitized command
   - **Error**: replace buffer with `[ERR] <message>`, original input
     recoverable via shell undo (Ctrl+Z / Ctrl+_)

#### Shell escaping rules (per-target-shell)

| Shell | Escapes applied |
|-------|----------------|
| bash/zsh/fish | Strip \`\`\` fences, replace bare `$(` → `\$(\`, `\`` → `\\\`` |
| PowerShell | Strip \`\`\` fences (if any), `$` is safe in strings but escape backticks |

#### Context collected

| Field | Source |
|-------|--------|
| `input` | Current prompt buffer text |
| `shell` | `$SHELL` or detected shell name |
| `cwd` | `$PWD` or `(Get-Location).Path` |
| `os` | `uname -s` / `$env:OS` |
| `history` | Last N commands from this session — mixed human + AI-generated. Count controlled by `session_history_lines` in config |
| `env` | (Optional) key env vars: `VIRTUAL_ENV`, `CONDA_PREFIX`, etc. |

#### Session history tracking

The shell module maintains an in-memory list of the last N commands executed
in the current session, **regardless of origin** (typed manually or
AI-generated). No tagging of source — just a FIFO buffer of commands.

- On each Enter-press, the command (whether human or AI-translated) is pushed
  onto the history list
- The list is trimmed to the most recent `session_history_lines` entries
- Sent to the daemon as part of every `/translate` request

This gives the LLM enough context to chain related commands (e.g. noticing you
just `cd`-d into a directory and tailoring suggestions to it), without tagging
or separating AI commands from human ones.

### 4.2 Daemon (`cmd-engine`)

A lightweight HTTP service that wraps the LLM.

- **Protocol**: `POST /translate` — JSON in, JSON out
- **Startup**: Launched on demand; stays alive for session
- **Port**: Configurable, default `11435`

#### Prompt template

```
You are a shell command translator. Given a natural language request
and context below, output ONLY the shell command — no explanations,
no markdown, no backticks, no commentary.

Shell: {shell}
OS: {os}
Current directory: {cwd}
Recent commands:
{history}

Request: {input}

Command:
```

#### Post-processing

1. Strip leading/trailing whitespace and backtick fences
2. Strip `$ ` / `# ` prefixes if present
3. (Optional) sanity-check: ensure output is a single line

#### Response schema

```json
{
  "command": "find ./bar -name 'foo*'",
  "error": null
}
```

### 4.3 LLM Backend

**BYOK** — user brings their own API endpoint and key. No models are bundled or
pre-installed.

Two provider types supported:

| Provider   | API format        | Endpoint path used by daemon     |
|------------|-------------------|----------------------------------|
| `openai`   | OpenAI-compatible | `{provider_url}/chat/completions`|
| `anthropic`| Anthropic Messages| `{provider_url}/messages`        |

- **`openai`** — works with OpenAI, Ollama, LM Studio, vLLM, Groq, any
  OpenAI-compatible `/v1/chat/completions` endpoint
- **`anthropic`** — native Anthropic Messages API (`x-api-key` header,
  `anthropic-version: 2023-06-01`)
- If `provider_url` or `api_key` is blank, daemon returns:
  `[ERR] Config incomplete: missing provider_url or api_key`
- `api_key` can be `""` for local providers with no auth (Ollama, LM Studio)
- Model ID is user-specified in config; no default enforced

---

## 5. Config File

`~/.config/cmd-engine/config.json`:

```json
{
  "provider_type": "openai",
  "provider_url": "http://localhost:11434/v1",
  "api_key": "",
  "model": "qwen2.5:3b",
  "daemon_port": 11435,
  "hotkey": "Ctrl+Space",
  "session_history_lines": 10,
  "include_env_vars": true,
  "timeout_ms": 5000
}
```

| Field | Type | Purpose |
|-------|------|---------|
| `provider_type` | `"openai"` \| `"anthropic"` | Which API format to use |
| `provider_url` | string | Base URL of the LLM API |
| `api_key` | string | API key; `""` for local providers with no auth |
| `model` | string | Model ID (e.g. `gpt-4o`, `claude-sonnet-4-20250514`, `qwen2.5:3b`) |
| `daemon_port` | u16 | Port the daemon HTTP server binds to |
| `hotkey` | string | Key combo to trigger translation |
| `session_history_lines` | u8 | How many recent commands to include as context |
| `include_env_vars` | bool | Include key env vars (`VIRTUAL_ENV`, etc.) |
| `timeout_ms` | u32 | Max wait time for LLM response |

---

## 6. Non-Goals (v1)

- No ghost-text / inline autocomplete
- No command execution — user must press Enter manually
- No output streaming — command returned atomically
- No multi-line / piped command generation
- No chat mode
- No history learning / fine-tuning

---

## 7. Implementation Order

| Phase | Step | Shell | Est. effort |
|-------|------|-------|-------------|
| 1 | R Daemon scaffold (Axum) + config loader + `/translate` endpoint + OpenAI provider + Anthropic provider | — | Medium |
| 2 | bash widget (.bashrc integration) | bash | Small |
| 3 | zsh widget (.zshrc integration) | zsh | Small |
| 4 | Spinner UX + shell escaping | all | Small |
| 5 | PowerShell PSReadLine plugin | pwsh | Medium |
| 6 | Config validation + clear error messages | — | Small |
| 7 | Cross-platform installer script | — | Small |
| 8 | fish support | fish | Small |

---

## 8. Open Questions

*(None — all resolved in design above.)*

---

## 9. Rust Crate Stack

| Crate | Purpose |
|-------|---------|
| `axum` | HTTP server for the daemon |
| `tokio` | Async runtime |
| `serde` / `serde_json` | JSON serialization |
| `reqwest` | HTTP client to call LLM providers |
| `dirs` | Cross-platform config/data directory paths |
| `tracing` / `tracing-subscriber` | Structured logging |

`Cargo.toml` deps:

```toml
[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", features = ["json"] }
dirs = "5"
tracing = "0.1"
tracing-subscriber = "0.3"
```

---

## 10. Daemon API Contract

### `POST /translate`

**Request**:
```json
{
  "input": "find all files named foo in bar",
  "shell": "bash",
  "cwd": "/home/user/projects",
  "os": "linux",
  "history": ["cd bar", "ls -la"]
}
```

**Response (200)**:
```json
{
  "command": "find ./bar -name 'foo*'",
  "error": null
}
```

**Response (error)**:
```json
{
  "command": null,
  "error": "Config incomplete: missing provider_url or api_key"
}
```

### `GET /health`

Returns `200 OK` if daemon is running and config is valid.