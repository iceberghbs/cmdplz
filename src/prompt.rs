use crate::TranslateRequest;

pub fn build_system_prompt() -> &'static str {
    "You are a shell command translator. Given a natural language request \
     and context below, output ONLY the shell command — no explanations, \
     no markdown, no backticks, no commentary. \
     Respond with the raw command text and nothing else."
}

pub fn build_user_prompt(req: &TranslateRequest) -> String {
    let mut prompt = String::new();
    prompt.push_str(&format!("Shell: {}\n", req.shell));
    prompt.push_str(&format!("OS: {}\n", req.os));
    prompt.push_str(&format!("Current directory: {}\n", req.cwd));

    if !req.history.is_empty() {
        prompt.push_str("Recent commands:\n");
        for cmd in &req.history {
            prompt.push_str(&format!("  {}\n", cmd));
        }
    }

    prompt.push('\n');
    prompt.push_str(&format!("Request: {}\n", req.input));
    prompt.push_str("\nCommand:");
    prompt
}