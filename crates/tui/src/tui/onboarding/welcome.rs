//! Welcome screen content for onboarding.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::localization::MessageId;
use crate::palette;
use crate::tui::app::App;

pub fn lines(app: &App) -> Vec<Line<'static>> {
    let steps = welcome_step_labels(app).join(" -> ");
    let version = app
        .tr(MessageId::OnboardWelcomeVersion)
        .replace("{version}", env!("CARGO_PKG_VERSION"));
    let next_steps = app
        .tr(MessageId::OnboardWelcomeSteps)
        .replace("{steps}", &steps);

    vec![
        Line::from(Span::styled(
            "codewhale",
            Style::default()
                .fg(palette::WHALE_HUMAN)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            version,
            Style::default().fg(palette::TEXT_MUTED),
        )),
        Line::from(""),
        Line::from(Span::styled(
            app.tr(MessageId::OnboardWelcomeLead).to_string(),
            Style::default().fg(palette::TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            app.tr(MessageId::OnboardWelcomeSetupBlurb).to_string(),
            Style::default().fg(palette::TEXT_MUTED),
        )),
        Line::from(Span::styled(
            next_steps,
            Style::default().fg(palette::TEXT_MUTED),
        )),
        Line::from(Span::styled(
            app.tr(MessageId::OnboardWelcomeDefaults).to_string(),
            Style::default().fg(palette::TEXT_MUTED),
        )),
        Line::from(""),
        Line::from(Span::styled(
            app.tr(MessageId::OnboardWelcomeEnter).to_string(),
            Style::default().fg(palette::TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            app.tr(MessageId::OnboardWelcomeExit).to_string(),
            Style::default().fg(palette::TEXT_MUTED),
        )),
    ]
}

fn welcome_step_labels(app: &App) -> Vec<String> {
    let mut steps = vec![
        app.tr(MessageId::OnboardWelcomeStepLanguage).to_string(),
        // #3937: the appearance step is unconditional, so the preview names it.
        app.tr(MessageId::OnboardWelcomeStepAppearance).to_string(),
    ];
    if app.onboarding_needs_api_key {
        steps.push(app.tr(MessageId::OnboardWelcomeStepApiKey).to_string());
    }
    if !app.trust_mode && super::needs_trust(&app.workspace) {
        steps.push(app.tr(MessageId::OnboardWelcomeStepTrust).to_string());
    }
    steps.push(
        app.tr(MessageId::OnboardWelcomeStepMentalModels)
            .to_string(),
    );
    steps.push(app.tr(MessageId::OnboardWelcomeStepTips).to_string());
    steps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::localization::Locale;
    use crate::tui::app::TuiOptions;
    use std::path::PathBuf;

    fn test_app_with_locale(locale: Locale) -> App {
        let options = TuiOptions {
            ..crate::test_support::test_tui_options(PathBuf::from("."))
        };
        let mut app = App::new(options, &Config::default());
        app.ui_locale = locale;
        app
    }

    fn body(app: &App) -> String {
        lines(app)
            .into_iter()
            .flat_map(|line| {
                line.spans
                    .into_iter()
                    .map(|span| span.content.to_string())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn welcome_copy_names_workspace_restore_and_tokens() {
        let mut app = test_app_with_locale(Locale::En);
        app.onboarding_needs_api_key = false;
        app.trust_mode = true;
        let body = body(&app);

        // First-run copy names shipped capabilities, not only the constitution.
        assert!(body.contains("/workspace"));
        assert!(body.contains("/restore"));
        assert!(body.contains("/tokens"));
        assert!(body.contains("only these screens will appear"));
        assert!(body.contains(
            "Next: choose language -> pick a look -> learn modes and permissions -> what you can do."
        ));
        // The lead dropped the constitution metaphor, so the standing-guidance
        // line has to name its own subject rather than dangle off it.
        assert!(body.contains("Standing guidance ships with valid defaults"));
        assert!(body.contains("/constitution"));
        assert!(!body.contains("Code means two things"));
        // `/restore` reverts files; `/undo` drops a turn. Do not blur them.
        assert!(!body.contains("rewind a turn"));
        assert!(!body.contains("add an API key"));
        assert!(!body.contains("land in the chat"));
    }

    #[test]
    fn welcome_wordmark_uses_the_human_brand_lane() {
        let app = test_app_with_locale(Locale::En);
        let welcome = lines(&app);
        assert_eq!(welcome[0].spans[0].style.fg, Some(palette::WHALE_HUMAN));
        assert_ne!(palette::WHALE_HUMAN, palette::WHALE_ACTION);
    }

    #[test]
    fn welcome_steps_include_optional_api_key_and_trust_screens() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut app = test_app_with_locale(Locale::En);
        app.workspace = tmp.path().to_path_buf();
        app.onboarding_needs_api_key = true;
        app.trust_mode = false;

        let body = body(&app);

        assert!(body.contains(
            "Next: choose language -> pick a look -> connect API key -> trust workspace -> learn modes and permissions -> what you can do."
        ));
    }

    #[test]
    fn welcome_copy_uses_locale_registry() {
        let mut app = test_app_with_locale(Locale::ZhHans);
        app.onboarding_needs_api_key = false;
        app.trust_mode = true;

        let body = body(&app);

        assert!(body.contains("/workspace"));
        assert!(body.contains("/restore"));
        assert!(body.contains("/tokens"));
        assert!(!body.contains("代码在这里有两层含义"));
        // The last step is named with the same phrase the tips screen titles
        // itself, so the preview and the screen read as one thing.
        assert!(body.contains("接下来：选择语言 -> 选择外观 -> 了解模式与权限 -> 你能做什么。"));
        assert!(!body.contains("Press Enter"));
    }
}
