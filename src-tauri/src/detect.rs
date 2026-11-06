//! Sensitive-content detection over plain text.
//!
//! Every detector is a regex plus, wherever the format allows it, a real
//! validator (Luhn for cards, check digits for national IDs, mod-97 for IBAN,
//! calendar checks for dates embedded in IDs). Validators are what keep the
//! false-positive rate low enough that people trust the defaults.
//!
//! Positions are reported as *char* indices (Unicode scalar values), which is
//! what the OCR layer uses to map a match back to pixel boxes.

use regex::Regex;
use serde::Serialize;
use std::sync::LazyLock;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Finding {
    /// Stable machine id, e.g. "email", "card", "cn_id", "secret".
    pub kind: &'static str,
    /// Char index range [start, end) of the part that should be hidden.
    pub start: usize,
    pub end: usize,
    /// Partially masked preview, safe to show in the UI.
    pub preview: String,
    /// 3 = credentials / government IDs / cards, 2 = contact details, 1 = context.
    pub severity: u8,
}

struct Rule {
    kind: &'static str,
    severity: u8,
    re: Regex,
    /// Capture group that holds the sensitive part (0 = whole match).
    group: usize,
    /// Require that the match is not glued to other ASCII letters/digits.
    bounded: bool,
    validate: fn(&str) -> bool,
}

fn always(_: &str) -> bool {
    true
}

fn rule(kind: &'static str, severity: u8, pattern: &str, group: usize, bounded: bool, validate: fn(&str) -> bool) -> Rule {
    Rule { kind, severity, re: Regex::new(pattern).expect(kind), group, bounded, validate }
}

static RULES: LazyLock<Vec<Rule>> = LazyLock::new(|| {
    vec![
        // ---- credentials -------------------------------------------------
        rule("private_key", 3, r"-----BEGIN (?:[A-Z]+ )*PRIVATE KEY-----", 0, false, always),
        rule("api_key", 3, r"sk-ant-[A-Za-z0-9_\-]{20,}", 0, true, always),
        rule("api_key", 3, r"sk-(?:proj-|svcacct-|admin-)?[A-Za-z0-9_\-]{20,}", 0, true, has_digit_and_letter),
        rule("api_key", 3, r"gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,}", 0, true, always),
        rule("api_key", 3, r"(?:AKIA|ASIA)[0-9A-Z]{16}", 0, true, always),
        rule("api_key", 3, r"xox[abprs]-[A-Za-z0-9\-]{10,}", 0, true, always),
        rule("api_key", 3, r"AIza[0-9A-Za-z_\-]{35}", 0, true, always),
        rule("api_key", 3, r"(?:sk|rk|pk)_(?:live|test)_[A-Za-z0-9]{16,}", 0, true, always),
        rule("api_key", 3, r"hf_[A-Za-z0-9]{30,}", 0, true, always),
        rule("api_key", 3, r"glpat-[A-Za-z0-9_\-]{20,}", 0, true, always),
        rule("token", 3, r"eyJ[A-Za-z0-9_\-]{8,}\.eyJ[A-Za-z0-9_\-]{8,}\.[A-Za-z0-9_\-]{8,}", 0, true, always),
        rule(
            "secret",
            3,
            r#"(?i)(?:password|passwd|pwd|passcode|secret|token|api[_\- ]?key|access[_\- ]?key|client[_\- ]?secret|auth|密码|口令|密钥|验证码)["']?\s*[:=：]\s*["']?([^\s"',;，。]{4,})"#,
            1,
            false,
            not_placeholder,
        ),
        rule("secret", 3, r"[?&](?:token|key|apikey|api_key|sig|signature|access_token|auth|password|code|secret)=([^&\s#]{8,})", 1, false, not_placeholder),
        rule("connection", 3, r"[a-z][a-z0-9+.\-]*://[^\s:/@]+:([^\s@/]{3,})@", 1, false, not_placeholder),
        // ---- government IDs and money ------------------------------------
        rule("cn_id", 3, r"[1-9]\d{5}(?:19|20)\d{2}(?:0[1-9]|1[0-2])(?:0[1-9]|[12]\d|3[01])\d{3}[\dXx]", 0, true, valid_cn_id),
        rule("my_ic", 3, r"\d{6}-\d{2}-\d{4}|\d{12}", 0, true, valid_my_ic),
        rule("sg_nric", 3, r"[STFGM]\d{7}[A-Z]", 0, true, valid_sg_nric),
        rule("hk_id", 3, r"[A-Z]{1,2}\d{6}\s?\(?[0-9A]\)?", 0, true, valid_hk_id),
        rule("card", 3, r"\d(?:[ \-]?\d){12,18}", 0, true, valid_card),
        rule("iban", 3, r"[A-Z]{2}\d{2}(?: ?[A-Z0-9]{4}){2,7}(?: ?[A-Z0-9]{1,4})?", 0, true, valid_iban),
        rule("crypto", 2, r"0x[a-fA-F0-9]{40}|bc1[ac-hj-np-z02-9]{25,60}", 0, true, always),
        // ---- contact details ---------------------------------------------
        rule("email", 2, r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9\-]+(?:\.[A-Za-z0-9\-]+)*\.[A-Za-z]{2,}", 0, true, always),
        rule("phone", 2, r"(?:\+?86[ \-]?)?1[3-9]\d[ \-]?\d{4}[ \-]?\d{4}", 0, true, always),
        rule("phone", 2, r"(?:\+?60[ \-]?|0)1[0-9][ \-]?\d{3,4}[ \-]?\d{4}", 0, true, always),
        rule("phone", 2, r"\+\d{1,3}[ \-]?(?:\(\d{1,4}\)[ \-]?)?\d{2,4}[ \-]?\d{3,4}[ \-]?\d{3,4}", 0, true, enough_digits),
        rule("phone", 2, r"\(\d{3}\)\s?\d{3}[\-. ]\d{4}|\d{3}[\-.]\d{3}[\-.]\d{4}", 0, true, always),
        // ---- context -----------------------------------------------------
        rule("ip", 1, r"(?:25[0-5]|2[0-4]\d|1?\d?\d)(?:\.(?:25[0-5]|2[0-4]\d|1?\d?\d)){3}", 0, true, not_trivial_ip),
    ]
});

/// Scan one piece of text (an OCR line or a whole clipboard text).
pub fn scan(text: &str) -> Vec<Finding> {
    // byte offset -> char index
    let mut char_at = vec![0usize; text.len() + 1];
    let mut ci = 0;
    for (bi, ch) in text.char_indices() {
        for k in 0..ch.len_utf8() {
            char_at[bi + k] = ci;
        }
        ci += 1;
    }
    char_at[text.len()] = ci;

    let mut found: Vec<Finding> = Vec::new();
    for r in RULES.iter() {
        for caps in r.re.captures_iter(text) {
            let whole = caps.get(0).unwrap();
            let Some(m) = caps.get(r.group) else { continue };
            if r.bounded && !ascii_bounded(text, whole.start(), whole.end()) {
                continue;
            }
            let s = m.as_str();
            if !(r.validate)(s) {
                continue;
            }
            found.push(Finding {
                kind: r.kind,
                start: char_at[m.start()],
                end: char_at[m.end()],
                preview: mask(s),
                severity: r.severity,
            });
        }
    }
    // Keep the strongest finding where several overlap: higher severity,
    // then longer span, then earlier rule order (stable sort keeps it).
    found.sort_by(|a, b| b.severity.cmp(&a.severity).then((b.end - b.start).cmp(&(a.end - a.start))));
    let mut kept: Vec<Finding> = Vec::new();
    for f in found {
        if kept.iter().all(|k| f.end <= k.start || f.start >= k.end) {
            kept.push(f);
        }
    }
    kept.sort_by_key(|f| f.start);
    kept
}

/// Replace the given char ranges with "[REDACTED:kind]" markers.
pub fn mask_text(text: &str, ranges: &[(usize, usize, String)]) -> String {
    let mut ranges: Vec<_> = ranges.to_vec();
    ranges.sort_by_key(|r| r.0);
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    for (s, e, kind) in ranges {
        if s < i || e > chars.len() || s >= e {
            continue;
        }
        out.extend(&chars[i..s]);
        out.push_str(&format!("[REDACTED:{kind}]"));
        i = e;
    }
    out.extend(&chars[i..]);
    out
}

/// "sk-ant-api03-abcd…wxyz" style preview: enough to recognise, not enough to reuse.
pub fn mask(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    if n <= 4 {
        return "•".repeat(n);
    }
    let head = if n >= 12 { 4 } else { 2 };
    let tail = if n >= 12 { 3 } else { 1 };
    let mut out: String = chars[..head].iter().collect();
    out.push_str("••••");
    out.extend(&chars[n - tail..]);
    out
}

fn ascii_bounded(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    let glued = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
    !glued(before) && !glued(after)
}

fn digits(s: &str) -> Vec<u32> {
    s.chars().filter_map(|c| c.to_digit(10)).collect()
}

fn has_digit_and_letter(s: &str) -> bool {
    s.chars().any(|c| c.is_ascii_digit()) && s.chars().filter(|c| c.is_ascii_alphabetic()).count() > 3
}

fn enough_digits(s: &str) -> bool {
    let n = digits(s).len();
    (9..=15).contains(&n)
}

fn not_placeholder(s: &str) -> bool {
    let l = s.to_lowercase();
    const WORDS: [&str; 14] = [
        "xxxx", "****", "••••", "<", "your", "example", "placeholder", "changeme", "redacted", "none", "null", "undefined", "true", "false",
    ];
    if WORDS.iter().any(|w| l.contains(w)) || l.starts_with('$') || l.starts_with("{{") || l.starts_with("process.env") {
        return false;
    }
    true
}

fn not_trivial_ip(s: &str) -> bool {
    !matches!(s, "0.0.0.0" | "127.0.0.1" | "255.255.255.255") && !s.starts_with("255.")
}

fn valid_card(s: &str) -> bool {
    let d = digits(s);
    if !(13..=19).contains(&d.len()) || d.iter().all(|&x| x == d[0]) {
        return false;
    }
    // Separators, if any, must be consistent groups (e.g. "4111 1111 1111 1111").
    if s.contains(' ') && s.contains('-') {
        return false;
    }
    if !matches!(d[0], 2..=6) {
        return false;
    }
    let sum: u32 = d
        .iter()
        .rev()
        .enumerate()
        .map(|(i, &x)| if i % 2 == 1 { let y = x * 2; if y > 9 { y - 9 } else { y } } else { x })
        .sum();
    sum % 10 == 0
}

fn valid_date(yy: u32, mm: u32, dd: u32, full_year: Option<u32>) -> bool {
    if !(1..=12).contains(&mm) || dd == 0 {
        return false;
    }
    let year = full_year.unwrap_or(2000 + yy);
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    dd <= days[(mm - 1) as usize]
}

fn valid_cn_id(s: &str) -> bool {
    let c: Vec<char> = s.chars().collect();
    if c.len() != 18 {
        return false;
    }
    let num = |a: usize, b: usize| s[a..b].parse::<u32>().unwrap_or(0);
    if !valid_date(0, num(10, 12), num(12, 14), Some(num(6, 10))) {
        return false;
    }
    const W: [u32; 17] = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
    const CHECK: [char; 11] = ['1', '0', 'X', '9', '8', '7', '6', '5', '4', '3', '2'];
    let sum: u32 = c[..17].iter().zip(W).map(|(ch, w)| ch.to_digit(10).unwrap_or(0) * w).sum();
    CHECK[(sum % 11) as usize] == c[17].to_ascii_uppercase()
}

/// Malaysian MyKad: YYMMDD-PB-###G. PB is a birthplace code; the valid set is
/// published by the National Registration Department.
fn valid_my_ic(s: &str) -> bool {
    let d = digits(s);
    if d.len() != 12 {
        return false;
    }
    let n = |i: usize| d[i] * 10 + d[i + 1];
    let (yy, mm, dd, pb) = (n(0), n(2), n(4), n(6));
    if !valid_date(yy, mm, dd, None) {
        return false;
    }
    let pb_ok = matches!(pb, 1..=16 | 21..=59 | 60..=68 | 71..=72 | 74..=79 | 82..=93 | 98..=99);
    if !pb_ok {
        return false;
    }
    // Undashed 12-digit runs are common (order numbers...), so they also
    // need a plausible birth year to count.
    s.contains('-') || yy >= 30 || yy <= 26
}

fn valid_sg_nric(s: &str) -> bool {
    let c: Vec<char> = s.chars().collect();
    if c.len() != 9 {
        return false;
    }
    const W: [u32; 7] = [2, 7, 6, 5, 4, 3, 2];
    let mut sum: u32 = c[1..8].iter().zip(W).map(|(ch, w)| ch.to_digit(10).unwrap_or(0) * w).sum();
    match c[0] {
        'T' | 'G' => sum += 4,
        'M' => sum += 3,
        _ => {}
    }
    let r = (sum % 11) as usize;
    let table: &[char; 11] = match c[0] {
        'S' | 'T' => &['J', 'Z', 'I', 'H', 'G', 'F', 'E', 'D', 'C', 'B', 'A'],
        'F' | 'G' => &['X', 'W', 'U', 'T', 'R', 'Q', 'P', 'N', 'M', 'L', 'K'],
        'M' => &['K', 'L', 'J', 'N', 'P', 'Q', 'R', 'T', 'U', 'W', 'X'],
        _ => return false,
    };
    let idx = if c[0] == 'M' { 10 - r } else { r };
    table[idx] == c[8]
}

fn valid_hk_id(s: &str) -> bool {
    let core: String = s.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let c: Vec<char> = core.chars().collect();
    if !(c.len() == 8 || c.len() == 9) {
        return false;
    }
    let letters = c.len() - 7;
    let val = |ch: char| -> u32 { if ch.is_ascii_digit() { ch.to_digit(10).unwrap() } else { ch as u32 - 'A' as u32 + 10 } };
    let mut sum = if letters == 1 { 36 * 9 } else { 0 };
    let mut w = 9 - (if letters == 1 { 1 } else { 0 });
    for &ch in &c[..c.len() - 1] {
        sum += val(ch) * w;
        w -= 1;
    }
    let check = c[c.len() - 1];
    let cv = if check == 'A' { 10 } else { check.to_digit(10).unwrap_or(99) };
    (sum + cv) % 11 == 0
}

fn valid_iban(s: &str) -> bool {
    let v: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if !(15..=34).contains(&v.len()) {
        return false;
    }
    let rearranged = format!("{}{}", &v[4..], &v[..4]);
    let mut rem: u64 = 0;
    for ch in rearranged.chars() {
        let n = if ch.is_ascii_digit() { ch.to_digit(10).unwrap() as u64 } else if ch.is_ascii_uppercase() { ch as u64 - 'A' as u64 + 10 } else { return false };
        rem = if n >= 10 { (rem * 100 + n) % 97 } else { (rem * 10 + n) % 97 };
    }
    rem == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(t: &str) -> Vec<&'static str> {
        scan(t).into_iter().map(|f| f.kind).collect()
    }
    fn hidden(t: &str) -> Vec<String> {
        let chars: Vec<char> = t.chars().collect();
        scan(t).into_iter().map(|f| chars[f.start..f.end].iter().collect()).collect()
    }

    #[test]
    fn emails_and_phones() {
        assert_eq!(kinds("联系我：derrick.gow@example.com 谢谢"), vec!["email"]);
        assert_eq!(kinds("手机号码13812345678，请回电"), vec!["phone"]);
        assert_eq!(kinds("call +60 12-345 6789 now"), vec!["phone"]);
        assert_eq!(kinds("office (415) 555-0132"), vec!["phone"]);
        assert_eq!(kinds("WhatsApp 012-3456789"), vec!["phone"]);
    }

    #[test]
    fn credentials() {
        assert_eq!(kinds("ANTHROPIC_API_KEY=sk-ant-api03-AbCdEf0123456789abcdefGHIJ"), vec!["api_key"]);
        assert_eq!(kinds("token ghp_aBcDeF0123456789aBcDeF0123456789abcd"), vec!["api_key"]);
        assert_eq!(kinds("aws AKIAIOSFODNN7EXAMPLE x"), vec!["api_key"]);
        assert_eq!(hidden("password: hunter2!"), vec!["hunter2!"]);
        assert_eq!(hidden("密码：Abc12345"), vec!["Abc12345"]);
        assert_eq!(hidden("https://x.io/cb?code=abcdef123456&state=1"), vec!["abcdef123456"]);
        assert_eq!(hidden("postgres://admin:s3cretPw@db.internal:5432/app"), vec!["s3cretPw"]);
        assert!(kinds("password: ${DB_PASSWORD}").is_empty());
        assert!(kinds("api_key: your-api-key-here").is_empty());
        assert_eq!(kinds("-----BEGIN OPENSSH PRIVATE KEY-----"), vec!["private_key"]);
        assert_eq!(kinds("Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U"), vec!["token"]);
    }

    #[test]
    fn cards_need_luhn() {
        assert_eq!(kinds("Visa 4111 1111 1111 1111 exp 12/29"), vec!["card"]);
        assert_eq!(kinds("card 5500-0000-0000-0004"), vec!["card"]);
        assert!(!kinds("order 4111 1111 1111 1112").contains(&"card"));
        assert!(!kinds("tracking 1234567890123456").contains(&"card"));
    }

    #[test]
    fn national_ids() {
        assert_eq!(kinds("身份证号 11010519491231002X"), vec!["cn_id"]);
        assert!(kinds("身份证号 110105194912310021").is_empty());
        assert_eq!(kinds("IC: 900101-14-5678"), vec!["my_ic"]);
        assert!(kinds("IC: 901301-14-5678").is_empty(), "month 13 is not a date");
        assert_eq!(kinds("NRIC S1234567D"), vec!["sg_nric"]);
        assert!(kinds("NRIC S1234567A").is_empty());
        assert_eq!(kinds("HKID A123456(3)"), vec!["hk_id"]);
        assert_eq!(kinds("IBAN GB82 WEST 1234 5698 7654 32"), vec!["iban"]);
    }

    #[test]
    fn quiet_on_ordinary_text() {
        for t in [
            "版本 2.3.0 发布于 2026-11-06 21:30",
            "Order #A1029 shipped, total RM 129.90",
            "The meeting is at 10:30 in room 4B.",
            "commit 58cf8f9c3 merged into main",
            "localhost 127.0.0.1:8080",
        ] {
            assert!(scan(t).is_empty(), "false positive in {t:?}: {:?}", scan(t));
        }
    }

    #[test]
    fn masking_text() {
        let t = "mail a@b.co or call 13812345678";
        let ranges: Vec<_> = scan(t).into_iter().map(|f| (f.start, f.end, f.kind.to_string())).collect();
        assert_eq!(mask_text(t, &ranges), "mail [REDACTED:email] or call [REDACTED:phone]");
        assert_eq!(mask("sk-ant-api03-abcdefgh"), "sk-a••••fgh");
    }
}
