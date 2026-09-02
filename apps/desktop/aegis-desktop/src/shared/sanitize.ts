// Helpers for rendering potentially-HTML strings from the backend.
//
// The server stores `item.name` (and similar label fields) verbatim
// from als-resolver, so an uploaded ALS may contain inline markup
// like `<br />` or `<b>`. Rendering that with
// `dangerouslySetInnerHTML` is the only way to show it as HTML, but
// the input is not trusted — sanitizer the string before it reaches
// the DOM so a stray `<script>` or `onerror=` attribute cannot run.

import DOMPurify from "dompurify";

/**
 * Whitelist of tags we knowingly want to render. Anything outside
 * this set — `<script>`, `<iframe>`, inline `style`, … — is dropped.
 * Extend this list when a new safe tag is genuinely needed; do NOT
 * loosen it for one-off content.
 */
const ALLOWED_TAGS = ["br", "b", "strong", "i", "em", "u", "p", "span"];

/**
 * Whitelist of attributes that survive sanitization. By default we
 * keep none — the whitelisted tags above don't need any attribute
 * to render correctly, and leaving `href`/`src` open would invite
 * `javascript:` payloads.
 */
const ALLOWED_ATTR: string[] = [];

/**
 * Sanitize a backend-supplied string and return it as HTML ready to
 * drop into `dangerouslySetInnerHTML`. Strips dangerous tags,
 * attributes, and protocols, then runs DOMPurify's built-in hook
 * for node removal — anything more exotic should be added to
 * `ALLOWED_TAGS` rather than bypassed here.
 */
export function sanitizeHtml(input: string): string {
  return DOMPurify.sanitize(input, {
    ALLOWED_TAGS,
    ALLOWED_ATTR,
    // Defence-in-depth: forbid URI schemes that can run JS even if a
    // tag slips through (e.g. an `<a href>` we didn't whitelist).
    ALLOWED_URI_REGEXP: /^(?:(?:https?|mailto|tel):)/i,
    USE_PROFILES: { html: true },
  });
}