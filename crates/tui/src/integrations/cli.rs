//! `codewhale integrations …` command surface (dispatched from the outer
//! `codewhale` binary as a passthrough).

use std::io::IsTerminal;
use std::path::Path;

use anyhow::{Context, Result};

use crate::config::Config;
use crate::integrations::dsh::{
    self, CLI_COMMAND, DetectEnv, DshIntegrationState, DshPaths, DshPlan, DshReceiptEvent,
    DshStatusReport, ProcessRunner, RELATIONSHIP_LABEL,
};
use crate::{DshIntegrationCommand, IntegrationsCommand};

pub(crate) fn run(config: &Config, workspace: &Path, command: IntegrationsCommand) -> Result<()> {
    match command {
        IntegrationsCommand::Dsh { command } => run_dsh(config, workspace, command),
    }
}

fn detect_now() -> dsh::DshDetection {
    dsh::detect::detect(&DetectEnv::from_process(), &ProcessRunner)
}

fn status_report(
    config: &Config,
    workspace: &Path,
    allow_full_access: bool,
) -> Result<(DshPaths, DshStatusReport)> {
    let paths = DshPaths::from_process()?;
    let detection = detect_now();
    let identity = dsh::codewhale_route_identity(config, workspace);
    let report = dsh::compute_status(&paths, detection, identity, allow_full_access)?;
    Ok((paths, report))
}

fn confirm(yes: bool, question: &str) -> Result<()> {
    if yes {
        return Ok(());
    }
    if !std::io::stdin().is_terminal() {
        anyhow::bail!("stdin is not a terminal; pass --yes to confirm the disclosed plan");
    }
    print!("{question} [y/N] ");
    use std::io::Write;
    std::io::stdout().flush().ok();
    let mut answer = String::new();
    std::io::stdin()
        .read_line(&mut answer)
        .context("read confirmation")?;
    if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
        anyhow::bail!("not confirmed; nothing was written");
    }
    Ok(())
}

fn print_status(report: &DshStatusReport) {
    println!("DeepSeek Harness (dsh) — {}", RELATIONSHIP_LABEL);
    println!("  state: {}", report.state.label());
    println!("  summary: {}", dsh::status_line(report));
    match report.detection.binary.as_ref() {
        Some(bin) => println!("  dsh binary: {}", bin.display()),
        None => println!("  dsh binary: not on PATH"),
    }
    println!(
        "  dsh version: {} ({})",
        report.detection.version.as_deref().unwrap_or("unknown"),
        report.detection.compatibility.label()
    );
    println!(
        "  DSH_HOME: {}{}{}",
        report.detection.dsh_home.display(),
        if report.detection.dsh_home_from_env {
            " (from env)"
        } else {
            ""
        },
        if report.detection.dsh_home_exists {
            ""
        } else {
            " (absent)"
        }
    );
    if !report.detection.profiles.is_empty() {
        println!("  dsh profiles: {}", report.detection.profiles.join(", "));
    }
    println!(
        "  dsh credentials file: {}{}",
        if report.detection.credentials_present {
            "present"
        } else {
            "absent"
        },
        match report.detection.credentials_mode_ok {
            Some(true) => " (0600)",
            Some(false) => " (mode is not 0600)",
            None => "",
        }
    );
    if !report.shadowing_namespaces.is_empty() {
        println!(
            "  dsh settings.yaml sections that can shadow the overlay: {}",
            report.shadowing_namespaces.join(", ")
        );
    }
    println!("  Codewhale-owned files: {}", report.paths_root.display());
    println!(
        "  overlay: {}{}",
        report.overlay_path.display(),
        if report.overlay_present {
            ""
        } else {
            " (absent)"
        }
    );
    if let Some(record) = report.record.as_ref() {
        println!(
            "  connected: {} · profile {} · {}/{} via {} · permission {}{}",
            record.connected_at,
            record.profile,
            record.identity.source.provider_id,
            record.identity.source.model,
            record.identity.dsh_provider().unwrap_or("unsupported"),
            record.identity.permission_mode.as_str(),
            if record.disabled { " · DISABLED" } else { "" }
        );
        if record.skin_enabled {
            println!(
                "  skin export: {} (unsupported overlay; not injected)",
                record
                    .skin_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            );
        }
    }
    match (&report.current_identity, &report.current_identity_error) {
        (Some(now), _) => println!(
            "  current Codewhale route: {}/{} · {} · would map via {}",
            now.source.provider_id,
            now.source.model,
            now.source.base_url,
            now.dsh_provider().unwrap_or("(not mappable)")
        ),
        (None, Some(error)) => println!("  current Codewhale route: unresolved ({error})"),
        (None, None) => {}
    }
    match &report.state {
        DshIntegrationState::Detected { .. } => {
            println!("  next: `{CLI_COMMAND} plan` then `{CLI_COMMAND} connect`")
        }
        DshIntegrationState::StaleConfig { .. } => println!("  next: `{CLI_COMMAND} update`"),
        DshIntegrationState::Disabled { .. } => println!("  next: `{CLI_COMMAND} enable`"),
        DshIntegrationState::Connected { .. } | DshIntegrationState::StaleVersion { .. } => {
            println!("  next: `{CLI_COMMAND} launch [--profile web|headless] [dsh app args]`")
        }
        _ => {}
    }
}

fn print_plan(plan: &DshPlan, event: &str) {
    println!("{RELATIONSHIP_LABEL} — {event} plan (nothing written yet)");
    println!(
        "  identity: {}/{} → DSH provider {} · endpoint {}",
        plan.mapped.source.provider_id,
        plan.mapped.source.model,
        plan.mapped.dsh_provider().unwrap_or("unsupported"),
        plan.mapped.source.base_url
    );
    println!(
        "  reasoning: codewhale={} → dsh={}",
        plan.mapped
            .source
            .reasoning_effort
            .as_deref()
            .unwrap_or("inherit"),
        plan.mapped
            .dsh_reasoning_effort
            .as_deref()
            .unwrap_or("(default)")
    );
    println!(
        "  permission: DSH_PERMISSION_MODE={}",
        plan.mapped.permission_mode.as_str()
    );
    println!("  workspace: {}", plan.mapped.source.workspace);
    println!("  will write: {}", plan.overlay_path.display());
    println!("  will write: {}", plan.receipt_path.display());
    if let Some(skin) = plan.skin_path.as_ref() {
        println!("  will write: {} (+ preview html)", skin.display());
    }
    println!(
        "  will NOT write: anything under $DSH_HOME, any API key, OAuth file, or file contents"
    );
    println!("  overlay sha256: {}", plan.overlay_sha256);
    println!("  launch: {}", plan.launch_command);
    for line in &plan.disclosures {
        println!("  note: {line}");
    }
    println!("  overlay preview:");
    for line in plan.overlay_text.lines() {
        println!("    {line}");
    }
}

fn run_dsh(config: &Config, workspace: &Path, command: DshIntegrationCommand) -> Result<()> {
    match command {
        DshIntegrationCommand::Status { json } => {
            let (_paths, report) = status_report(config, workspace, false)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print_status(&report);
            }
            Ok(())
        }
        DshIntegrationCommand::Plan {
            json,
            profile,
            allow_full_access,
            skin,
        } => {
            let (paths, report) = status_report(config, workspace, allow_full_access)?;
            let identity = dsh::codewhale_route_identity(config, workspace)
                .map_err(|e| anyhow::anyhow!("cannot resolve the current Codewhale route: {e}"))?;
            let plan = dsh::plan(
                &paths,
                &report.detection,
                &identity,
                &profile,
                allow_full_access,
                skin,
            )?;
            if json {
                println!("{}", serde_json::to_string_pretty(&plan)?);
            } else {
                print_plan(&plan, "connect");
                if !report.detection.installed() {
                    println!(
                        "  warning: dsh is not on PATH; the plan is valid but launch will fail"
                    );
                }
            }
            Ok(())
        }
        DshIntegrationCommand::Connect {
            profile,
            allow_full_access,
            skin,
            yes,
        } => {
            let (paths, report) = status_report(config, workspace, allow_full_access)?;
            ensure_launchable_dsh(&report)?;
            if report.record.as_ref().is_some_and(|r| !r.disabled) {
                anyhow::bail!(
                    "DSH is already connected; use `{CLI_COMMAND} update` to rewrite the overlay"
                );
            }
            let identity = dsh::codewhale_route_identity(config, workspace)
                .map_err(|e| anyhow::anyhow!("cannot resolve the current Codewhale route: {e}"))?;
            let plan = dsh::plan(
                &paths,
                &report.detection,
                &identity,
                &profile,
                allow_full_access,
                skin,
            )?;
            print_plan(&plan, "connect");
            confirm(yes, "Write these Codewhale-owned files and connect DSH?")?;
            let record =
                dsh::apply_plan(&paths, &report.detection, &plan, DshReceiptEvent::Connect)?;
            println!(
                "connected: {} (sha256 {})",
                record.overlay_path.display(),
                record.overlay_sha256
            );
            println!("receipt: {}", paths.receipt.display());
            Ok(())
        }
        DshIntegrationCommand::Update {
            profile,
            allow_full_access,
            skin,
            yes,
        } => {
            let (paths, report) = status_report(config, workspace, allow_full_access)?;
            ensure_launchable_dsh(&report)?;
            let record = report.record.as_ref().ok_or_else(|| {
                anyhow::anyhow!("DSH is not connected; run `{CLI_COMMAND} connect`")
            })?;
            let profile = profile.unwrap_or_else(|| record.profile.clone());
            let skin = skin.unwrap_or(record.skin_enabled);
            let identity = dsh::codewhale_route_identity(config, workspace)
                .map_err(|e| anyhow::anyhow!("cannot resolve the current Codewhale route: {e}"))?;
            let plan = dsh::plan(
                &paths,
                &report.detection,
                &identity,
                &profile,
                allow_full_access,
                skin,
            )?;
            print_plan(&plan, "update");
            confirm(yes, "Rewrite the Codewhale overlay for DSH?")?;
            let record =
                dsh::apply_plan(&paths, &report.detection, &plan, DshReceiptEvent::Update)?;
            println!(
                "updated: {} (sha256 {})",
                record.overlay_path.display(),
                record.overlay_sha256
            );
            Ok(())
        }
        DshIntegrationCommand::Launch {
            profile,
            dry_run,
            args,
        } => {
            let (_paths, report) = status_report(config, workspace, false)?;
            let spec = dsh::launch_spec(&report, profile.as_deref(), &args, workspace)?;
            println!("{RELATIONSHIP_LABEL}: {}", spec.display());
            if dry_run {
                return Ok(());
            }
            let code = dsh::spawn_launch(&spec)?;
            if code != 0 {
                anyhow::bail!("dsh exited with status {code}");
            }
            Ok(())
        }
        DshIntegrationCommand::Disable => {
            let paths = DshPaths::from_process()?;
            let record = dsh::set_disabled(&paths, true)?;
            println!(
                "disabled: overlay kept at {}; launches refused",
                record.overlay_path.display()
            );
            Ok(())
        }
        DshIntegrationCommand::Enable => {
            let paths = DshPaths::from_process()?;
            let record = dsh::set_disabled(&paths, false)?;
            println!("enabled: {}", record.overlay_path.display());
            Ok(())
        }
        DshIntegrationCommand::Remove { yes } => {
            let paths = DshPaths::from_process()?;
            println!(
                "remove will delete only Codewhale-owned files under {}:",
                paths.root.display()
            );
            for path in [&paths.overlay, &paths.skin, &paths.skin_preview] {
                if path.exists() {
                    println!("  {}", path.display());
                }
            }
            println!(
                "  (receipt history is kept at {}; $DSH_HOME is never touched)",
                paths.receipt.display()
            );
            confirm(yes, "Remove the DSH integration files?")?;
            let removed = dsh::remove(&paths)?;
            println!("removed {} file(s)", removed.len());
            Ok(())
        }
    }
}

fn ensure_launchable_dsh(report: &DshStatusReport) -> Result<()> {
    match &report.state {
        DshIntegrationState::NotInstalled => {
            anyhow::bail!(
                "dsh is not on PATH; install the official DeepSeek Harness first (npm i -g @deepseek-ai/dsh)"
            )
        }
        DshIntegrationState::Offline { reason } => anyhow::bail!("dsh is offline: {reason}"),
        DshIntegrationState::Incompatible { reason, .. } => {
            anyhow::bail!("installed dsh is incompatible with this integration: {reason}")
        }
        _ => Ok(()),
    }
}
