import { describe, expect, it, vi } from "vitest";

vi.mock("next/font/local", () => ({ default: () => ({ variable: "font" }) }));

const { default: LocaleLayout } = await import("@/app/[locale]/layout");

function render(locale: string) {
  return LocaleLayout({ children: null, params: Promise.resolve({ locale }) });
}

describe("locale layout", () => {
  // Middleware skips dotted paths, so `/wp-login.php` reaches `[locale]`.
  it("answers not-found for a segment that is not a locale", async () => {
    await expect(render("wp-login.php")).rejects.toMatchObject({
      digest: expect.stringContaining("404"),
    });
  });

  it("renders a real locale", async () => {
    const element = await render("en");
    expect(element.props.lang).toBe("en");
  });
});
