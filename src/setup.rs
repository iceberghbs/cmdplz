use colored::*;
use dialoguer::{Confirm, Input, Select};
use serde_json;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn run_setup() {
    println!("{}", "\n╔══════════════════════════════════════════╗".blue());
    println!("{}", "║     cmd-engine — Interactive Setup        ║".blue().bold());
    println!("{}", "╚══════════════════════════════════════════╝\n".blue());

    // Step 1 — provider type
    let provider_type = choose_provider_type();

    // Step 2 — provider URL
    let provider_url = choose_provider_url(&provider_type);

    // Step 3 — API key
    let api_key = ask_api_key(&provider_type);

    // Step 4 — model ID
    let model = ask_model();

    // Step 5 — hotkey
    let hotkey = choose_hotkey();

    // Step 6 — timeout + history
    let timeout_ms = ask_number("Timeout (ms)", 15000u64);
    let session_history_lines = ask_number("History lines to include", 10u64) as u8;

    let daemon_port = 11435u16;

    // Summary
    println!("\n{}", "──────────────────────────────────────────".blue());
    println!("{}", "  Configuration Summary".bold());
    println!("{}", "──────────────────────────────────────────".blue());
    println!("  Provider type : {}", provider_type);
    println!("  Provider URL  : {}", provider_url);
    println!("  API key       : {}", mask_key(&api_key));
    println!("  Model ID      : {}", model);
    println!("  Hotkey        : {}", hotkey);
    println!("  Timeout       : {}ms", timeout_ms);
    println!("  History lines : {}", session_history_lines);
    println!("  Daemon port   : {}", daemon_port);
    println!("{}", "──────────────────────────────────────────".blue());

    // Test connection
    if !Confirm::new()
        .with_prompt("Test connection with a sample translation?")
        .default(true)
        .interact()
        .unwrap_or(false)
    {
        println!("{} Skipping connection test.", "→".yellow());
    } else {
        test_connection(&provider_type, &provider_url, &api_key, &model, timeout_ms);
    }

    // Save config
    if !Confirm::new()
        .with_prompt("Save configuration?")
        .default(true)
        .interact()
        .unwrap_or(false)
    {
        println!("{} Setup cancelled — nothing saved.", "✗".red());
        return;
    }

    let config_json = format!(
        r#"{{
  "provider_type": "{}",
  "provider_url": "{}",
  "api_key": "{}",
  "model": "{}",
  "daemon_port": {},
  "hotkey": "{}",
  "session_history_lines": {},
  "include_env_vars": true,
  "timeout_ms": {}
}}"#,
        provider_type, provider_url, api_key, model, daemon_port, hotkey, session_history_lines, timeout_ms
    );

    let config_dir = config_dir_path();
    fs::create_dir_all(&config_dir).ok();
    let config_path = config_dir.join("config.json");
    fs::write(&config_path, &config_json).expect("Failed to write config");
    println!("{} Config saved to {}", "✓".green(), config_path.display());

    // Install shell hook (auto, no confirmation)
    let shell = detect_shell();
    install_hook(shell);
    println!("{} Hook installed for {} ({})", "✓".green(), shell, rc_path_for(shell));

    // Auto-start at boot
    if Confirm::new()
        .with_prompt("Auto-start daemon at boot?")
        .default(true)
        .interact()
        .unwrap_or(false)
    {
        install_auto_start();
        start_daemon_now();
        println!();
        println!("{} Setup complete.", "✓".green().bold());
        println!("  Daemon is running.");
        print_stop_start_commands();
    } else {
        println!();
        println!("{} Setup complete.", "✓".green().bold());
        println!("  Start daemon manually: {}", "./target/debug/cmd-engine &".bold());
    }

    println!(
        "\n  Press {} in any terminal to translate.",
        hotkey.bold()
    );
    println!();
}

fn choose_provider_type() -> String {
    let items = vec!["openai (OpenAI-compatible API)", "anthropic (Anthropic Messages API)"];
    let idx = Select::new()
        .with_prompt("Step 1/6 — Provider endpoint")
        .items(&items)
        .default(0)
        .interact()
        .unwrap_or(0);
    if idx == 0 { "openai".to_string() } else { "anthropic".to_string() }
}

fn choose_provider_url(provider_type: &str) -> String {
    let official = if provider_type == "openai" {
        "https://api.openai.com/v1"
    } else {
        "https://api.anthropic.com"
    };
    let items = vec![official.to_string(), "Custom URL".to_string()];

    let idx = Select::new()
        .with_prompt("Step 2/6 — Provider URL")
        .items(&items.iter().map(|s| s.as_str()).collect::<Vec<_>>())
        .default(0)
        .interact()
        .unwrap_or(0);

    if idx == 1 {
        Input::<String>::new()
            .with_prompt("Enter custom URL")
            .interact()
            .unwrap_or_default()
    } else {
        items[0].to_string()
    }
}

fn ask_api_key(provider_type: &str) -> String {
    let default_empty = provider_type == "openai";
    if default_empty {
        Input::<String>::new()
            .with_prompt("Step 3/6 — API key (leave empty for local/no-auth)")
            .allow_empty(true)
            .default(String::new())
            .interact()
            .unwrap_or_default()
    } else {
        Input::<String>::new()
            .with_prompt("Step 3/6 — API key (required for Anthropic)")
            .interact()
            .unwrap_or_default()
    }
}

fn ask_model() -> String {
    Input::<String>::new()
        .with_prompt("Step 4/6 — Model ID")
        .interact()
        .unwrap_or_default()
}

fn choose_hotkey() -> String {
    let items = vec!["Ctrl+T", "Ctrl+G", "Ctrl+/", "Custom"];
    let idx = Select::new()
        .with_prompt("Step 5/6 — Hotkey")
        .items(&items)
        .default(0)
        .interact()
        .unwrap_or(0);
    if idx == items.len() - 1 {
        Input::<String>::new()
            .with_prompt("Enter custom hotkey (e.g. Ctrl+E)")
            .default("Ctrl+T".to_string())
            .interact()
            .unwrap_or_else(|_| "Ctrl+T".to_string())
    } else {
        items[idx].to_string()
    }
}

fn ask_number<T: std::str::FromStr + std::fmt::Display>(label: &str, default: T) -> T {
    Input::<String>::new()
        .with_prompt(format!("Step 6/6 — {}", label))
        .default(default.to_string())
        .interact()
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn mask_key(key: &str) -> String {
    if key.is_empty() {
        return "(none)".dimmed().to_string();
    }
    if key.len() <= 8 {
        return "••••".to_string();
    }
    format!("{}••••{}", &key[..4], &key[key.len() - 4..])
}

fn config_dir_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("cmd-engine")
}

fn detect_shell() -> &'static str {
    if std::env::var("ZSH_VERSION").is_ok() { return "zsh"; }
    if std::env::var("BASH_VERSION").is_ok() { return "bash"; }
    if std::env::var("FISH_VERSION").is_ok() { return "fish"; }
    let shell = std::env::var("SHELL").unwrap_or_default();
    if shell.contains("zsh")  { return "zsh"; }
    if shell.contains("bash") { return "bash"; }
    if shell.contains("fish") { return "fish"; }
    if shell.contains("pwsh") { return "pwsh"; }
    "bash"
}

fn rc_path_for(shell: &str) -> String {
    match shell {
        "bash" => format!("{}/.bashrc", std::env::var("HOME").unwrap_or_default()),
        "zsh"  => format!("{}/.zshrc", std::env::var("HOME").unwrap_or_default()),
        "fish" => format!("{}/.config/fish/config.fish", std::env::var("HOME").unwrap_or_default()),
        "pwsh" => {
            if cfg!(windows) {
                "Documents\\PowerShell\\Microsoft.PowerShell_profile.ps1".to_string()
            } else {
                format!("{}/.config/powershell/Microsoft.PowerShell_profile.ps1",
                    std::env::var("HOME").unwrap_or_default())
            }
        }
        _      => format!("{}/.bashrc", std::env::var("HOME").unwrap_or_default()),
    }
}

fn install_hook(shell: &str) {
    let rc_path = rc_path_for(shell);
    let hook = hook_snippet(shell);

    // Check if already installed
    if let Ok(existing) = fs::read_to_string(&rc_path) {
        if existing.contains("__cmd_engine_translate") {
            return; // already installed
        }
    }

    // Ensure parent dir exists
    if let Some(parent) = PathBuf::from(&rc_path).parent() {
        fs::create_dir_all(parent).ok();
    }

    let mut file = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&rc_path)
        .expect("Failed to open shell RC file");

    writeln!(file, "\n{}", hook).expect("Failed to write hook");
}

fn hook_snippet(shell: &str) -> &'static str {
    match shell {
        "bash" | "zsh" => {
r#"# cmd-engine — AI terminal command translator
__cmd_engine_translate() {
  local input="$READLINE_LINE"
  [ -z "$input" ] && return

  printf '\n⏳ translating...\n'

  local resp=$(curl -s --max-time 20 -X POST http://127.0.0.1:11435/translate \
    -H "Content-Type: application/json" \
    -d "$(jq -n \
      --arg input "$input" \
      --arg shell "bash" \
      --arg cwd "$PWD" \
      --arg os "$(uname -s)" \
      '{input: $input, shell: $shell, cwd: $cwd, os: $os, history: []}')")

  local cmd=$(echo "$resp" | jq -r '.command // empty')
  local err=$(echo "$resp" | jq -r '.error // empty')

  if [ -n "$cmd" ]; then
    READLINE_LINE="$cmd"
    READLINE_POINT="${#cmd}"
  elif [ -n "$err" ]; then
    printf '[ERR] %s\n' "$err"
  fi
}"#
        }
        "fish" => {
r#"function __cmd_engine_translate
  set -l input (commandline)
  test -z "$input"; and return

  echo ""
  echo "⏳ translating..."

  set -l resp (curl -s --max-time 20 -X POST http://127.0.0.1:11435/translate \
    -H "Content-Type: application/json" \
    -d (jq -n --arg input "$input" --arg shell "fish" --arg cwd "$PWD" \
      --arg os (uname -s) '{input: $input, shell: $shell, cwd: $cwd, os: $os, history: []}'))

  set -l cmd (echo "$resp" | jq -r '.command // empty')
  set -l err (echo "$resp" | jq -r '.error // empty')

  if test -n "$cmd"
    commandline "$cmd"
  else if test -n "$err"
    echo "[ERR] $err"
  end
end
bind \ct __cmd_engine_translate"#
        }
        "pwsh" => {
r#"# cmd-engine — AI terminal command translator
Set-PSReadLineKeyHandler -Key Ctrl+T -ScriptBlock {
  $input = $null
  [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$input, [ref]$null)
  if (-not $input) { return }
  Write-Host "`n⏳ translating..."
  $body = @{
    input   = $input
    shell   = "pwsh"
    cwd     = (Get-Location).Path
    os      = if ($IsWindows) { "windows" } else { "linux" }
    history = @()
  } | ConvertTo-Json
  try {
    $resp = Invoke-RestMethod -Uri "http://127.0.0.1:11435/translate" `
      -Method Post -Body $body -ContentType "application/json" -TimeoutSec 20
    if ($resp.command) {
      [Microsoft.PowerShell.PSConsoleReadLine]::Replace(0, $input.Length, $resp.command)
    } elseif ($resp.error) {
      Write-Host "[ERR] $($resp.error)"
    }
  } catch {
    Write-Host "[ERR] Failed to reach daemon at http://127.0.0.1:11435"
  }
}"#
        }
        _ => "",
    }
}

fn test_connection(provider_type: &str, provider_url: &str, api_key: &str, model: &str, timeout_ms: u64) {
    use std::process::Command;

    println!("{} Testing connection...", "→".yellow());

    let daemon_bin = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("./target/debug/cmd-engine"));

    // Write temp config for this test
    let tmp_config = format!(
        r#"{{
  "provider_type": "{}",
  "provider_url": "{}",
  "api_key": "{}",
  "model": "{}",
  "daemon_port": 11435,
  "hotkey": "Ctrl+T",
  "session_history_lines": 10,
  "include_env_vars": true,
  "timeout_ms": {}
}}"#,
        provider_type, provider_url, api_key, model, timeout_ms
    );

    let tmp_dir = config_dir_path();
    fs::create_dir_all(&tmp_dir).ok();
    let tmp_path = tmp_dir.join("config.json");
    fs::write(&tmp_path, &tmp_config).ok();

    // Kill daemon if already running on 11435, but NOT ourselves
    let in_use = std::process::Command::new("ss")
        .args(["-tlnp", "sport", "=:11435"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("11435"))
        .unwrap_or(false);

    if in_use {
        let our_pid = std::process::id();
        let _ = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("ss -tlnp 'sport = :11435' | grep -oP 'pid=\\K\\d+' | grep -v {} | xargs -r kill 2>/dev/null", our_pid))
            .output();
    }

    // Start daemon on a different port to avoid conflict
    let child = Command::new(&daemon_bin)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    match child {
        Ok(child) => {
            let pid = child.id();
            std::thread::sleep(std::time::Duration::from_secs(2));

            let output = Command::new("curl")
                .args([
                    "-s", "--max-time", "15",
                    "-X", "POST",
                    "http://127.0.0.1:11435/translate",
                    "-H", "Content-Type: application/json",
                    "-d", r#"{"input":"hello world","shell":"bash","cwd":"/tmp","os":"linux","history":[]}"#,
                ])
                .output()
                .unwrap_or_else(|_| std::process::Output { status: Default::default(), stdout: vec![], stderr: vec![] });

            // Kill our child daemon
            let _ = Command::new("kill").arg(pid.to_string()).output();
            std::thread::sleep(std::time::Duration::from_millis(500));

            if let Ok(resp) = String::from_utf8(output.stdout) {
                let cmd = resp.trim();
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(cmd) {
                    if let Some(c) = parsed["command"].as_str() {
                        println!("{} Connection OK — sample response:", "✓".green());
                        println!("    Input:  \"hello world\"");
                        println!("    Output: \"{}\"", c);
                        return;
                    }
                    if let Some(e) = parsed["error"].as_str() {
                        println!("{} Connection failed: {}", "✗".red(), e);
                        return;
                    }
                }
            }
            println!("{} Connection failed — no response from daemon.", "✗".red());
        }
        Err(e) => {
            println!("{} Cannot start daemon for test: {}", "✗".red(), e);
        }
    }
}

fn install_auto_start() {
    if cfg!(target_os = "linux") {
        install_systemd();
    } else if cfg!(target_os = "macos") {
        install_launchd();
    } else if cfg!(target_os = "windows") {
        install_schtask();
    }
}

fn print_stop_start_commands() {
    if cfg!(target_os = "linux") {
        println!("  Stop:  {} {}", "systemctl stop cmd-engine".bold(), "(manual control)".dimmed());
        println!("  Start: {} {}", "systemctl start cmd-engine".bold(), "(manual control)".dimmed());
        println!("  Logs:  {} {}", "journalctl -u cmd-engine -f".bold(), "(tail logs)".dimmed());
    } else if cfg!(target_os = "macos") {
        let plist = "~/Library/LaunchAgents/com.cmdplz.cmd-engine.plist";
        println!("  Stop:  launchctl unload {}", plist);
        println!("  Start: launchctl load {}", plist);
    } else if cfg!(target_os = "windows") {
        println!("  Stop:  {} {}", "schtasks /end /tn cmd-engine".bold(), "(manual control)".dimmed());
        println!("  Start: {} {}", "schtasks /run /tn cmd-engine".bold(), "(manual control)".dimmed());
    }
}

fn install_systemd() {
    let service = r#"[Unit]
Description=cmd-engine — AI Terminal Command Translator Daemon
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=DAEMON_BIN_PLACEHOLDER
Restart=on-failure
RestartSec=5
Environment=RUST_LOG=info
StandardOutput=append:LOG_DIR_PLACEHOLDER/cmd-engine.log
StandardError=append:LOG_DIR_PLACEHOLDER/cmd-engine.log

[Install]
WantedBy=multi-user.target
"#;

    let daemon_bin = std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("/root/trmnl-cmplt-agent/target/debug/cmd-engine"));
    let log_dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"));

    let service = service
        .replace("DAEMON_BIN_PLACEHOLDER", &daemon_bin.display().to_string())
        .replace("LOG_DIR_PLACEHOLDER", &log_dir.display().to_string());

    let svc_path = "/etc/systemd/system/cmd-engine.service";
    fs::write(svc_path, &service).expect("Failed to write systemd service file (need root?)");

    let _ = std::process::Command::new("systemctl")
        .args(["daemon-reload"])
        .output();
    let _ = std::process::Command::new("systemctl")
        .args(["enable", "cmd-engine.service"])
        .output();

    println!("{} Systemd service installed and enabled.", "✓".green());
}

fn install_launchd() {
    let daemon_bin = std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("/usr/local/bin/cmd-engine"));

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.cmdplz.cmd-engine</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{}/Library/Logs/cmd-engine.log</string>
    <key>StandardErrorPath</key>
    <string>{}/Library/Logs/cmd-engine.log</string>
</dict>
</plist>"#,
        daemon_bin.display(),
        std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()),
        std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()),
    );

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let launch_agents = PathBuf::from(&home).join("Library").join("LaunchAgents");
    fs::create_dir_all(&launch_agents).ok();
    let plist_path = launch_agents.join("com.cmdplz.cmd-engine.plist");
    fs::write(&plist_path, &plist).expect("Failed to write launchd plist");

    let _ = std::process::Command::new("launchctl")
        .args(["load", "-w"])
        .arg(&plist_path)
        .output();

    println!("{} Launchd service installed and loaded.", "✓".green());
}

fn install_schtask() {
    let daemon_bin = std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("cmd-engine.exe"));

    let _ = std::process::Command::new("schtasks")
        .args([
            "/create",
            "/tn", "cmd-engine",
            "/tr", &daemon_bin.display().to_string(),
            "/sc", "onlogon",
            "/f",
        ])
        .output();

    println!("{} Scheduled task registered (runs on logon).", "✓".green());
}

fn start_daemon_now() {
    if cfg!(target_os = "linux") {
        let _ = std::process::Command::new("systemctl")
            .args(["start", "cmd-engine.service"])
            .output();
        std::thread::sleep(std::time::Duration::from_secs(2));
    } else if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let plist = PathBuf::from(&home)
            .join("Library")
            .join("LaunchAgents")
            .join("com.cmdplz.cmd-engine.plist");
        let _ = std::process::Command::new("launchctl")
            .args(["load", "-w"])
            .arg(&plist)
            .output();
        std::thread::sleep(std::time::Duration::from_secs(2));
    } else if cfg!(target_os = "windows") {
        let _ = std::process::Command::new("schtasks")
            .args(["/run", "/tn", "cmd-engine"])
            .output();
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    println!("{} Daemon started.", "✓".green());
}