import "@testing-library/jest-dom/vitest";

// jsdom does not implement window.scrollTo; TanStack Router's scroll
// restoration emits this on every render. Silence it.
window.scrollTo = () => undefined;
