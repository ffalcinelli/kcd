with open("src/utils/secrets.rs", "r") as f:
    content = f.read()

content = content.replace("""fn obfuscate_string(s: &str) -> String {
    if s.is_empty() {
        return s.to_string();
    }

    let len = s.len();
    if len <= 3 {
        return "***".to_string();
    }
    let first = &s[0..1];
    let last = &s[len - 1..len];
    format!("{}***{}", first, last)
}""", """fn obfuscate_string(s: &str) -> String {
    if s.is_empty() {
        return s.to_string();
    }

    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= 3 {
        return "***".to_string();
    }
    let first = chars[0];
    let last = chars[chars.len() - 1];
    format!("{}***{}", first, last)
}""")

with open("src/utils/secrets.rs", "w") as f:
    f.write(content)
