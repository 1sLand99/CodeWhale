/* codewhale-brand: an explicit identity lockup for the DSH skin.
 *
 * DSH renders this component through its additive shell.overlay Slot. The
 * lockup is pointer-inert, compact on narrow viewports, uses the active skin
 * tokens, and leaves DSH-owned chrome untouched.
 */
function CodewhaleBrand() {
	var css =
		"#codewhale-brand-lockup{" +
		"position:fixed;top:18px;right:20px;z-index:40;display:flex;align-items:center;gap:10px;min-width:272px;padding:10px 12px;box-sizing:border-box;pointer-events:none;user-select:none;border:1px solid var(--dsw-alias-brand-primary,#6aaef2);border-left:3px solid var(--dsw-alias-state-business-primary,#f6c453);border-radius:12px;background:var(--dsw-alias-bg-layer-1,#0e1729);color:var(--dsw-alias-label-primary,#f6f2e8);box-shadow:0 18px 44px rgba(0,0,0,.38),inset 0 1px 0 rgba(255,255,255,.07);font-family:Inter,ui-sans-serif,-apple-system,BlinkMacSystemFont,\"Segoe UI\",sans-serif;line-height:1;" +
		"}" +
		"#codewhale-brand-mark{" +
		"display:grid;place-items:center;width:34px;height:34px;flex:0 0 auto;border-radius:10px;background:linear-gradient(145deg,var(--dsw-alias-brand-primary,#6aaef2),var(--dsw-alias-state-business-primary,#f6c453));color:#03070d;font-size:18px;box-shadow:0 8px 22px rgba(49,95,216,.34);" +
		"}" +
		"#codewhale-brand-copy{display:flex;min-width:0;flex:1;flex-direction:column;gap:5px;}" +
		"#codewhale-brand-kicker{display:block;color:var(--dsw-alias-state-business-primary,#f6c453);font-size:9px;font-weight:800;letter-spacing:2.1px;white-space:nowrap;}" +
		"#codewhale-brand-name{display:block;color:var(--dsw-alias-label-primary,#f6f2e8);font-size:15px;font-weight:850;letter-spacing:1.7px;white-space:nowrap;}" +
		"#codewhale-brand-bridge{display:block;margin-left:auto;color:var(--dsw-alias-brand-primary,#6aaef2);font-family:ui-monospace,\"SFMono-Regular\",Menlo,Consolas,monospace;font-size:9px;font-weight:750;letter-spacing:.8px;white-space:nowrap;}" +
		"@media(max-width:759px){" +
		"#codewhale-brand-lockup{top:8px;right:8px;min-width:0;padding:7px 9px;gap:7px;}" +
		"#codewhale-brand-mark{width:28px;height:28px;font-size:15px;}" +
		"#codewhale-brand-kicker,#codewhale-brand-bridge{display:none;}" +
		"}";

	return React.createElement(
		React.Fragment,
		null,
		React.createElement("style", { "data-codewhale-brand-style": true }, css),
		React.createElement(
			"aside",
			{
				id: "codewhale-brand-lockup",
				"aria-label": "Whale Brothers. Codewhale connected to DeepSeek Harness.",
			},
			React.createElement(
				"span",
				{ id: "codewhale-brand-mark", "aria-hidden": true },
				"🐋",
			),
			React.createElement(
				"span",
				{ id: "codewhale-brand-copy" },
				React.createElement("span", { id: "codewhale-brand-kicker" }, "WHALE BROTHERS"),
				React.createElement("span", { id: "codewhale-brand-name" }, "CODEWHALE"),
			),
			React.createElement("span", { id: "codewhale-brand-bridge" }, "\u00d7 DEEPSEEK HARNESS"),
		),
	);
}
