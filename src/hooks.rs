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
# Zen Shell Integration (v4)
# Wraps zen binary so 'zen activate' modifies the current shell

# Locate the real zen binary once
__ZEN_BIN="$(command which zen 2>/dev/null)"
__ZEN_ACTIVE_NAME=""

zen() {
    local cmd="${1:-}"

    case "$cmd" in
        activate)
            shift
            local env_name="${1:-}"

            # Query the real binary for the environment name and path
            # Binary outputs two lines on stdout: name (line 1), path (line 2)
            local extra_args=""
            if [ -n "$env_name" ]; then
                extra_args="$env_name"
            fi
            local output=$("$__ZEN_BIN" activate $extra_args --path-only 2>/dev/tty)
            # NOTE: $extra_args is intentionally unquoted — it is either empty
            # (no word-splitting) or a single validated envname token. Zen's
            # EnvName::new() rejects any name containing shell metacharacters,
            # whitespace, or special characters, making injection impossible.
            local rc=$?

            if [ $rc -eq 0 ] && [ -n "$output" ]; then
                local display_name
                display_name=$(echo "$output" | head -1)
                local env_path
                env_path=$(echo "$output" | tail -1)

                if [ -d "$env_path" ] && [ -f "$env_path/bin/activate" ]; then
                    source "$env_path/bin/activate"
                    # Override the prompt — activate hardcodes the dir name
                    # Sanitize display_name for PS1: strip any non-alphanumeric/dash/underscore/dot
                    local safe_name
                    safe_name=$(printf '%s' "$display_name" | tr -cd 'a-zA-Z0-9._-')
                    PS1="($safe_name) ${_OLD_VIRTUAL_PS1:-}"
                    export PS1
                    VIRTUAL_ENV_PROMPT="($display_name) "
                    export VIRTUAL_ENV_PROMPT
                    __ZEN_ACTIVE_NAME="$display_name"
                    echo "✓ Activated environment: $display_name ($env_path)"
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
                local display_name="${__ZEN_ACTIVE_NAME:-$(basename "$VIRTUAL_ENV")}"
                deactivate 2>/dev/null
                __ZEN_ACTIVE_NAME=""
                echo "✓ Deactivated environment: $display_name"
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
# Zen Shell Integration for Fish (v4)

set -g __ZEN_BIN (command which zen 2>/dev/null)
set -g __ZEN_ACTIVE_NAME ""

function zen --wraps zen
    set cmd $argv[1]

    switch "$cmd"
        case activate
            set env_name $argv[2]

            # Binary outputs two lines on stdout: name (line 1), path (line 2)
            if test -n "$env_name"
                set output ($__ZEN_BIN activate $env_name --path-only 2>/dev/tty)
            else
                set output ($__ZEN_BIN activate --path-only 2>/dev/tty)
            end

            if test $status -eq 0 -a (count $output) -ge 2
                set display_name $output[1]
                set env_path $output[2]

                if test -d "$env_path" -a -f "$env_path/bin/activate.fish"
                    source "$env_path/bin/activate.fish"
                    # Override the prompt — activate hardcodes the dir name
                    set -gx VIRTUAL_ENV_PROMPT "($display_name) "
                    set -g __ZEN_ACTIVE_NAME "$display_name"
                    echo "✓ Activated environment: $display_name ($env_path)"
                else
                    echo "Error: Activation script not found at $env_path/bin/activate.fish"
                    return 1
                end
            end
        case deactivate
            if set -q VIRTUAL_ENV
                if test -n "$__ZEN_ACTIVE_NAME"
                    set display_name "$__ZEN_ACTIVE_NAME"
                else
                    set display_name (basename $VIRTUAL_ENV)
                end
                deactivate 2>/dev/null
                set -g __ZEN_ACTIVE_NAME ""
                echo "✓ Deactivated environment: $display_name"
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
