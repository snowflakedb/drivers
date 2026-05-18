use std::env;
use std::process::{Command, exit};

const TARGET_CRATE: &str = "arrow_buffer";
const TARGET_TRIPLE: &str = "arm64ec-pc-windows-msvc";

fn main() {
    let mut args: Vec<String> = env::args_os()
        .skip(1)
        .map(|s| {
            s.into_string().unwrap_or_else(|os| {
                eprintln!(
                    "rustc-wrapper-arm64ec: non-UTF-8 rustc argument: {os:?}"
                );
                exit(2);
            })
        })
        .collect();

    if args.is_empty() {
        eprintln!(
            "rustc-wrapper-arm64ec: no rustc command provided; \
             RUSTC_WRAPPER must be invoked by cargo"
        );
        exit(2);
    }

    let crate_name = find_value(&args, "--crate-name");
    let target = find_value(&args, "--target");

    let inject = crate_name.as_deref() == Some(TARGET_CRATE)
        && target.as_deref() == Some(TARGET_TRIPLE);

    if inject {
        args.push("--allow=explicit_builtin_cfgs_in_flags".to_string());
        args.push("--cfg".to_string());
        args.push(r#"target_arch="aarch64""#.to_string());

        if verbose() {
            let version = env::var("CARGO_PKG_VERSION").unwrap_or_default();
            eprintln!(
                "rustc-wrapper-arm64ec: injecting --cfg target_arch=\"aarch64\" \
                 into arrow-buffer {version} for {TARGET_TRIPLE}"
            );
        }
    }

    // First positional arg is the path to the real rustc.
    let rustc = args.remove(0);
    let status = Command::new(&rustc)
        .args(&args)
        .status()
        .unwrap_or_else(|e| {
            eprintln!(
                "rustc-wrapper-arm64ec: failed to invoke `{rustc}`: {e}"
            );
            exit(2);
        });
    exit(status.code().unwrap_or(2));
}

/// Returns the value following the first occurrence of `flag` in `args`,
/// handling both `--flag value` and `--flag=value` forms.
fn find_value(args: &[String], flag: &str) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == flag {
            return iter.next().cloned();
        }
        if let Some(rest) = arg.strip_prefix(flag).and_then(|s| s.strip_prefix('=')) {
            return Some(rest.to_string());
        }
    }
    None
}

fn verbose() -> bool {
    matches!(
        env::var("RUSTC_WRAPPER_ARM64EC_VERBOSE").as_deref(),
        Ok("1" | "true" | "TRUE")
    )
}
