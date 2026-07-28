# v0.9.2 real-session recording (#4906)
#
# The tape referenced by docs/releases/v0.9.2-media-plan.md step 3. It replays
# the four beats that plan specifies, at the 120x32 / Blue Stage dark grid the
# canonical screenshot already uses, so the moving and still surfaces read as
# one product.
#
# THIS TAPE RECORDS A REAL SESSION. It types real commands into the real
# release binary and captures whatever actually happens. It does not stage,
# fake, or reconstruct output. If a take is unimpressive, change the task and
# re-record — do not edit frames. That constraint is the whole point of #4906.
#
# Prerequisites (see the media plan for the full procedure):
#   1. `scripts/release/install-dogfood.sh` has installed the exact
#      release-candidate SHA named in the completion ledger.
#   2. A sealed environment is exported — HOME and CODEWHALE_HOME both point
#      into a scratch dir, and no provider key is in the environment.
#   3. `ollama serve` is running on loopback with the model already pulled, so
#      the recording never waits on a download and never touches a paid route.
#
# Run:
#   scripts/media/record-session.sh
#
# Do not run `vhs` on this tape directly unless you have already sealed the
# environment yourself — the wrapper exists so a recording cannot accidentally
# capture your real home directory, shell history, or credentials.

Output first-fleet-session.gif

Require codewhale

Set Shell "bash"
Set FontSize 16
Set Width 1280
Set Height 720
Set Padding 0
Set Framerate 24

# Terminal theme, NOT the product theme. "Blue Stage dark" in the media plan is
# a Codewhale theme: the TUI emits its own colors and paints every cell it
# owns, so the emulator palette must not try to reproduce it. Setting a
# hand-written "BlueStage" palette here would be inventing colors and would
# fight the real ones. A plain dark terminal is the correct backdrop; Blue
# Stage is selected inside the session, from the sealed config the wrapper
# writes.
Set Theme "Builtin Dark"

# --- Beat 1: install / launch in a scratch repository -----------------------
Type "cd /tmp/cw-media-demo && codewhale --version"
Enter
Sleep 2s

# --- Beat 2: first keyless session, read-only Plan-mode exploration ---------
Type "codewhale"
Enter
Sleep 6s
Type "/mode plan"
Enter
Sleep 3s
Type "What does this repository do? Read only — do not change anything."
Enter
Sleep 25s

# --- Beat 3: provider connection (local Ollama route) -----------------------
# Provider, model, and reasoning tier are visible in the footer after this.
Type "/model"
Enter
Sleep 4s
Escape
Sleep 1s

# --- Beat 4: one Fleet workflow, ending on its receipt ----------------------
Type "/quit"
Enter
Sleep 2s
Type "codewhale fleet init"
Enter
Sleep 5s
Type "codewhale fleet run --dry-run"
Enter
Sleep 12s

# Hold on the final receipt so the last frame is a result, not a prompt.
Sleep 4s
