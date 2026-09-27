//! Deny rules hold against commands whose word the shell resolves at run
//! time, and allow rules only cover the command as written.

use codewhale_execpolicy::{
    AskForApproval, ExecApprovalRequirement, ExecPolicyContext, ExecPolicyEngine, PermissionAction,
    Ruleset, ToolAskRule,
    bash_arity::BashArityDict,
    command_safety::is_parallel_readonly_command,
    toml_rules::{ExecPolicyConfig, RuleDecision},
};

fn context(command: &str, approval: AskForApproval) -> ExecPolicyContext<'_> {
    ExecPolicyContext {
        command,
        cwd: "/workspace",
        tool: Some("exec_shell"),
        path: None,
        ask_for_approval: approval,
        sandbox_mode: None,
    }
}

fn deny_rm_engines() -> [ExecPolicyEngine; 2] {
    [
        ExecPolicyEngine::new(vec![], vec!["rm".to_string()]),
        ExecPolicyEngine::with_rulesets(vec![Ruleset::user(vec![], vec![]).with_ask_rules(vec![
            ToolAskRule {
                action: PermissionAction::Deny,
                ..ToolAskRule::exec_shell("rm")
            },
        ])]),
    ]
}

/// Spellings whose command word is only known at run time, or which run a
/// command behind a reserved word or a wrapper's operands.
const HIDDEN_RM: &[&str] = &[
    "v=rm; $v -f f",
    "v=rm; \"$v\" -f f",
    "v=rm; ${v} -f f",
    "sudo $v f",
    "bash -c '$v f'",
    "eval \"$v f\"",
    "$(echo rm) -f f",
    "`echo rm` f",
    "rm${IFS}x",
    "x=r; ${x}m f",
    "IFS=,; c=rm,x; $c",
    "/bin/r[m] -f f",
    "{rm,-f,f}",
    "$'\\x72m' f",
    "printf rm | sh",
    "echo rm x | bash",
    "sh <<< 'rm x'",
    "source <(echo rm x)",
    "find . -exec rm {} +",
    "find . -execdir rm {} \\;",
    "if true; then rm x; fi",
    "while rm x; do :; done",
    "until rm x; do :; done",
    "! rm x",
    "function f { rm x; }",
    "chroot /newroot rm -rf /",
    "chroot /newroot sh -c 'rm -rf /'",
    "sudo --user root bash -c 'rm -rf /'",
    "sudo --user root rm -rf /",
    "timeout -s KILL 5 rm x",
    "env -S'rm x'",
];

/// Literal spellings that were already denied and must stay denied.
const LITERAL_RM: &[&str] = &[
    "rm -f f",
    "\\rm x",
    "r''m x",
    "/bin/rm x",
    "(rm x)",
    "{ rm x; }",
    "case a in a) rm x;; esac",
    "f(){ rm -f x; }; f",
    "xargs rm",
    "command -p rm x",
];

#[test]
fn deny_rules_hold_against_runtime_resolved_and_reserved_word_spellings() {
    for engine in deny_rm_engines() {
        for command in HIDDEN_RM.iter().chain(LITERAL_RM) {
            let decision = engine
                .check(context(command, AskForApproval::Never))
                .expect("policy check");
            assert!(
                !decision.allow
                    && matches!(
                        decision.requirement,
                        ExecApprovalRequirement::Forbidden { .. }
                    ),
                "{command:?} was not denied: {decision:?}"
            );
        }
    }
}

#[test]
fn ordinary_commands_stay_allowed_next_to_a_deny_rule() {
    for engine in deny_rm_engines() {
        for command in [
            "ls *.rs",
            "echo $HOME",
            "[ -f x ] && ls",
            "find . -name '*.rs'",
            "if true; then ls; fi",
            "rmdir x",
            "sudo -u root ls",
            "command -v rm",
            "command -pV rm",
        ] {
            let decision = engine
                .check(context(command, AskForApproval::Never))
                .expect("policy check");
            assert!(decision.allow, "{command:?} was denied: {decision:?}");
        }
    }
    // Without any deny rule, a runtime-resolved word is left to the mode.
    let open = ExecPolicyEngine::new(vec![], vec![]);
    for command in ["v=ls; $v", "echo $HOME", "ls *.rs"] {
        let decision = open
            .check(context(command, AskForApproval::Never))
            .expect("policy check");
        assert!(decision.allow, "{command:?} was denied: {decision:?}");
    }
}

#[test]
fn trusted_prefix_does_not_cover_interposed_options_or_nested_code() {
    let engine = ExecPolicyEngine::new(vec!["git status".to_string(), "ls".to_string()], vec![]);
    let trusted = |command: &str| {
        matches!(
            engine
                .check(context(command, AskForApproval::UnlessTrusted))
                .expect("policy check")
                .requirement,
            ExecApprovalRequirement::Skip { .. }
        )
    };
    assert!(trusted("git status"));
    assert!(trusted("git status -s --porcelain"));
    assert!(trusted("ls -la"));
    for command in [
        "git -ccore.fsmonitor=x status",
        "git -c core.fsmonitor=x status",
        "git --exec-path=/x status",
        "git -C /elsewhere status",
        "ls $(touch x)",
        "ls `touch x`",
        "$L -la",
    ] {
        assert!(!trusted(command), "{command:?} was auto-approved");
    }

    let dict = BashArityDict::new();
    assert!(!dict.allow_rule_matches("git status", "git --exec-path=/x status"));
    assert!(dict.allow_rule_matches("python -m pytest", "python -m pytest -x"));
    assert!(!dict.allow_rule_matches("python -m pytest", "python -m pip install x"));
}

#[test]
fn file_rules_fail_closed_on_runtime_resolved_words() {
    let config = ExecPolicyConfig::parse(
        r#"
[rules.shell]
allow = ["git status", "ls"]
deny = ["rm", "rm *"]
"#,
    )
    .expect("parse rules");
    for command in HIDDEN_RM {
        assert!(
            matches!(config.evaluate(command), RuleDecision::Deny(_)),
            "{command:?} was not denied"
        );
    }
    assert_eq!(config.evaluate("git status -s"), RuleDecision::Allow);
    for command in ["git -ccore.fsmonitor=x status", "ls $(touch x)"] {
        assert!(
            matches!(config.evaluate(command), RuleDecision::AskUser(_)),
            "{command:?} was auto-approved"
        );
    }
}

#[test]
fn parallel_read_only_rejects_parentheses() {
    assert!(is_parallel_readonly_command("cat README.md"));
    for command in [
        "cat .(e:'touch pwned':)",
        "ls foo(e:'id':)",
        "rg needle .(+cmd)",
        "cat (id)",
        "gh pr view 1(e:'id':)",
    ] {
        assert!(
            !is_parallel_readonly_command(command),
            "{command:?} was classified read-only"
        );
    }
}
