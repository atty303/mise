#![allow(unknown_lints)]
#![allow(clippy::literal_string_with_formatting_args)]
use std::fmt::Display;

use indoc::formatdoc;

use crate::shell::{ActivateOptions, Shell};

#[derive(Default)]
pub struct Nushell {}

impl Nushell {
    fn need_escape(ch: char) -> bool {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '=' | '/' | ',' | '.' | '+' => false,
            _ => true,
        }
    }

    fn escape_string_raw(s: &str) -> String {
        if s.contains(Self::need_escape) {
            format!("r#'{}'#", s)
        } else {
            s.to_owned()
        }
    }
}

impl Shell for Nushell {
    fn activate(&self, opts: ActivateOptions) -> String {
        let exe = opts.exe;
        let flags = opts.flags;
        let exe = exe.to_string_lossy().replace('\\', r#"\\"#);

        formatdoc! {r#"
          export-env {{
            $env.MISE_SHELL = "nu"
            let mise_hook = {{
              condition: {{ "MISE_SHELL" in $env }}
              code: {{ mise_hook }}
            }}
            add-hook hooks.pre_prompt $mise_hook
            add-hook hooks.env_change.PWD $mise_hook
          }}

          def --env add-hook [field: cell-path new_hook: any] {{
            let old_config = $env.config? | default {{}}
            let old_hooks = $old_config | get $field --ignore-errors | default []
            $env.config = ($old_config | upsert $field ($old_hooks ++ [$new_hook]))
          }}

          def "parse vars" [] {{
            $in | from csv --noheaders --no-infer | rename 'op' 'name' 'value'
          }}

          export def --env --wrapped main [command?: string, --help, ...rest: string] {{
            let commands = ["deactivate", "shell", "sh"]

            if ($command == null) {{
              ^"{exe}"
            }} else if ($command == "activate") {{
              $env.MISE_SHELL = "nu"
            }} else if ($command in $commands) {{
              ^"{exe}" $command ...$rest
              | parse vars
              | update-env
            }} else {{
              ^"{exe}" $command ...$rest
            }}
          }}

          def --env "update-env" [] {{
            for $var in $in {{
              if $var.op == "set" {{
                if $var.name == 'PATH' {{
                  $env.PATH = ($var.value | split row (char esep))
                }} else {{
                  load-env {{($var.name): $var.value}}
                }}
              }} else if $var.op == "hide" {{
                hide-env $var.name
              }}
            }}
          }}

          def --env mise_hook [] {{
            ^"{exe}" hook-env{flags} -s nu
              | parse vars
              | update-env
          }}

        "#}
    }

    fn deactivate(&self) -> String {
        [
            self.unset_env("MISE_SHELL"),
            self.unset_env("__MISE_DIFF"),
            self.unset_env("__MISE_DIFF"),
        ]
        .join("")
    }

    fn set_env(&self, k: &str, v: &str) -> String {
        let k = Nushell::escape_string_raw(k);
        let v = Nushell::escape_string_raw(v);
        format!("$env | upsert {k} {v}\n")
    }

    fn prepend_env(&self, k: &str, v: &str) -> String {
        let k = Nushell::escape_string_raw(k);
        let v = Nushell::escape_string_raw(v);
        format!("$env.{k} = ($env.{k} | split row (char esep) | prepend {v})\n")
    }

    fn unset_env(&self, k: &str) -> String {
        let k = Nushell::escape_string_raw(k);
        format!("hide-env {k} --ignore-errors\n")
    }
}

impl Display for Nushell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "nu")
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use std::path::Path;
    use test_log::test;

    use crate::test::replace_path;

    use super::*;

    #[test]
    fn test_hook_init() {
        let nushell = Nushell::default();
        let exe = Path::new("/some/dir/mise");
        let opts = ActivateOptions {
            exe: exe.to_path_buf(),
            flags: " --status".into(),
            no_hook_env: false,
        };
        assert_snapshot!(nushell.activate(opts));
    }

    #[test]
    fn test_set_env() {
        assert_snapshot!(Nushell::default().set_env("FOO", "1 2"));
    }

    #[test]
    fn test_prepend_env() {
        let sh = Nushell::default();
        assert_snapshot!(replace_path(&sh.prepend_env("PATH", "/some/dir")));
    }

    #[test]
    fn test_unset_env() {
        assert_snapshot!(Nushell::default().unset_env("FOO BAR"));
    }

    #[test]
    fn test_deactivate() {
        let deactivate = Nushell::default().deactivate();
        assert_snapshot!(replace_path(&deactivate));
    }
}
