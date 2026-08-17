/* codewhale-ocean: ambient ocean scene behind the DSH web UI.
 *
 * Plain script (no module syntax) so it can be spliced verbatim into the
 * bundle's lib/client.js — dsh-client-modules serves exactly one file per
 * client plugin (`/plugins/<id>/client.js`), so a sibling scene.js would
 * never be fetched. Exposes `createOcean(palette)`; the caller mounts it.
 *
 * Contract: one full-viewport <canvas> (fixed, inset 0, z-index -1,
 * pointer-events none) painted below the app root and above the body
 * background. ~30 fps, paused while the document is hidden, one static
 * frame under prefers-reduced-motion, no per-frame allocations, DPR-aware.
 * Off switch: body class `codewhale-ocean-off` or
 * localStorage["codewhale.ocean"] === "off". `palette` carries
 * { light: {base, accent, ink, dim}, dark: {...} } CSS hex colors taken from
 * the skin token table.
 */
function createOcean(palette) {
	var STORAGE_KEY = "codewhale.ocean";
	var OFF_CLASS = "codewhale-ocean-off";
	var FRAME_MS = 1000 / 30;
	var FISH_COUNT = 14;
	var BUBBLE_COUNT = 26;
	var TAU = Math.PI * 2;
	var MONO = '13px "SF Mono", "JetBrains Mono", "Fira Code", Menlo, Consolas, monospace';

	function isOff() {
		try {
			if (typeof localStorage !== "undefined" && localStorage.getItem(STORAGE_KEY) === "off") return true;
		} catch (_) {}
		return !!(document.body && document.body.classList.contains(OFF_CLASS));
	}

	function hexToRgb(hex) {
		var h = String(hex).replace("#", "");
		if (h.length === 3) h = h[0] + h[0] + h[1] + h[1] + h[2] + h[2];
		var n = parseInt(h, 16);
		if (isNaN(n)) return [128, 128, 128];
		return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
	}
	function mix(a, b, t) {
		return [
			Math.round(a[0] + (b[0] - a[0]) * t),
			Math.round(a[1] + (b[1] - a[1]) * t),
			Math.round(a[2] + (b[2] - a[2]) * t),
		];
	}
	function rgba(c, a) {
		return "rgba(" + c[0] + "," + c[1] + "," + c[2] + "," + a + ")";
	}

	// Per-scheme colour set, precomputed once per scheme switch (no strings
	// are built inside the frame loop).
	function buildColors(scheme) {
		var p = palette[scheme] || palette.dark;
		var base = hexToRgb(p.base);
		var accent = hexToRgb(p.accent);
		var ink = hexToRgb(p.ink);
		var dim = hexToRgb(p.dim);
		var dark = scheme === "dark";
		return {
			dark: dark,
			// Depth gradient: light water reads darker/bluer with depth; dark
			// water is lit faintly from above and falls to the base at the floor.
			top: dark ? rgba(mix(base, accent, 0.09), 1) : rgba(base, 1),
			bottom: dark ? rgba(base, 1) : rgba(mix(base, accent, 0.11), 1),
			whaleNear: dark ? mix(dim, accent, 0.35) : mix(ink, accent, 0.25),
			whaleFar: dark ? mix(dim, base, 0.25) : mix(dim, accent, 0.2),
			eye: base,
			fish: dark ? mix(accent, dim, 0.15) : mix(accent, ink, 0.1),
			bubble: dark ? mix(dim, accent, 0.4) : mix(accent, dim, 0.3),
		};
	}

	function detectScheme() {
		var cs = document.documentElement.style.colorScheme;
		if (cs === "dark" || cs === "light") return cs;
		if (document.body && document.body.hasAttribute("data-ds-dark-theme")) return "dark";
		if (typeof matchMedia === "function" && matchMedia("(prefers-color-scheme: dark)").matches) return "dark";
		return "light";
	}

	function reducedMotion() {
		return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
	}

	// Deterministic PRNG so two frames of the same seed are comparable and
	// the scene does not depend on Math.random ordering.
	var seed = 0x2f6e2b1;
	function rand() {
		seed = (seed * 1664525 + 1013904223) >>> 0;
		return seed / 4294967296;
	}

	// ---- Whale --------------------------------------------------------------
	// Unit-space silhouette facing +x: fluke tip at x=0, snout at x=1, spine on
	// y=0. Drawn with the CTM scaled to the whale's length, so every number
	// here is a proportion. The fluke is added under a rotation about the tail
	// stock (0.11, 0) — canvas paths capture the CTM per segment, so the flick
	// needs no point math.
	function tracePectoral(g) {
		// long, swept-back flipper hung under the forebody (humpback cue)
		g.moveTo(0.79, 0.105);
		g.bezierCurveTo(0.73, 0.15, 0.6, 0.235, 0.5, 0.245);
		g.bezierCurveTo(0.56, 0.2, 0.64, 0.12, 0.7, 0.085);
		g.closePath();
	}
	function traceWhale(g, flick) {
		// blunt forehead, long flat back, concave peduncle, then the flukes
		g.moveTo(1.0, 0.03);
		g.bezierCurveTo(1.0, -0.04, 0.96, -0.11, 0.87, -0.13);
		g.bezierCurveTo(0.76, -0.15, 0.62, -0.14, 0.5, -0.13);
		// low falcate dorsal fin, tip swept back
		g.bezierCurveTo(0.47, -0.14, 0.45, -0.165, 0.43, -0.17);
		g.bezierCurveTo(0.425, -0.15, 0.415, -0.135, 0.4, -0.125);
		// peduncle narrows into the tail stock
		g.bezierCurveTo(0.31, -0.11, 0.21, -0.055, 0.15, -0.03);
		// flukes: convex leading edges, concave trailing edges, pointed tips,
		// held with a slight upward lift like a slow dive
		g.save();
		g.translate(0.15, 0);
		g.rotate(-0.1 + flick);
		g.translate(-0.15, 0);
		g.bezierCurveTo(0.1, -0.08, 0.03, -0.12, -0.02, -0.15);
		g.bezierCurveTo(0.04, -0.11, 0.07, -0.045, 0.09, 0.0);
		g.bezierCurveTo(0.07, 0.045, 0.04, 0.11, -0.02, 0.15);
		g.bezierCurveTo(0.03, 0.12, 0.1, 0.08, 0.15, 0.03);
		g.restore();
		// belly and jaw
		g.bezierCurveTo(0.22, 0.06, 0.34, 0.12, 0.5, 0.135);
		g.bezierCurveTo(0.7, 0.15, 0.88, 0.14, 0.96, 0.1);
		g.bezierCurveTo(0.99, 0.08, 1.0, 0.055, 1.0, 0.03);
		g.closePath();
	}

	// One crossing = one cubic bezier from off-screen to off-screen. Fields
	// live in a flat Float64Array: [x0,y0,x1,y1,x2,y2,x3,y3,dir,dur,start,len,phase]
	function Whale(near) {
		this.near = near;
		this.p = new Float64Array(13);
		this.dir = 1;
	}
	Whale.prototype.reset = function (W, H, t, first) {
		var p = this.p;
		var near = this.near;
		var len = near ? Math.max(220, Math.min(0.3 * W, 520)) : Math.max(120, Math.min(0.17 * W, 300));
		this.dir = rand() < 0.5 ? -1 : 1;
		var yBand0 = near ? 0.42 : 0.2;
		var yBand1 = near ? 0.9 : 0.6;
		var y0 = H * (yBand0 + rand() * (yBand1 - yBand0));
		var y3 = H * (yBand0 + rand() * (yBand1 - yBand0));
		var margin = len * 1.2;
		var xa = -margin, xb = W + margin;
		if (this.dir < 0) { xa = W + margin; xb = -margin; }
		p[0] = xa; p[1] = y0;
		p[2] = xa + (xb - xa) * (0.25 + rand() * 0.15); p[3] = y0 + (rand() - 0.5) * H * 0.35;
		p[4] = xa + (xb - xa) * (0.6 + rand() * 0.15); p[5] = y3 + (rand() - 0.5) * H * 0.35;
		p[6] = xb; p[7] = y3;
		p[9] = (near ? 70 : 105) * (0.85 + rand() * 0.3); // seconds per crossing
		// First whale starts mid-crossing so the scene is never empty on load.
		p[10] = first ? t - p[9] * (0.3 + rand() * 0.25) : t + (near ? 4 : 6) + rand() * 10;
		p[11] = len;
		p[12] = rand() * TAU;
	};
	Whale.prototype.draw = function (g, t, W, H, colors, alpha) {
		var p = this.p;
		var u = (t - p[10]) / p[9];
		if (u < 0) return;
		if (u > 1) { this.reset(W, H, t, false); return; }
		// ease in/out very slightly so entry/exit are calm
		var v = 1 - u, v2 = v * v, u2 = u * u;
		var x = v2 * v * p[0] + 3 * v2 * u * p[2] + 3 * v * u2 * p[4] + u2 * u * p[6];
		var y = v2 * v * p[1] + 3 * v2 * u * p[3] + 3 * v * u2 * p[5] + u2 * u * p[7];
		var dx = 3 * v2 * (p[2] - p[0]) + 6 * v * u * (p[4] - p[2]) + 3 * u2 * (p[6] - p[4]);
		var dy = 3 * v2 * (p[3] - p[1]) + 6 * v * u * (p[5] - p[3]) + 3 * u2 * (p[7] - p[5]);
		var ang = Math.atan2(dy, dx);
		if (this.dir < 0) ang = ang - Math.PI; // mirrored body, keep pitch small
		ang = Math.max(-0.28, Math.min(0.28, ang > Math.PI ? ang - TAU : ang < -Math.PI ? ang + TAU : ang));
		// gentle body roll + breathing
		var bob = Math.sin(t * 0.45 + p[12]) * p[11] * 0.012;
		// swim stroke: slow sway with an occasional deeper flick
		var burst = Math.sin(t * 0.21 + p[12]);
		burst = burst > 0 ? burst * burst * burst * burst : 0;
		var flick = Math.sin(t * 1.15 + p[12]) * (0.05 + 0.13 * burst);
		var L = p[11];
		g.save();
		g.translate(x, y + bob);
		g.rotate(ang);
		g.scale(this.dir * L, L);
		g.globalAlpha = alpha;
		g.beginPath();
		traceWhale(g, flick);
		g.fill();
		g.beginPath();
		tracePectoral(g);
		g.fill();
		// eye: a pinprick of water colour, only on the near whale
		if (this.near) {
			g.globalAlpha = alpha * 0.9;
			g.fillStyle = colors.eyeStyle;
			g.beginPath();
			g.arc(0.905, -0.035, 0.011, 0, TAU);
			g.fill();
		}
		g.restore();
	};

	// ---- Fish school (flocking-lite) -----------------------------------------
	// Layout: [x, y, vx, vy, ox, oy, size, phase, facing]
	var FS = 9;
	function initFish(fish, W, H) {
		for (var i = 0; i < FISH_COUNT; i++) {
			var o = i * FS;
			var a = rand() * TAU, r = 30 + rand() * 90;
			fish[o + 4] = Math.cos(a) * r * 1.6;
			fish[o + 5] = Math.sin(a) * r * 0.55;
			fish[o + 0] = W * 0.5 + fish[o + 4];
			fish[o + 1] = H * 0.55 + fish[o + 5];
			fish[o + 2] = 0; fish[o + 3] = 0;
			fish[o + 6] = 0.85 + rand() * 0.4;
			fish[o + 7] = rand() * TAU;
			fish[o + 8] = 1;
		}
	}
	function stepFish(fish, t, dt, W, H) {
		// leader on a slow lissajous inside the middle band
		var lx = W * (0.5 + 0.36 * Math.sin(t * 0.055));
		var ly = H * (0.5 + 0.2 * Math.sin(t * 0.09 + 1.3));
		for (var i = 0; i < FISH_COUNT; i++) {
			var o = i * FS;
			var wob = fish[o + 7];
			var tx = lx + fish[o + 4] + Math.sin(t * 0.7 + wob) * 6;
			var ty = ly + fish[o + 5] + Math.cos(t * 0.9 + wob) * 4;
			var ax = (tx - fish[o]) * 0.9, ay = (ty - fish[o + 1]) * 0.9;
			fish[o + 2] += ax * dt; fish[o + 3] += ay * dt;
			// damping + speed cap
			fish[o + 2] *= 0.985; fish[o + 3] *= 0.985;
			var sp = Math.sqrt(fish[o + 2] * fish[o + 2] + fish[o + 3] * fish[o + 3]);
			var cap = 55;
			if (sp > cap) { fish[o + 2] *= cap / sp; fish[o + 3] *= cap / sp; }
			fish[o] += fish[o + 2] * dt; fish[o + 1] += fish[o + 3] * dt;
			// facing with hysteresis so glyphs don't flip-flop
			if (fish[o + 2] > 6) fish[o + 8] = 1; else if (fish[o + 2] < -6) fish[o + 8] = -1;
		}
	}
	function drawFish(g, fish, alpha) {
		g.font = MONO;
		g.textBaseline = "middle";
		g.textAlign = "center";
		for (var i = 0; i < FISH_COUNT; i++) {
			var o = i * FS;
			var big = (i % 5) === 0;
			var right = fish[o + 8] > 0;
			var glyph = big ? (right ? "><o>" : "<o><") : (right ? "><>" : "<><");
			g.globalAlpha = alpha * (0.55 + 0.45 * (fish[o + 6] - 0.85) / 0.4);
			g.save();
			g.translate(fish[o], fish[o + 1]);
			g.scale(fish[o + 6], fish[o + 6]);
			g.fillText(glyph, 0, 0);
			g.restore();
		}
	}

	// ---- Bubbles ---------------------------------------------------------------
	// Layout: [x, y, r, speed, phase]
	var BS = 5;
	function resetBubble(b, o, W, H, fresh) {
		b[o] = rand() * W;
		b[o + 1] = fresh ? rand() * H : H + 10 + rand() * 40;
		b[o + 2] = 0.8 + rand() * 1.9;
		b[o + 3] = 9 + rand() * 14;
		b[o + 4] = rand() * TAU;
	}
	function initBubbles(b, W, H) {
		for (var i = 0; i < BUBBLE_COUNT; i++) resetBubble(b, i * BS, W, H, true);
	}
	function stepBubbles(b, t, dt, W, H) {
		for (var i = 0; i < BUBBLE_COUNT; i++) {
			var o = i * BS;
			b[o + 1] -= b[o + 3] * dt;
			b[o] += Math.sin(t * 0.8 + b[o + 4]) * 6 * dt;
			if (b[o + 1] < -12) resetBubble(b, o, W, H, false);
		}
	}
	function drawBubbles(g, b, alpha) {
		g.lineWidth = 1;
		for (var i = 0; i < BUBBLE_COUNT; i++) {
			var o = i * BS;
			g.globalAlpha = alpha * (0.35 + 0.4 * (b[o + 2] - 0.8) / 1.9);
			g.beginPath();
			g.arc(b[o], b[o + 1], b[o + 2], 0, TAU);
			g.stroke();
		}
	}

	// ---- Scene ------------------------------------------------------------------
	var canvas = null, g = null;
	var W = 0, H = 0, dpr = 1;
	var colors = null, scheme = null;
	var whales = [new Whale(false), new Whale(true)];
	var fish = new Float64Array(FISH_COUNT * FS);
	var bubbles = new Float64Array(BUBBLE_COUNT * BS);
	var gradient = null;
	var raf = 0, running = false, mounted = false, lastFrame = 0, lastT = 0, t0 = 0;
	var intensity = 1;
	var staticOnly = false;

	function resize() {
		var w = window.innerWidth, h = window.innerHeight;
		var d = Math.min(2, window.devicePixelRatio || 1);
		if (w === W && h === H && d === dpr && gradient) return;
		var first = W === 0;
		W = w; H = h; dpr = d;
		canvas.width = Math.max(1, Math.round(W * dpr));
		canvas.height = Math.max(1, Math.round(H * dpr));
		g.setTransform(dpr, 0, 0, dpr, 0, 0);
		rebuildGradient();
		if (first) {
			var t = (performance.now() - t0) / 1000;
			whales[0].reset(W, H, t, false);
			whales[1].reset(W, H, t, true);
			initFish(fish, W, H);
			initBubbles(bubbles, W, H);
		}
	}
	function rebuildGradient() {
		if (!g || !colors) return;
		gradient = g.createLinearGradient(0, 0, 0, H);
		gradient.addColorStop(0, colors.top);
		gradient.addColorStop(1, colors.bottom);
	}
	function applyScheme(next) {
		if (next !== "dark") next = "light";
		if (next === scheme && colors) return;
		scheme = next;
		colors = buildColors(next);
		colors.whaleNearStyle = rgba(colors.whaleNear, 1);
		colors.whaleFarStyle = rgba(colors.whaleFar, 1);
		colors.eyeStyle = rgba(colors.eye, 1);
		colors.fishStyle = rgba(colors.fish, 1);
		colors.bubbleStyle = rgba(colors.bubble, 1);
		rebuildGradient();
	}

	function frame(now) {
		raf = 0;
		if (!running) return;
		if (now - lastFrame < FRAME_MS - 1) { raf = requestAnimationFrame(frame); return; }
		lastFrame = now;
		var t = (now - t0) / 1000;
		var dt = Math.min(0.1, t - lastT);
		lastT = t;
		step(t, dt);
		paint(t);
		if (!staticOnly) raf = requestAnimationFrame(frame);
	}
	function step(t, dt) {
		stepFish(fish, t, dt, W, H);
		stepBubbles(bubbles, t, dt, W, H);
	}
	function paint(t) {
		g.globalAlpha = 1;
		g.fillStyle = gradient;
		g.fillRect(0, 0, W, H);
		var a = intensity;
		g.fillStyle = colors.whaleFarStyle;
		whales[0].draw(g, t, W, H, colors, a * (colors.dark ? 0.32 : 0.3));
		g.strokeStyle = colors.bubbleStyle;
		drawBubbles(g, bubbles, a * (colors.dark ? 0.55 : 0.5));
		g.fillStyle = colors.fishStyle;
		drawFish(g, fish, a * (colors.dark ? 0.85 : 0.9));
		g.fillStyle = colors.whaleNearStyle;
		whales[1].draw(g, t, W, H, colors, a * (colors.dark ? 0.62 : 0.5));
		g.globalAlpha = 1;
	}

	function onVisibility() {
		if (document.hidden) { if (raf) { cancelAnimationFrame(raf); raf = 0; } }
		else if (running && !raf && !staticOnly) { lastFrame = 0; lastT = (performance.now() - t0) / 1000; raf = requestAnimationFrame(frame); }
	}

	function mount() {
		if (mounted) return true;
		if (isOff()) return false;
		canvas = document.createElement("canvas");
		canvas.setAttribute("aria-hidden", "true");
		canvas.setAttribute("data-codewhale-ocean", "");
		canvas.style.cssText = "position:fixed;inset:0;width:100vw;height:100vh;z-index:-1;pointer-events:none;display:block;";
		g = canvas.getContext("2d", { alpha: false });
		if (!g) return false;
		document.body.insertBefore(canvas, document.body.firstChild);
		t0 = performance.now();
		applyScheme(detectScheme());
		resize();
		window.addEventListener("resize", resize);
		document.addEventListener("visibilitychange", onVisibility);
		mounted = true;
		return true;
	}

	function start() {
		if (!document.body) {
			document.addEventListener("DOMContentLoaded", function () { start(); }, { once: true });
			return true;
		}
		if (!mount()) return false;
		staticOnly = reducedMotion();
		running = true;
		if (staticOnly) {
			// one calm frame, then nothing moves: settle the school first so
			// the still image reads as a school, not a spawn point
			var t = 0;
			for (var i = 0; i < 90; i++) { t += 1 / 30; step(t, 1 / 30); }
			paint(t);
			return true;
		}
		if (!raf) { lastFrame = 0; lastT = (performance.now() - t0) / 1000; raf = requestAnimationFrame(frame); }
		return true;
	}
	function stop() {
		running = false;
		if (raf) { cancelAnimationFrame(raf); raf = 0; }
		if (canvas && canvas.parentNode) canvas.parentNode.removeChild(canvas);
		window.removeEventListener("resize", resize);
		document.removeEventListener("visibilitychange", onVisibility);
		mounted = false; W = 0; H = 0; gradient = null;
	}
	function setIntensity(v) {
		intensity = Math.max(0, Math.min(1, Number(v) || 0));
		if (staticOnly && running) paint(3);
	}
	function setScheme(next) {
		applyScheme(next);
		if (staticOnly && running) paint(3);
	}

	var api = { start: start, stop: stop, setIntensity: setIntensity, setScheme: setScheme, isOff: isOff, get running() { return running; } };
	window.__codewhaleOcean = api;
	return api;
}
