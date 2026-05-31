# cmdplz

**AI terminal command translator** — type in plain English, press `Ctrl+T`, get shell commands.

```
$ find all config files recursively
▸ Ctrl+T ▸
$ find . -name "config*" -type f
```

## One-line install

```bash
curl -fsSL https://raw.githubusercontent.com/iceberghbs/cmdplz/main/install.sh | bash
```

Downloads the prebuilt binary, adds it to PATH, and launches the interactive setup wizard.

## Quick start

```bash
# Interactive setup (choose provider, model, hotkey, etc.)
cmd-engine setup

# Start the daemon (or let setup auto-configure systemd/launchd/schtasks)
cmd-engine &

# Type natural language in any terminal, press Ctrl+T
```

## Supported providers

| Provider type | Examples |
|---------------|----------|
| OpenAI-compatible | OpenAI, Ollama, LM Studio, vLLM, Groq |
| Anthropic | Claude API |

## Supported shells

| Shell | Integration |
|-------|-------------|
| bash | Readline `bind -x` |
| zsh | `zle` widget + `bindkey` |
| fish | `bind` + `commandline` |
| PowerShell | PSReadLine key handler |

## Manual config

Create `~/.config/cmd-engine/config.json`:

```json
{
  "provider_type": "openai",
  "provider_url": "https://api.openai.com/v1",
  "api_key": "sk-...",
  "model": "gpt-4.1-mini",
  "daemon_port": 11435,
  "hotkey": "Ctrl+T",
  "session_history_lines": 10,
  "include_env_vars": true,
  "timeout_ms": 5000
}
```

## Build from source

```bash
git clone git@github.com:iceberghbs/cmdplz.git
cd cmdplz
cargo build --release
./target/release/cmd-engine setup
```

## License

AGPL-3.0