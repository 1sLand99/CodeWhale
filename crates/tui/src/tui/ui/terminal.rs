//! Terminal lifecycle: raw mode, alternate screen, keyboard-enhancement and
//! bracketed-paste flags, viewport recapture, and the input-event pump's
//! polling primitives.
//!
//! Moved verbatim out of `ui.rs`.

use super::*;

pub(crate) fn next_terminal_event(
    input: &TerminalInputPump,
    pending: &mut VecDeque<Event>,
    timeout: Duration,
) -> io::Result<Option<Event>> {
    if let Some(event) = pending.pop_front() {
        return Ok(Some(event));
    }
    let event = input.recv_timeout(timeout)?;
    if let Some(event) = event.as_ref() {
        observe_terminal_attention(event);
    }
    Ok(event)
}

pub(crate) fn try_next_terminal_event(
    input: &TerminalInputPump,
    pending: &mut VecDeque<Event>,
) -> io::Result<Option<Event>> {
    if let Some(event) = pending.pop_front() {
        return Ok(Some(event));
    }
    let event = input.try_recv()?;
    if let Some(event) = event.as_ref() {
        observe_terminal_attention(event);
    }
    Ok(event)
}

/// Drain input that Codewhale already read before releasing the terminal.
///
/// Ordinary buffered input is discarded so it cannot leak into the child.
/// Escape and Ctrl+C are different: they are cancellation authority. If one
/// is pending, preserve the complete input sequence and refuse the handoff so
/// the normal event loop can process it.
pub(crate) fn prepare_terminal_input_handoff(
    input: &TerminalInputPump,
    pending: &mut VecDeque<Event>,
) -> io::Result<bool> {
    let mut drained = VecDeque::new();
    while let Some(event) = input.try_recv()? {
        drained.push_back(event);
    }
    let interrupted = pending
        .iter()
        .chain(drained.iter())
        .any(terminal_event_interrupts_child_handoff);
    if interrupted {
        pending.extend(drained);
        return Ok(false);
    }
    pending.clear();
    Ok(true)
}

fn terminal_event_interrupts_child_handoff(event: &Event) -> bool {
    let Event::Key(key) = event else {
        return false;
    };
    if key.kind == KeyEventKind::Release {
        return false;
    }
    let mut key = *key;
    normalize_raw_ctrl_c(&mut key);
    matches!(key.code, KeyCode::Esc)
        || matches!(key.code, KeyCode::Char('c')) && key.modifiers.contains(KeyModifiers::CONTROL)
}

pub(crate) fn collect_pending_terminal_events(
    input: &TerminalInputPump,
    pending: &mut VecDeque<Event>,
) -> io::Result<()> {
    while let Some(event) = input.try_recv()? {
        // Focus is notification authority, not merely a render event. Apply
        // it at pump receipt so a queued FocusGained cannot sit behind an
        // engine TurnComplete and produce a false background notification.
        observe_terminal_attention(&event);
        pending.push_back(event);
    }
    Ok(())
}

fn observe_terminal_attention(event: &Event) {
    match event {
        Event::FocusGained => crate::tui::notifications::set_terminal_focused(true),
        Event::FocusLost => crate::tui::notifications::set_terminal_focused(false),
        _ => {}
    }
}

/// Refuse to enter raw mode unless both interactive streams are TTYs.
///
/// Keeping this check independent from `std::io` makes the launch contract
/// testable without trying to manipulate the test runner's own terminal.
pub(crate) fn require_interactive_terminal(stdin_is_tty: bool, stdout_is_tty: bool) -> Result<()> {
    if stdin_is_tty && stdout_is_tty {
        return Ok(());
    }
    Err(anyhow::anyhow!(
        "Codewhale TUI requires an interactive terminal (stdin and stdout must be a TTY).\n\
         Open a real terminal (Terminal.app, iTerm, Windows Terminal, …) and run `codew` \
         or `codewhale` there — not from a pipe, cron job, or non-TTY launcher.\n\
         For headless prompts use `codewhale exec \"…\"` instead."
    ))
}

/// Refuse to enter terminal modes from a background Unix process group.
///
/// A TTY can still report `isatty(3) == true` after a shell has suspended the
/// process. Reading from that background group triggers `SIGTTIN`; enabling
/// mouse or keyboard protocols before that stop poisons the shell with raw
/// escape reports. Check foreground ownership before the first mode change.
#[cfg(unix)]
pub(crate) fn require_foreground_terminal_owner() -> Result<()> {
    // SAFETY: both calls are read-only process/terminal queries on the
    // controlling stdin descriptor and require no borrowed memory.
    let (terminal_pgid, process_pgid) =
        unsafe { (libc::tcgetpgrp(libc::STDIN_FILENO), libc::getpgrp()) };
    if terminal_pgid < 0 {
        return Err(anyhow::anyhow!(
            "Codewhale TUI could not verify foreground terminal ownership: {}",
            io::Error::last_os_error()
        ));
    }
    validate_foreground_process_group(terminal_pgid, process_pgid)
}

#[cfg(not(unix))]
pub(crate) fn require_foreground_terminal_owner() -> Result<()> {
    Ok(())
}

#[cfg(unix)]
pub(crate) fn validate_foreground_process_group(
    terminal_pgid: libc::pid_t,
    process_pgid: libc::pid_t,
) -> Result<()> {
    if terminal_pgid == process_pgid {
        return Ok(());
    }
    Err(anyhow::anyhow!(
        "Codewhale TUI cannot start from a background or suspended terminal job \
         (terminal foreground process group {terminal_pgid}, Codewhale process group {process_pgid}).\n\
         Run `fg` to foreground the job or launch `codew` in a new terminal. \
         For automated prompts use `codewhale exec \"…\"` instead."
    ))
}

/// One side of the raw-mode probe abandonment handshake between the startup
/// probe timeout and the blocking `enable_raw_mode` task finishing late.
///
/// Each side publishes its own flag (`publish`), then checks whether the
/// other side's flag (`check`) is already up; a `true` return means this
/// side must disable raw mode again. `SeqCst` ordering guarantees that when
/// both sides run, at least one observes the other's flag, so a raw-mode
/// enable landing after the probe timeout is always undone. Both sides
/// observing each other is fine — a duplicate `disable_raw_mode` is a no-op.
pub(crate) fn raw_mode_probe_handshake(publish: &AtomicBool, check: &AtomicBool) -> bool {
    publish.store(true, Ordering::SeqCst);
    check.load(Ordering::SeqCst)
}

pub(crate) fn terminal_probe_timeout(config: &Config) -> Duration {
    let timeout_ms = config
        .tui
        .as_ref()
        .and_then(|tui| tui.terminal_probe_timeout_ms)
        .unwrap_or(DEFAULT_TERMINAL_PROBE_TIMEOUT_MS)
        .clamp(100, 5_000);
    Duration::from_millis(timeout_ms)
}

pub(crate) fn subagent_terminal_verb(status: &SubAgentStatus) -> &'static str {
    match status {
        SubAgentStatus::Completed => "completed",
        SubAgentStatus::Interrupted(_) => "interrupted",
        SubAgentStatus::Failed(_) => "failed",
        SubAgentStatus::Cancelled => "cancelled",
        SubAgentStatus::BudgetExhausted => "exhausted its budget",
        SubAgentStatus::Running => "finished",
    }
}

pub(crate) fn subagent_terminal_projection_from_mailbox(
    message: &MailboxMessage,
) -> Option<(&str, SubAgentStatus, Option<String>)> {
    match message {
        MailboxMessage::Completed { agent_id, summary } => Some((
            agent_id.as_str(),
            SubAgentStatus::Completed,
            Some(summary.clone()),
        )),
        MailboxMessage::Failed { agent_id, error } => Some((
            agent_id.as_str(),
            SubAgentStatus::Failed(error.clone()),
            Some(error.clone()),
        )),
        MailboxMessage::Interrupted { agent_id, reason } => Some((
            agent_id.as_str(),
            SubAgentStatus::Interrupted(reason.clone()),
            Some(reason.clone()),
        )),
        MailboxMessage::Cancelled { agent_id } => Some((
            agent_id.as_str(),
            SubAgentStatus::Cancelled,
            Some("cancelled".to_string()),
        )),
        _ => None,
    }
}

pub(crate) fn terminal_input_recovery_relevant(app: &App, has_running_agents: bool) -> bool {
    app.is_loading
        || has_running_agents
        || app.is_compacting
        || app.is_purging
        || matches!(app.runtime_turn_status.as_deref(), Some("in_progress"))
        || active_turn_has_running_tool(app)
}

/// Which screen the live terminal is on, for teardown paths that cannot see
/// `App` (the `TerminalCleanupGuard` drop, the panic hook).
///
/// A runtime `/inline` or `/fullscreen` switch moves the terminal after the
/// guard was built, so the guard must read the current screen rather than the
/// one startup chose — otherwise a rolled-back or switched session emits a
/// `LeaveAlternateScreen` for a screen it is not on (or skips the one it is).
static LIVE_ALT_SCREEN: AtomicBool = AtomicBool::new(false);

pub(crate) fn set_live_alt_screen(on_alt_screen: bool) {
    LIVE_ALT_SCREEN.store(on_alt_screen, Ordering::Release);
}

pub(crate) fn live_alt_screen() -> bool {
    LIVE_ALT_SCREEN.load(Ordering::Acquire)
}

/// Rows a full-height inline viewport should request.
///
/// `Viewport::Inline` clamps to the terminal height anyway; asking for the
/// full height is what makes inline mode a drop-in replacement for the alt
/// screen rather than a shrunken strip.
fn inline_viewport_rows(backend: &ColorCompatBackend<Stdout>) -> u16 {
    ratatui::backend::Backend::size(backend)
        .map_or(24, |size| size.height)
        .max(1)
}

/// Build the ratatui terminal for `mode`.
///
/// Inline is the fallible one: `Terminal::with_options` measures the terminal
/// and appends lines to make room for the viewport, so it is the probe. It is
/// deliberately given the *full* terminal height, which makes the anchoring
/// independent of where the cursor happens to be — the newlines it prints
/// scroll whatever was on screen into the host's real scrollback instead of
/// being painted over.
pub(crate) fn build_app_terminal(
    backend: ColorCompatBackend<Stdout>,
    mode: ScreenMode,
) -> io::Result<AppTerminal> {
    match mode {
        ScreenMode::Fullscreen => Terminal::new(backend),
        ScreenMode::Inline => {
            let rows = inline_viewport_rows(&backend);
            Terminal::with_options(
                backend,
                ratatui::TerminalOptions {
                    viewport: ratatui::Viewport::Inline(rows),
                },
            )
        }
    }
}

/// Move the live terminal to `target` in place, rolling back on failure.
///
/// Stock ratatui cannot change an existing terminal's viewport, so the switch
/// rebuilds one over a fresh backend and only adopts it once the rebuild
/// succeeded. That ordering *is* the rollback: on failure the caller's
/// terminal was never touched, so undoing the alternate-screen escape restores
/// the previous mode exactly.
///
/// Nothing is committed to the host scrollback here. Inline mode paints a
/// full-height viewport, so no transcript row ever leaves the live region and
/// `Terminal::insert_before` has nothing to commit — see
/// `docs/CONFIGURATION.md`.
pub(crate) fn switch_screen_mode(
    terminal: &mut AppTerminal,
    app: &mut App,
    target: ScreenMode,
) -> std::result::Result<(), String> {
    let from = app.screen_mode;
    if from == target {
        return Ok(());
    }

    // Everything the previous mode staged must reach the terminal before the
    // escapes below move the cursor out from under it.
    let _ = terminal.backend_mut().flush();

    let carried = terminal.backend().respawn(io::stdout());
    let outcome = transition_screen(
        terminal,
        from,
        target,
        &mut |on_alt_screen| {
            let mut stdout = io::stdout();
            let _ = if on_alt_screen {
                execute!(stdout, EnterAlternateScreen)
            } else {
                execute!(stdout, LeaveAlternateScreen)
            };
            set_live_alt_screen(on_alt_screen);
        },
        move || build_app_terminal(carried, target),
    );

    // Either way the screen changed underneath the app: repaint.
    app.needs_redraw = true;
    if outcome.is_ok() {
        app.screen_mode = target;
        let _ = reset_terminal_viewport(terminal, app.synchronized_output_enabled);
    }
    outcome
}

/// Give an inline session a viewport the size of the terminal it is now in.
///
/// Stock ratatui keeps `Viewport::Inline(rows)` at the rows it was built with,
/// so after the window grows a "full-height" inline viewport would stop at the
/// old height and leave the new rows blank. Rebuild it over the same
/// negotiated backend facts, sized to the event-reported `size` (the
/// `terminal::size()` query can lag a resize — see the `#582` note in the
/// event loop).
///
/// The cursor is parked on row 0 first. A full-height viewport is anchored
/// there, and from row 0 the full height is exactly the room ratatui asks
/// for, so it appends no lines and the host scrollback gains nothing. In
/// inline mode the visible screen is the session's own frame, so nothing of
/// the user's is painted over.
pub(crate) fn refit_inline_viewport(terminal: &mut AppTerminal, size: Size) -> io::Result<()> {
    let _ = terminal.backend_mut().flush();
    let mut backend = terminal.backend().respawn(io::stdout());
    backend.force_size(size);
    backend.set_terminal_size(size);
    ratatui::backend::Backend::set_cursor_position(
        &mut backend,
        ratatui::layout::Position::ORIGIN,
    )?;
    *terminal = build_app_terminal(backend, ScreenMode::Inline)?;
    terminal.backend_mut().clear_forced_size();
    Ok(())
}

/// The fallible half of [`switch_screen_mode`], with the terminal escapes and
/// the rebuild injected so the rollback can be exercised against a fake
/// backend.
///
/// `alt_screen` programs the alternate screen (best effort, like every other
/// teardown escape in this module); `build` is the probe. A failed probe never
/// touched `terminal`, so rollback is exactly one call to `alt_screen` with the
/// previous mode's answer.
fn transition_screen<B, F>(
    terminal: &mut Terminal<B>,
    from: ScreenMode,
    target: ScreenMode,
    alt_screen: &mut dyn FnMut(bool),
    build: F,
) -> std::result::Result<(), String>
where
    B: ratatui::backend::Backend,
    F: FnOnce() -> io::Result<Terminal<B>>,
{
    alt_screen(target.uses_alt_screen());
    match build() {
        Ok(rebuilt) => {
            *terminal = rebuilt;
            Ok(())
        }
        Err(err) => {
            alt_screen(from.uses_alt_screen());
            Err(format!(
                "{} viewport probe failed: {err}; staying in {}",
                target.as_str(),
                from.as_str()
            ))
        }
    }
}

pub(crate) fn pause_terminal(
    terminal: &mut AppTerminal,
    use_alt_screen: bool,
    use_mouse_capture: bool,
    use_bracketed_paste: bool,
) -> Result<()> {
    // Focus reporting is about to be disabled. Fail closed to "focused" so
    // a child process or external editor cannot leave stale background state
    // that later emits a surprise Codewhale notification.
    crate::tui::notifications::set_terminal_focused(true);
    // #443: pop keyboard enhancement flags before handing the terminal
    // to a child process so it doesn't inherit a half-configured input
    // mode. Best-effort — terminals that didn't accept the flags
    // silently ignore the pop. Matches the shutdown and panic paths.
    pop_keyboard_enhancement_flags(terminal.backend_mut());
    disable_alternate_scroll_mode(terminal.backend_mut());
    execute!(terminal.backend_mut(), DisableFocusChange)?;
    disable_raw_mode()?;
    if use_alt_screen {
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        #[cfg(windows)]
        crate::logging::restore_verbose_state();
    }
    if use_mouse_capture {
        execute!(terminal.backend_mut(), DisableMouseCapture)?;
    }
    if use_bracketed_paste {
        disable_bracketed_paste_mode(terminal.backend_mut());
    }
    Ok(())
}

pub(crate) fn resume_terminal(
    terminal: &mut AppTerminal,
    use_alt_screen: bool,
    use_mouse_capture: bool,
    use_bracketed_paste: bool,
    sync_output_enabled: bool,
) -> Result<()> {
    // No trustworthy focus transition exists while reporting is disabled.
    // Resume from the quiet/focused state and wait for a real FocusLost.
    crate::tui::notifications::set_terminal_focused(true);
    enable_raw_mode()?;
    if use_alt_screen {
        execute!(terminal.backend_mut(), EnterAlternateScreen)?;
        // Re-entering alt-screen after mode recovery — suppress verbose
        // CLI logging again so eprintln! doesn't leak into the TUI.
        #[cfg(windows)]
        crate::logging::set_verbose(false);
    }
    recover_terminal_modes(
        terminal.backend_mut(),
        use_mouse_capture,
        use_bracketed_paste,
    );
    // Cache the real terminal size *before* resetting the viewport, so that
    // reset_terminal_viewport → terminal.clear() → autoresize() → backend.size()
    // picks up the cached size instead of falling through to
    // crossterm::terminal::size() which may return stale buffer metadata
    // (especially on Windows after a secondary EnterAlternateScreen).
    if let Ok((cols, rows)) = crossterm::terminal::size() {
        terminal
            .backend_mut()
            .set_terminal_size(Size::new(cols, rows));
    }
    reset_terminal_viewport(terminal, sync_output_enabled)?;
    Ok(())
}

pub(crate) fn reset_terminal_viewport(
    terminal: &mut AppTerminal,
    sync_output_enabled: bool,
) -> Result<()> {
    // Reset scroll margins and origin mode before clearing. Some interactive
    // child processes leave DECSTBM/DECOM behind; if ratatui's diff renderer
    // then writes "row 0", terminals can place it relative to the leaked
    // scroll region and the whole viewport appears shifted down. We
    // deliberately do *not* emit CSI 2J/3J here — see TERMINAL_ORIGIN_RESET
    // for why; the immediately-following ratatui `terminal.clear()` flushes a
    // single clear via the diff renderer, which the alt-screen buffer absorbs
    // without visible flicker on the affected terminals.
    //
    // Wrap the reset+clear sequence in DEC 2026 synchronized-output mode
    // (`\x1b[?2026h` … `\x1b[?2026l`) so GPU-accelerated terminals
    // (Ghostty, VSCode, Kitty, WezTerm) defer rendering until the whole
    // frame is staged. Terminals that don't support it silently ignore.
    // The wrap is opt-out via `synchronized_output = "off"` for terminals
    // that mishandle the sequence (Ptyxis 50.x on VTE 0.84.x flashes the
    // whole viewport on each wrapped frame).
    if sync_output_enabled {
        let _ = terminal.backend_mut().write_all(BEGIN_SYNC_UPDATE);
    }

    let result = (|| -> Result<()> {
        terminal.backend_mut().write_all(TERMINAL_ORIGIN_RESET)?;
        terminal.clear()?;
        Ok(())
    })();

    // Always end the synchronized update, regardless of success or failure.
    if sync_output_enabled {
        let _ = terminal.backend_mut().write_all(END_SYNC_UPDATE);
    }
    let _ = terminal.backend_mut().flush();
    result
}

pub(crate) fn push_keyboard_enhancement_flags<W: Write>(writer: &mut W) {
    // crossterm's PushKeyboardEnhancementFlags command unconditionally
    // returns Unsupported on Windows (is_ansi_code_supported() == false), so
    // the ANSI escape is written directly on that platform. Modern Windows
    // terminals (VSCode integrated terminal, Windows Terminal ≥1.17) honour
    // the kitty keyboard protocol but crossterm's event reader does not
    // decode CSI u sequences on Windows (issue #1599). Write \033[>0u to
    // probe the protocol without enabling any flags — Enter stays as \n.
    #[cfg(windows)]
    {
        if let Err(err) = write!(writer, "\x1b[>0u").and_then(|()| writer.flush()) {
            tracing::debug!(
                target: "kitty_keyboard",
                ?err,
                "PushKeyboardEnhancementFlags direct write failed on Windows"
            );
        }
    }
    #[cfg(not(windows))]
    if let Err(err) = execute!(
        writer,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    ) {
        tracing::debug!(
            target: "kitty_keyboard",
            ?err,
            "PushKeyboardEnhancementFlags ignored (terminal lacks support)"
        );
    }
}

pub(crate) fn pop_keyboard_enhancement_flags<W: Write>(writer: &mut W) {
    // Mirror of push_keyboard_enhancement_flags: crossterm's
    // PopKeyboardEnhancementFlags also has is_ansi_code_supported() == false
    // on Windows, so write the pop escape directly to restore the terminal to
    // its pre-launch keyboard mode.
    // pub(crate) so the panic hook in main.rs and external_editor.rs can
    // also call the Windows-aware path instead of using the raw crossterm
    // execute!() macro which silently no-ops on Windows.
    #[cfg(windows)]
    {
        if let Err(err) = write!(writer, "\x1b[<1u").and_then(|()| writer.flush()) {
            tracing::debug!(
                target: "kitty_keyboard",
                ?err,
                "PopKeyboardEnhancementFlags direct write failed on Windows"
            );
        }
    }
    #[cfg(not(windows))]
    let _ = execute!(writer, PopKeyboardEnhancementFlags);
}

pub(crate) fn set_alternate_scroll_mode<W: Write>(writer: &mut W, enabled: bool) {
    let sequence = if enabled {
        ENABLE_ALT_SCROLL_MODE
    } else {
        DISABLE_ALT_SCROLL_MODE
    };
    if let Err(err) = writer.write_all(sequence).and_then(|()| writer.flush()) {
        tracing::debug!(
            ?err,
            enabled,
            "alternate-scroll terminal mode change ignored"
        );
    }
}

pub(crate) fn disable_alternate_scroll_mode<W: Write>(writer: &mut W) {
    set_alternate_scroll_mode(writer, false);
}

/// Best-effort terminal restoration for emergency exit paths
/// (panic hook, signal handlers). Mirrors the normal teardown in
/// `run_event_loop` but tolerates any subset of modes not actually being
/// active — every step is discarded on failure so a half-initialized TUI
/// (e.g. SIGINT during startup before `EnterAlternateScreen`) still gets
/// raw mode + kitty keyboard flags cleared, which is what causes the
/// `^[[>5u` shell pollution reported in #1583.
pub fn emergency_restore_terminal() {
    let mut stdout = std::io::stdout();
    crate::tui::cursor_accent::restore_cursor_accent();
    pop_keyboard_enhancement_flags(&mut stdout);
    disable_alternate_scroll_mode(&mut stdout);
    let _ = execute!(stdout, DisableFocusChange);
    disable_bracketed_paste_mode(&mut stdout);
    let _ = execute!(stdout, DisableMouseCapture);
    let _ = disable_raw_mode();
    let _ = execute!(stdout, LeaveAlternateScreen);
}

/// On Windows, ensure the console input handle has `ENABLE_WINDOW_INPUT`
/// (0x0008) set. crossterm's `enable_raw_mode()` removes this flag, which
/// breaks IME composition (Chinese/Japanese/Korean input methods cannot
/// commit characters) on some Windows configurations (e.g. Windows Terminal
/// in conhost compatibility mode, or the legacy console with VT input).
///
/// Best-effort and idempotent. Silently ignored if the console handle or
/// mode query fails.
#[cfg(target_os = "windows")]
pub(crate) fn enable_windows_ime_console_mode() {
    use windows::Win32::System::Console::CONSOLE_MODE;
    const ENABLE_WINDOW_INPUT: CONSOLE_MODE = CONSOLE_MODE(0x0008);

    // SAFETY: Win32 console API is safe to call from any thread.
    // Failures (console handle invalid, mode query fails) are silently
    // ignored — this is a best-effort IME compatibility tweak.
    unsafe {
        let Ok(handle) = GetStdHandle(windows::Win32::System::Console::STD_INPUT_HANDLE) else {
            return;
        };
        let mut mode = CONSOLE_MODE(0);
        if GetConsoleMode(handle, &mut mode).is_err() {
            return;
        }
        if mode.0 & ENABLE_WINDOW_INPUT.0 == 0 {
            let _ = SetConsoleMode(handle, mode | ENABLE_WINDOW_INPUT);
        }
    }
}

/// Re-establish terminal mode flags. Idempotent and best-effort: each
/// underlying flag is silently discarded by terminals that don't support
/// it, and a single flag's failure doesn't prevent later flags from being
/// attempted.
///
/// **Canonical location for terminal-mode setup.** If you add a new mode
/// flag at startup or in `resume_terminal`, add it here too — `FocusGained`
/// recovery calls this and will silently fall behind otherwise.
///
/// Excluded by design: raw mode and the alternate screen — those persist
/// across focus events and are only re-established by `resume_terminal`
/// after a suspension, which always runs a separate path.
///
pub(crate) fn recover_terminal_modes<W: Write>(
    writer: &mut W,
    use_mouse_capture: bool,
    use_bracketed_paste: bool,
) {
    #[cfg(target_os = "windows")]
    enable_windows_ime_console_mode();

    pop_keyboard_enhancement_flags(writer);
    push_keyboard_enhancement_flags(writer);
    // DECSET 1007 converts wheel input into arrow keys. While mouse capture
    // is active, mouse reporting is the authoritative wheel channel and
    // terminals disagree about precedence (iTerm2 converts — #5223), so keep
    // 1007 off; #4026 already leaves it off without mouse capture.
    disable_alternate_scroll_mode(writer);
    if use_mouse_capture && let Err(err) = execute!(writer, EnableMouseCapture) {
        tracing::debug!(?err, "EnableMouseCapture ignored");
    }
    if use_bracketed_paste {
        try_enable_bracketed_paste_mode(writer);
    }
    if let Err(err) = execute!(writer, EnableFocusChange) {
        tracing::debug!(?err, "EnableFocusChange ignored");
    }
}

pub(crate) fn try_enable_bracketed_paste_mode<W: Write>(writer: &mut W) -> bool {
    match execute!(writer, EnableBracketedPaste) {
        Ok(()) => true,
        Err(err) => {
            tracing::debug!(?err, "EnableBracketedPaste ignored");
            false
        }
    }
}

pub(crate) fn disable_bracketed_paste_mode<W: Write>(writer: &mut W) {
    if let Err(err) = execute!(writer, DisableBracketedPaste) {
        tracing::debug!(?err, "DisableBracketedPaste ignored");
    }
}

pub(crate) fn terminal_event_needs_viewport_recapture(evt: &Event) -> bool {
    matches!(evt, Event::FocusGained)
}

pub(crate) fn terminal_pause_has_live_owner(app: &App) -> bool {
    app.active_cell.as_ref().is_some_and(|active| {
        active.entries().iter().any(|cell| {
            matches!(
                cell,
                HistoryCell::Tool(ToolCell::Exec(exec)) if exec.status == ToolStatus::Running
            )
        })
    })
}

pub(crate) fn active_poll_ms(app: &App) -> u64 {
    if app.low_motion {
        96
    } else {
        UI_ACTIVE_POLL_MS
    }
}

pub(crate) fn idle_poll_ms(app: &App) -> u64 {
    if app.low_motion { 120 } else { UI_IDLE_POLL_MS }
}

#[cfg(test)]
mod screen_mode_tests {
    use super::*;
    use ratatui::backend::TestBackend;

    fn probe_failure() -> io::Error {
        io::Error::other("terminal refused the inline viewport")
    }

    #[test]
    fn failed_probe_rolls_the_screen_back_and_says_why() {
        let mut terminal =
            Terminal::new(TestBackend::new(20, 6)).expect("fullscreen test terminal");
        let mut alt_screen_writes: Vec<bool> = Vec::new();

        let error = transition_screen(
            &mut terminal,
            ScreenMode::Fullscreen,
            ScreenMode::Inline,
            &mut |on_alt_screen| alt_screen_writes.push(on_alt_screen),
            || Err(probe_failure()),
        )
        .expect_err("a failing probe must not report a switch");

        assert!(
            error.contains("inline viewport probe failed"),
            "message must name the probe that failed: {error}"
        );
        assert!(
            error.contains("terminal refused the inline viewport"),
            "message must carry the terminal's own reason: {error}"
        );
        assert!(
            error.contains("staying in fullscreen"),
            "message must name the mode the user is left in: {error}"
        );
        // Left the alt screen for the probe, then went straight back to it.
        assert_eq!(alt_screen_writes, vec![false, true]);
        // The caller's terminal is the one it started with.
        assert_eq!(terminal.get_frame().area(), Rect::new(0, 0, 20, 6));
    }

    #[test]
    fn successful_probe_adopts_the_rebuilt_terminal() {
        let mut terminal =
            Terminal::new(TestBackend::new(20, 6)).expect("fullscreen test terminal");
        let mut alt_screen_writes: Vec<bool> = Vec::new();

        transition_screen(
            &mut terminal,
            ScreenMode::Fullscreen,
            ScreenMode::Inline,
            &mut |on_alt_screen| alt_screen_writes.push(on_alt_screen),
            || {
                Terminal::with_options(
                    TestBackend::new(20, 6),
                    ratatui::TerminalOptions {
                        viewport: ratatui::Viewport::Inline(3),
                    },
                )
                .map_err(|err| io::Error::other(err.to_string()))
            },
        )
        .expect("a successful probe must switch");

        assert_eq!(alt_screen_writes, vec![false], "no rollback write");
        assert_eq!(
            terminal.get_frame().area().height,
            3,
            "inline viewport adopted"
        );
    }

    #[test]
    fn inline_viewport_asks_for_the_full_terminal_height() {
        // Inline is a drop-in for the alt screen, not a strip: the viewport is
        // the whole terminal, which is also what makes its anchoring
        // independent of where the cursor happened to be.
        let backend = crate::tui::color_compat::ColorCompatBackend::new(
            io::stdout(),
            crate::palette::ColorDepth::TrueColor,
            crate::palette::PaletteMode::Dark,
        );
        let mut backend = backend;
        backend.set_terminal_size(Size::new(80, 24));
        assert_eq!(inline_viewport_rows(&backend), 24);
    }
}
