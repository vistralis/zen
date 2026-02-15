// SPDX-License-Identifier: Apache-2.0

//! Shell integration hooks for Zen.
//!
//! Generates shell-specific scripts that wrap the `zen` binary with a shell function.
//! This allows `zen activate` and `zen deactivate` to modify the current shell's
//! environment (PATH, VIRTUAL_ENV) — something a child process cannot do.
//!
//! A shell function named `zen` intercepts activation subcommands and
//! passes everything else to the real binary.

/// Generates a shell hook that wraps `zen` with activate/deactivate support.
///
/// Usage: `eval "$(zen hook zsh)"` or `eval "$(zen hook bash)"`
///
/// The generated hook:
/// - Wraps `zen` as a shell function intercepting `activate` and `deactivate`
/// - Preserves `za` as a convenient shortcut for `zen activate`
/// - Passes all other subcommands through to the real binary
pub fn generate_hook(shell: &str) -> String {
    match shell {
        "zsh" | "bash" => {
            // Find the real binary path at hook-eval time
            r#"
# Zen Shell Integration (v2)
# Wraps zen binary so 'zen activate' modifies the current shell

# Locate the real zen binary once
__ZEN_BIN="$(command which zen 2>/dev/null)"

zen() {
    local cmd="${1:-}"

    case "$cmd" in
        activate)
            shift
            local env_name="${1:-}"

            # Query the real binary for the environment path
            # Supports: zen activate <env>, zen activate (no args, menu), zen activate --last
            local extra_args=""
            if [ -n "$env_name" ]; then
                extra_args="$env_name"
            fi
            local env_path=$("$__ZEN_BIN" activate $extra_args --path-only 2>/dev/tty)
            local rc=$?

            if [ $rc -eq 0 ] && [ -n "$env_path" ] && [ -d "$env_path" ]; then
                if [ -f "$env_path/bin/activate" ]; then
                    source "$env_path/bin/activate"
                    echo "✓ Activated environment: $(basename $env_path)"
                else
                    echo "Error: Activation script not found at $env_path/bin/activate"
                    return 1
                fi
            elif [ $rc -ne 0 ]; then
                return $rc
            fi
            ;;
        deactivate)
            if [ -n "${VIRTUAL_ENV:-}" ]; then
                local env_name=$(basename "$VIRTUAL_ENV")
                deactivate 2>/dev/null
                echo "✓ Deactivated environment: $env_name"
            else
                echo "No active environment to deactivate."
            fi
            ;;
        *)
            # Pass everything else to the real binary
            "$__ZEN_BIN" "$@"
            ;;
    esac
}

# Shortcut: 'za myenv' = 'zen activate myenv'
za() {
    zen activate "$@"
}

# Shortcut: 'zd' = 'zen deactivate'
zd() {
    zen deactivate
}
"#
            .to_string()
        }
        "fish" => r#"
# Zen Shell Integration for Fish (v2)

set -g __ZEN_BIN (command which zen 2>/dev/null)

function zen --wraps zen
    set cmd $argv[1]

    switch "$cmd"
        case activate
            set env_name $argv[2]

            # Supports: zen activate <env>, zen activate (no args, menu), zen activate --last
            if test -n "$env_name"
                set env_path (eval $__ZEN_BIN activate "$env_name" --path-only 2>/dev/tty)
            else
                set env_path (eval $__ZEN_BIN activate --path-only 2>/dev/tty)
            end

            if test $status -eq 0 -a -n "$env_path" -a -d "$env_path"
                if test -f "$env_path/bin/activate.fish"
                    source "$env_path/bin/activate.fish"
                    echo "✓ Activated environment: "(basename $env_path)
                else
                    echo "Error: Activation script not found at $env_path/bin/activate.fish"
                    return 1
                end
            end
        case deactivate
            if set -q VIRTUAL_ENV
                set env_name (basename $VIRTUAL_ENV)
                deactivate 2>/dev/null
                echo "✓ Deactivated environment: $env_name"
            else
                echo "No active environment to deactivate."
            end
        case '*'
            eval $__ZEN_BIN $argv
    end
end

# Shortcut: 'za myenv' = 'zen activate myenv'
function za
    zen activate $argv
end

# Shortcut: 'zd' = 'zen deactivate'
function zd
    zen deactivate
end
"#
        .to_string(),
        _ => format!("echo \"Zen: Unsupported shell '{}'\"", shell),
    }
}
