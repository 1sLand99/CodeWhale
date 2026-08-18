/* codewhale-brand: an explicit, reversible identity lockup for the DSH skin.
 *
 * The lockup is appended to body rather than rewriting DSH-owned chrome. It is
 * pointer-inert, compact on narrow viewports, uses the active skin tokens, and
 * is removed with the client plugin effect.
 */
function createCodewhaleBrand() {
	var ID = "codewhale-brand-lockup";
	var COMPACT_AT = 760;
	var root = null;
	var mark = null;
	var kicker = null;
	var bridge = null;
	var readyHandler = null;
	var mounted = false;

	function setStyle(node, css) {
		node.style.cssText = css;
		return node;
	}

	function make(tag, text, css) {
		var node = document.createElement(tag);
		if (text !== null) node.textContent = text;
		return setStyle(node, css);
	}

	function layout() {
		if (!root) return;
		var compact = window.innerWidth < COMPACT_AT;
		root.style.top = compact ? "8px" : "18px";
		root.style.right = compact ? "8px" : "20px";
		root.style.minWidth = compact ? "0" : "272px";
		root.style.padding = compact ? "7px 9px" : "10px 12px";
		root.style.gap = compact ? "7px" : "10px";
		mark.style.width = compact ? "28px" : "34px";
		mark.style.height = compact ? "28px" : "34px";
		mark.style.fontSize = compact ? "15px" : "18px";
		kicker.style.display = compact ? "none" : "block";
		bridge.style.display = compact ? "none" : "block";
	}

	function mount() {
		if (mounted) return true;
		if (!document.body) return false;
		var prior = document.getElementById(ID);
		if (prior && prior.parentNode) prior.parentNode.removeChild(prior);

		root = make("aside", null,
			"position:fixed;top:18px;right:20px;z-index:40;display:flex;align-items:center;gap:10px;min-width:272px;padding:10px 12px;box-sizing:border-box;pointer-events:none;user-select:none;border:1px solid var(--dsw-alias-brand-primary,#6aaef2);border-left:3px solid var(--dsw-alias-state-business-primary,#f6c453);border-radius:12px;background:var(--dsw-alias-bg-layer-1,#0e1729);color:var(--dsw-alias-label-primary,#f6f2e8);box-shadow:0 18px 44px rgba(0,0,0,.38),inset 0 1px 0 rgba(255,255,255,.07);font-family:Inter,ui-sans-serif,-apple-system,BlinkMacSystemFont,\"Segoe UI\",sans-serif;line-height:1;");
		root.id = ID;
		root.setAttribute("aria-label", "Whale Brothers. Codewhale connected to DeepSeek Harness.");

		mark = make("span", "🐋",
			"display:grid;place-items:center;width:34px;height:34px;flex:0 0 auto;border-radius:10px;background:linear-gradient(145deg,var(--dsw-alias-brand-primary,#6aaef2),var(--dsw-alias-state-business-primary,#f6c453));color:#03070d;font-size:18px;box-shadow:0 8px 22px rgba(49,95,216,.34);");
		mark.setAttribute("aria-hidden", "true");

		var copy = make("span", null, "display:flex;min-width:0;flex:1;flex-direction:column;gap:5px;");
		kicker = make("span", "WHALE BROTHERS",
			"display:block;color:var(--dsw-alias-state-business-primary,#f6c453);font-size:9px;font-weight:800;letter-spacing:2.1px;white-space:nowrap;");
		var brand = make("span", "CODEWHALE",
			"display:block;color:var(--dsw-alias-label-primary,#f6f2e8);font-size:15px;font-weight:850;letter-spacing:1.7px;white-space:nowrap;");
		bridge = make("span", "× DEEPSEEK HARNESS",
			"display:block;margin-left:auto;color:var(--dsw-alias-brand-primary,#6aaef2);font-family:ui-monospace,\"SFMono-Regular\",Menlo,Consolas,monospace;font-size:9px;font-weight:750;letter-spacing:.8px;white-space:nowrap;");

		copy.appendChild(kicker);
		copy.appendChild(brand);
		root.appendChild(mark);
		root.appendChild(copy);
		root.appendChild(bridge);
		document.body.appendChild(root);
		window.addEventListener("resize", layout);
		mounted = true;
		layout();
		return true;
	}

	function start() {
		if (document.body) return mount();
		if (!readyHandler) {
			readyHandler = function () { readyHandler = null; mount(); };
			document.addEventListener("DOMContentLoaded", readyHandler, { once: true });
		}
		return true;
	}

	function stop() {
		if (readyHandler) {
			document.removeEventListener("DOMContentLoaded", readyHandler);
			readyHandler = null;
		}
		window.removeEventListener("resize", layout);
		if (root && root.parentNode) root.parentNode.removeChild(root);
		root = null; mark = null; kicker = null; bridge = null; mounted = false;
	}

	var api = { start: start, stop: stop, get mounted() { return mounted; } };
	window.__codewhaleBrand = api;
	return api;
}
