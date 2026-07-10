//! Safe system-browser launching for interactive auth flows.
//!
//! Both the external-browser SSO flow and the OAuth authorization-code
//! flow open an IdP-supplied URL in the user's system browser. On WSL
//! (Windows Subsystem for Linux) the default launch path can route the URL
//! through a Windows shell interpreter, which is unsafe when the URL comes
//! from an untrusted party. See SNOW-3649282 for the mechanism.
//!
//! Two layers of defense live here:
//!
//! 1. [`open_url`] launches the URL on WSL via a direct browser/protocol
//!    handler (`explorer.exe`, falling back to `wslview`) rather than the
//!    shell path, so the URL is treated as an opaque navigation target
//!    instead of a command. This is the primary fix and does not reject any
//!    legitimate URL. Non-WSL platforms defer to the `webbrowser` crate.
//! 2. [`validate_browser_url`] additionally rejects URLs that are not
//!    `https://`, or that contain a character unsafe to pass to a launcher.
//!    This is transport-independent defense-in-depth.
//!
//! ### Why validation is a targeted blocklist, not the ticket's full set
//!
//! SNOW-3649282 lists several metacharacters as candidates to reject. But
//! real SAML/OAuth `ssoUrl`s carry multi-parameter query strings
//! (`?SAMLRequest=…&RelayState=…`) and percent-encoding — so `&`, `(`,
//! `)`, and `%` occur in perfectly legitimate URLs. The reference
//! `snowflake-connector-python` `is_valid_url` allowlist confirms this:
//! it *permits* `& ( ) %` and only *rejects* `| < > ! ^`. Blanket-
//! rejecting the full set would break genuine SSO logins on every
//! platform. The WSL transport bypass (layer 1) already makes `& ( ) %`
//! safe on the only platform of concern, so validation only rejects the
//! subset that is both risky and never legitimate in a URL.
//!
//! On top of `| < > ! ^`, validation also rejects their **percent-encoded**
//! forms (`%7C` … `%5E`) so an encoded payload cannot slip past the literal
//! scan, and characters that are never valid unencoded in a URL or that
//! could perturb argument parsing at the launcher — quotes, backtick,
//! backslash, and raw whitespace / control bytes. Encoded whitespace
//! (`%20`) stays legal, so this does not false-reject.
//!
//! ### Launcher ordering: `explorer.exe` before `wslview`
//!
//! `explorer.exe` is tried first: it hands the URL to the Windows protocol
//! handler (which opens the user's default browser) with no interpreter in
//! the chain. `wslview` (from the `wslu` package) is the fallback — it also
//! resolves the default browser, but is a shell script that may invoke a
//! Windows tool internally, so it carries a small third-party-trust surface.
//! Validation runs before any launcher, so ordering `explorer.exe` first
//! simply removes the trust question from the common path.

/// Characters that are unsafe to pass to a launcher and are **also** never
/// legitimate in a URL, so rejecting them is safe on every platform —
/// exactly the set the reference connector's `is_valid_url` allowlist also
/// excludes. See SNOW-3649282.
///
/// `&`, `(`, `)`, and `%` are deliberately **absent**: they occur in
/// real SSO URLs and are made safe on WSL by [`open_url`]. See the
/// module docs.
const SHELL_METACHARS: &[char] = &['|', '<', '>', '!', '^'];

/// Percent-encodings (lowercase) of [`SHELL_METACHARS`], rejected so an
/// encoded payload (e.g. `%7C` for `|`) cannot slip past the literal scan.
/// Encoded whitespace such as `%20` is intentionally NOT here — encoded
/// spaces are legal in URLs and must not be false-rejected.
const ENCODED_SHELL_METACHARS: &[&str] = &["%7c", "%3c", "%3e", "%21", "%5e"];

/// Characters rejected because they perturb Windows argv splitting
/// (`CommandLineToArgvW`) or are never valid **unencoded** in a URL:
/// quotes, backtick, and backslash. (Whitespace / control bytes are
/// handled separately via [`char::is_ascii_control`].)
const FORBIDDEN_LITERAL_CHARS: &[char] = &['"', '\'', '`', '\\'];

/// Reject a URL that is unsafe to hand to a system-browser launcher.
///
/// A URL is accepted only if it uses the `https://` scheme (case-
/// insensitive) and contains none of: [`SHELL_METACHARS`], their
/// [`ENCODED_SHELL_METACHARS`] forms, [`FORBIDDEN_LITERAL_CHARS`], or raw
/// ASCII whitespace / control bytes. Returns a short human-readable reason
/// on rejection; the reason deliberately does not echo the whole URL.
pub(crate) fn validate_browser_url(url: &str) -> Result<(), String> {
    // Scheme names are case-insensitive (RFC 3986 §3.1), so `HTTPS://` is a
    // legitimate equivalent of `https://` and must not be false-rejected.
    if !url
        .get(..8)
        .is_some_and(|p| p.eq_ignore_ascii_case("https://"))
    {
        // Truncate at the first `?`/`#` so the error can never echo query or
        // fragment content (which may carry tokens); then cap the length.
        let sanitized = url.split(['?', '#']).next().unwrap_or(url);
        return Err(format!(
            "URL must use https scheme, got: {}",
            sanitized.chars().take(50).collect::<String>()
        ));
    }
    // Literal scan: shell operators, quoting/argv-splitting chars, and any
    // raw whitespace or control byte (a raw space would split argv on the
    // WSL join; `%20` is fine and handled as literal `%`,`2`,`0`).
    if let Some(bad) = url.chars().find(|c| {
        SHELL_METACHARS.contains(c)
            || FORBIDDEN_LITERAL_CHARS.contains(c)
            || c.is_ascii_control()
            || *c == ' '
    }) {
        return Err(format!(
            "URL contains forbidden character {bad:?}; refusing to open browser"
        ));
    }
    // Encoded scan: catch percent-encoded shell operators (e.g. `%7C`).
    let lower = url.to_ascii_lowercase();
    if let Some(seq) = ENCODED_SHELL_METACHARS.iter().find(|s| lower.contains(**s)) {
        return Err(format!(
            "URL contains percent-encoded shell metacharacter {seq}; refusing to open browser"
        ));
    }
    Ok(())
}

/// Whether the current process is running under WSL.
///
/// Result is cached in a `OnceLock`: [`open_url`] is called from `async`
/// auth flows via a synchronous trait method, so the underlying `/proc`
/// reads must not run on the executor thread on every launch. The reads
/// happen at most once per process.
///
/// This detection is **load-bearing**, not an optimization: `&`, `(`, `)`,
/// and `%` are deliberately absent from [`SHELL_METACHARS`], so the only
/// thing keeping them off the WSL shell path is [`open_url`] taking the
/// non-`webbrowser` route gated on this check. Our markers are a superset
/// of the `webbrowser` crate's own WSL detection (it checks only
/// `WSLInterop`), so whenever the crate would take the shell path, this
/// bypass has already fired. See SNOW-3649282.
#[cfg(target_os = "linux")]
fn is_wsl() -> bool {
    use std::sync::OnceLock;
    static CACHED: OnceLock<bool> = OnceLock::new();
    *CACHED.get_or_init(detect_wsl)
}

/// One-shot WSL detection backing [`is_wsl`]'s cache. Reads the two markers
/// the interop layer exposes: the `WSLInterop` binfmt registration and the
/// `microsoft` tag in `/proc/version`.
#[cfg(target_os = "linux")]
fn detect_wsl() -> bool {
    if std::fs::read_to_string("/proc/sys/fs/binfmt_misc/WSLInterop")
        .map(|s| s.contains("enabled"))
        .unwrap_or(false)
    {
        return true;
    }
    std::fs::read_to_string("/proc/version")
        .map(|s| s.to_ascii_lowercase().contains("microsoft"))
        .unwrap_or(false)
}

/// Ordered list of `(program, args)` launcher candidates to try on WSL.
///
/// Every candidate passes `url` as its own standalone argv element to a
/// program that opens it via the Windows protocol handler — never through
/// a shell. `explorer.exe` is tried first; `wslview` (from the `wslu`
/// package) is the fallback. Both open the user's default browser.
///
/// Pure and side-effect free so it can be unit-tested without spawning.
#[cfg(target_os = "linux")]
fn wsl_launch_candidates(url: &str) -> Vec<(&'static str, Vec<String>)> {
    vec![
        ("explorer.exe", vec![url.to_string()]),
        ("wslview", vec![url.to_string()]),
    ]
}

/// Try each `(program, args)` candidate in order, returning `Ok` on the
/// first that `spawn` accepts. On exhaustion returns the last error.
///
/// The `spawn` strategy is injected so the ordered-fallback logic — the
/// WSL-specific control flow — is unit-testable without launching real
/// processes or requiring a WSL host.
#[cfg(target_os = "linux")]
fn open_via_candidates(
    candidates: Vec<(&'static str, Vec<String>)>,
    mut spawn: impl FnMut(&str, &[String]) -> Result<(), String>,
) -> Result<(), String> {
    let mut last_err = String::from("no WSL browser launcher available");
    for (program, args) in candidates {
        match spawn(program, &args) {
            Ok(()) => return Ok(()),
            Err(e) => last_err = format!("{program}: {e}"),
        }
    }
    Err(last_err)
}

/// Open `url` in the user's system browser.
///
/// Runs [`validate_browser_url`] first — so this function is safe to call
/// from any site regardless of whether the caller pre-validated — then, on
/// WSL, launches via [`wsl_launch_candidates`] instead of the shell path
/// (SNOW-3649282); on every other platform, defers to the `webbrowser`
/// crate.
///
/// Callers that need to distinguish "URL rejected" from "launch failed"
/// (e.g. to decide whether to print a manual-paste fallback) should still
/// call [`validate_browser_url`] themselves first; the internal check here
/// is a backstop that guarantees no unvalidated URL ever reaches a
/// launcher, not a replacement for that decision.
pub(crate) fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        open_url_impl(
            url,
            is_wsl(),
            |program, args| {
                std::process::Command::new(program)
                    .args(args)
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            },
            |u| webbrowser::open(u).map_err(|e| e.to_string()),
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        // Backstop: never hand an unvalidated URL to a launcher.
        validate_browser_url(url)?;
        webbrowser::open(url).map_err(|e| e.to_string())
    }
}

/// Routing core of [`open_url`], with `is_wsl` and the launch strategies
/// injected so the WSL-vs-fallback decision is unit-testable without a WSL
/// host, real subprocesses, or opening a browser.
///
/// Validation runs first (the backstop that no caller can bypass); then on
/// WSL the URL goes to `spawn` via [`wsl_launch_candidates`], otherwise to
/// `fallback` (the `webbrowser` crate in production).
#[cfg(target_os = "linux")]
fn open_url_impl(
    url: &str,
    is_wsl: bool,
    spawn: impl FnMut(&str, &[String]) -> Result<(), String>,
    fallback: impl FnOnce(&str) -> Result<(), String>,
) -> Result<(), String> {
    // Backstop: never hand an unvalidated URL to any launcher, even if a
    // caller forgot to validate (SNOW-3649282).
    validate_browser_url(url)?;

    if is_wsl {
        return open_via_candidates(wsl_launch_candidates(url), spawn);
    }
    fallback(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_accepts_plain_https_url() {
        assert!(validate_browser_url("https://idp.example.com/sso?state=abc123").is_ok());
        // Scheme comparison is case-insensitive (RFC 3986 §3.1).
        assert!(validate_browser_url("HTTPS://idp.example.com/sso?state=abc123").is_ok());
    }

    #[test]
    fn validate_accepts_multi_param_saml_style_url() {
        // Real SAML/OAuth ssoUrls carry `&`-separated params, parentheses,
        // and percent-encoding. These MUST be accepted — rejecting them
        // would break genuine SSO logins. They are made safe on WSL by the
        // transport-layer bypass in `open_url`, not by validation.
        assert!(
            validate_browser_url(
                "https://idp.example.com/app/sso/saml?SAMLRequest=aB%2Bc&RelayState=x(y)"
            )
            .is_ok()
        );
        assert!(validate_browser_url("https://login.example.com/authorize?rt=code&x=1").is_ok());
    }

    #[test]
    fn validate_accepts_encoded_space_but_rejects_raw_space() {
        // `%20` is the legitimate encoding of a space and must pass; a RAW
        // space would split argv on the WSL join and is rejected.
        assert!(validate_browser_url("https://idp.example.com/sso?s=a%20b").is_ok());
        assert!(validate_browser_url("https://idp.example.com/sso?s=a b").is_err());
    }

    #[test]
    fn validate_rejects_non_https_scheme() {
        assert!(validate_browser_url("http://idp.example.com/sso").is_err());
        assert!(validate_browser_url("file:///etc/passwd").is_err());
        assert!(validate_browser_url("javascript:alert(1)").is_err());
        // Even a URL that merely contains "https://" later must be rejected.
        assert!(validate_browser_url("ftp://x/https://y").is_err());
    }

    #[test]
    fn validate_rejects_the_poc_pipe_injection() {
        // Representative shell-metacharacter payload (SNOW-3649282).
        let payload = "https://evil-idp.example.com/sso?state=poc|calc";
        assert!(
            validate_browser_url(payload).is_err(),
            "pipe payload must be rejected"
        );
    }

    #[test]
    fn validate_rejects_every_shell_metacharacter() {
        for c in SHELL_METACHARS {
            let url = format!("https://idp.example.com/sso?x=a{c}b");
            assert!(
                validate_browser_url(&url).is_err(),
                "URL containing {c:?} must be rejected"
            );
        }
    }

    #[test]
    fn validate_rejects_percent_encoded_shell_metacharacters() {
        // Encoded forms of the shell operators must not slip past the literal
        // scan (case-insensitive). `%7C`=`|`, `%3C`=`<`, `%3E`=`>`, `%21`=`!`,
        // `%5E`=`^`.
        for payload in [
            "https://idp.example.com/sso?s=poc%7Ccalc", // encoded pipe
            "https://idp.example.com/sso?s=poc%7ccalc", // lowercase hex
            "https://idp.example.com/sso?s=%3Cin",      // encoded <
            "https://idp.example.com/sso?s=out%3E",     // encoded >
            "https://idp.example.com/sso?s=%21bang",    // encoded !
            "https://idp.example.com/sso?s=%5Ecaret",   // encoded ^
        ] {
            assert!(
                validate_browser_url(payload).is_err(),
                "percent-encoded shell metacharacter must be rejected: {payload}"
            );
        }
    }

    #[test]
    fn validate_rejects_quotes_backtick_and_backslash() {
        // Chars that perturb Windows argv splitting or are never valid
        // unencoded in a URL.
        for c in FORBIDDEN_LITERAL_CHARS {
            let url = format!("https://idp.example.com/sso?x=a{c}b");
            assert!(
                validate_browser_url(&url).is_err(),
                "URL containing {c:?} must be rejected"
            );
        }
    }

    #[test]
    fn validate_rejects_control_bytes() {
        // Newline / CR / tab / NUL must never reach a launcher.
        for payload in [
            "https://idp.example.com/sso?s=a\nb",
            "https://idp.example.com/sso?s=a\rb",
            "https://idp.example.com/sso?s=a\tb",
            "https://idp.example.com/sso?s=a\0b",
        ] {
            assert!(
                validate_browser_url(payload).is_err(),
                "control byte must be rejected"
            );
        }
    }

    #[test]
    fn validate_rejects_dangerous_and_illegitimate_payloads() {
        // Payloads relying on a char that is BOTH risky at a launcher AND
        // never legitimate in a URL — caught by validation directly (in
        // addition to the transport-layer bypass). See SNOW-3649282.
        for payload in [
            "https://x/sso?s=y|calc",     // pipe
            "https://x/sso?s=y!DELAYED!", // bang
            "https://x/sso?s=y>out.txt",  // gt
            "https://x/sso?s=y<in.txt",   // lt
            "https://x/sso?s=y^&calc",    // caret
        ] {
            assert!(
                validate_browser_url(payload).is_err(),
                "payload must be rejected: {payload}"
            );
        }
    }

    // ─── WSL launcher (transport-layer) tests ────────────────────────────

    #[cfg(target_os = "linux")]
    #[test]
    fn wsl_launcher_never_uses_a_shell_interpreter() {
        // Even for a payload that (hypothetically) got past validation, the
        // launcher must never route through cmd.exe / powershell, and must
        // never pass "/c". This asserts the mitigation, not an attack.
        let payload = "https://x/sso?s=y|calc&whoami";
        for (program, args) in wsl_launch_candidates(payload) {
            assert_ne!(program, "cmd.exe", "must not launch via cmd.exe");
            assert_ne!(program, "powershell.exe", "must not launch via powershell");
            assert!(
                !args.iter().any(|a| a == "/c"),
                "must not pass the /c shell-command switch"
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn wsl_launcher_passes_url_as_single_opaque_argument() {
        // The URL — metacharacters and all — must arrive as exactly one
        // argv element, never split or concatenated with other tokens, so
        // the launcher treats it as one navigation target.
        let payload = "https://x/sso?s=y|calc";
        for (_program, args) in wsl_launch_candidates(payload) {
            assert_eq!(
                args,
                vec![payload.to_string()],
                "URL must be the sole argument, passed verbatim"
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn wsl_launcher_prefers_explorer_then_wslview() {
        let candidates = wsl_launch_candidates("https://idp.example.com/sso");
        let programs: Vec<_> = candidates.iter().map(|(p, _)| *p).collect();
        assert_eq!(programs, vec!["explorer.exe", "wslview"]);
    }

    // ─── WSL runtime fallback loop (open_via_candidates) ─────────────────

    #[cfg(target_os = "linux")]
    #[test]
    fn open_via_candidates_returns_on_first_success_and_stops() {
        use std::cell::RefCell;
        let tried = RefCell::new(Vec::new());
        let result = open_via_candidates(
            wsl_launch_candidates("https://idp.example.com/sso"),
            |program, _args| {
                tried.borrow_mut().push(program.to_string());
                Ok(()) // first candidate (explorer.exe) succeeds
            },
        );
        assert!(result.is_ok());
        assert_eq!(
            *tried.borrow(),
            vec!["explorer.exe"],
            "must stop at the first launcher that succeeds"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn open_via_candidates_falls_through_to_next_on_failure() {
        use std::cell::RefCell;
        let tried = RefCell::new(Vec::new());
        let result = open_via_candidates(
            wsl_launch_candidates("https://idp.example.com/sso"),
            |program, args| {
                tried.borrow_mut().push(program.to_string());
                // Every candidate still receives the URL as a single arg.
                assert_eq!(args, ["https://idp.example.com/sso".to_string()]);
                if program == "explorer.exe" {
                    Err("not found".to_string())
                } else {
                    Ok(()) // wslview succeeds
                }
            },
        );
        assert!(result.is_ok());
        assert_eq!(
            *tried.borrow(),
            vec!["explorer.exe", "wslview"],
            "must try explorer.exe first, then fall through to wslview"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn open_via_candidates_reports_last_error_when_all_fail() {
        let result = open_via_candidates(
            wsl_launch_candidates("https://idp.example.com/sso"),
            |program, _args| Err(format!("{program} unavailable")),
        );
        let err = result.expect_err("all launchers failed");
        assert!(
            err.contains("wslview"),
            "error should surface the last-tried launcher, got: {err}"
        );
    }

    // ─── open_url routing (open_url_impl) ────────────────────────────────

    #[cfg(target_os = "linux")]
    #[test]
    fn open_url_on_wsl_uses_safe_launcher_never_fallback() {
        // With is_wsl = true, the URL must go to the launcher candidates —
        // as a single arg, explorer.exe first — and the webbrowser fallback
        // must never be reached. This is the composition that guarantees the
        // fix is actually active on WSL (a regression that left is_wsl always
        // false would otherwise pass every other test).
        use std::cell::RefCell;
        let tried = RefCell::new(Vec::new());
        let result = open_url_impl(
            "https://idp.example.com/sso",
            true,
            |program, args| {
                assert_eq!(args, ["https://idp.example.com/sso".to_string()]);
                tried.borrow_mut().push(program.to_string());
                Ok(())
            },
            |_| panic!("fallback (webbrowser) must not be used on WSL"),
        );
        assert!(result.is_ok());
        assert_eq!(*tried.borrow(), vec!["explorer.exe"]);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn open_url_off_wsl_uses_fallback_never_spawn() {
        use std::cell::Cell;
        let fell_back = Cell::new(false);
        let result = open_url_impl(
            "https://idp.example.com/sso",
            false,
            |_, _| panic!("launcher must not spawn off WSL"),
            |u| {
                assert_eq!(u, "https://idp.example.com/sso");
                fell_back.set(true);
                Ok(())
            },
        );
        assert!(result.is_ok());
        assert!(fell_back.get(), "off WSL must defer to the fallback");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn open_url_impl_rejects_unsafe_url_before_routing() {
        // An unsafe URL is refused before either strategy runs, on both
        // is_wsl branches.
        for is_wsl in [true, false] {
            let result = open_url_impl(
                "https://x/sso?s=y|calc",
                is_wsl,
                |_, _| panic!("must not spawn for a rejected URL"),
                |_| panic!("must not fall back for a rejected URL"),
            );
            assert!(
                result.is_err(),
                "unsafe URL must be rejected (is_wsl={is_wsl})"
            );
        }
    }

    // ─── open_url self-validation backstop ───────────────────────────────

    #[test]
    fn open_url_rejects_unvalidated_url_before_launching() {
        // `open_url` must refuse an unsafe URL itself — even a caller that
        // skipped `validate_browser_url` cannot reach a launcher with a
        // rejected URL. These return at the validation gate, before any
        // spawn / webbrowser call, so the test is portable (no browser opens).
        assert!(
            open_url("https://evil-idp.example.com/sso?state=poc|calc").is_err(),
            "metacharacter payload must be rejected by open_url itself"
        );
        assert!(
            open_url("http://idp.example.com/sso").is_err(),
            "non-https URL must be rejected by open_url itself"
        );
    }
}
