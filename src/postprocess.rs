pub fn postprocess(raw: &str) -> String {
    let mut cmd = raw.trim().to_string();

    if cmd.starts_with("```") {
        if let Some(after) = cmd.strip_prefix("```") {
            cmd = after.trim().to_string();
            if let Some(idx) = cmd.find("```") {
                cmd = cmd[..idx].to_string();
            }
        }
    }
    if cmd.ends_with("```") {
        cmd = cmd.trim_end_matches("```").to_string();
    }

    if let Some(stripped) = cmd.strip_prefix('`').and_then(|s| s.strip_suffix('`')) {
        cmd = stripped.to_string();
    }

    if cmd.starts_with("$ ") {
        cmd = cmd[2..].to_string();
    }

    cmd = cmd.trim().to_string();
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_backtick_fences() {
        assert_eq!(postprocess("```\nls -la\n```"), "ls -la");
    }

    #[test]
    fn strips_single_backticks() {
        assert_eq!(postprocess("`ls -la`"), "ls -la");
    }

    #[test]
    fn strips_dollar_prefix() {
        assert_eq!(postprocess("$ ls -la"), "ls -la");
    }

    #[test]
    fn trims_whitespace() {
        assert_eq!(postprocess("  ls -la  "), "ls -la");
    }

    #[test]
    fn handles_clean_input() {
        assert_eq!(postprocess("ls -la"), "ls -la");
    }
}