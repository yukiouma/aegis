import { describe, expect, it } from "vitest";

import { sanitizeHtml } from "../../shared/sanitize";

describe("sanitizeHtml", () => {
  it("keeps whitelisted tags intact", () => {
    // DOMPurify normalizes `<br />` to `<br>` — same DOM, just a
    // different string. The point is that the tag survives, not the
    // exact serialization.
    expect(sanitizeHtml("hello <br /> world")).toBe("hello <br> world");
    expect(sanitizeHtml("<b>bold</b> text")).toBe("<b>bold</b> text");
    expect(sanitizeHtml("<i>italic</i>")).toBe("<i>italic</i>");
  });

  it("strips <script> tags and their bodies", () => {
    const out = sanitizeHtml("safe<script>alert(1)</script>end");
    expect(out).not.toMatch(/script/i);
    expect(out).not.toContain("alert(1)");
  });

  it("strips inline event handlers", () => {
    const out = sanitizeHtml('<b onclick="alert(1)">click</b>');
    expect(out.toLowerCase()).not.toContain("onclick");
    // The tag itself may survive, but the attribute must be gone.
    expect(out).not.toMatch(/onclick/i);
  });

  it("strips javascript: URLs", () => {
    const out = sanitizeHtml('<a href="javascript:alert(1)">x</a>');
    // `<a>` is not in the allowed-tags list so the tag is dropped
    // outright, but defence-in-depth: even if it survived, the
    // scheme must be stripped.
    expect(out).not.toMatch(/javascript:/i);
  });

  it("strips <iframe> and other dangerous tags", () => {
    const out = sanitizeHtml(
      '<iframe src="https://evil.example"></iframe>safe',
    );
    expect(out).not.toMatch(/iframe/i);
    expect(out).toContain("safe");
  });

  it("returns plain text unchanged when no tags are present", () => {
    expect(sanitizeHtml("just text")).toBe("just text");
  });
});