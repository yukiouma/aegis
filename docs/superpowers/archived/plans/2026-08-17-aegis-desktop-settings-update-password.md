# Settings Update Password Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an "Update password" button to the existing Settings page that opens a password-entry dialog, then a confirmation dialog, then updates the user's credential and logs them out.

**Architecture:** Keep all UI in `SettingsPage.tsx` (currently small). Add one new `useUpdatePassword` mutation hook in `features/settings/data/update-password.ts` mirroring the existing `logout.ts` pattern. Drive two stacked MUI `Dialog`s via local `useState`. Reuse the existing `useLogout` flow for the post-success step.

**Tech Stack:** React 19, TypeScript 5.8, MUI Dialog, `@tanstack/react-query` 5, `@aegis/ui/i18n` keys, Vitest + `@testing-library/react` for tests.

---

## Global Constraints

- Workspace: `d:/project/aegis`. Desktop app lives in `apps/desktop/aegis-desktop/`. UI package lives in `lib/packages/ui/`.
- Hook style: every API-consuming hook is a `useMutation` / `useQuery` that wraps the `api` object in `features/<feature>/data/`. No `fetch` calls in components.
- API: `api.updateUserCredential({ userCode, password })` already exists in `apps/desktop/aegis-desktop/src/shared/api/index.ts`. The hook must NOT call `api.updateUser` (which is for non-credential user fields).
- Logout: `useLogout()` from `features/auth/data/logout` already clears the query cache and closes project workspace windows. The success path must `await logout.mutateAsync()` then `await navigate({ to: "/login" })`.
- i18n: every user-visible string goes through `useI18n`'s `t()`. Keys are flat strings of dotted segments. Both `en.ts` and `zhCN.ts` must stay in sync.
- Errors: `errorMessage(e)` from `shared/api/error` is the standard one-line rendering for any `ApiError` rejection.
- Tests: follow `apps/desktop/aegis-desktop/src/test/features/auth/user-footer.test.tsx` — `vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))` at the top, `mockCommands({...})` to dispatch by command name, `renderInRouter` to wrap with the app router.

---

## File layout

```
apps/desktop/aegis-desktop/src/
├── features/settings/
│   ├── data/
│   │   └── update-password.ts        # new — useUpdatePassword hook
│   ├── index.ts                       # edit — re-export useUpdatePassword
│   └── pages/
│       └── SettingsPage.tsx           # edit — button + two dialogs
└── test/features/settings/
    └── settings-page.test.tsx         # new — dialog flow tests

lib/packages/ui/src/i18n/locales/
├── en.ts                              # edit — settings.password.* keys
└── zhCN.ts                            # edit — same keys, Chinese values
```

---

## Task 1: Add i18n keys for settings.password.*

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

- [ ] **Step 1: Add keys to `en.ts`**

In `lib/packages/ui/src/i18n/locales/en.ts`, add the following block immediately after the existing `settings.language.label` line (so the new keys sit with the other `settings.*` keys):

```ts
  'settings.password.button': 'Update password',
  'settings.password.dialog.title': 'Update password',
  'settings.password.dialog.field': 'New password',
  'settings.password.dialog.next': 'Next',
  'settings.password.confirm.title': 'Confirm password update',
  'settings.password.confirm.message': 'Updating your password will log you out. Continue?',
  'settings.password.confirm.confirm': 'Update',
  'settings.password.confirm.cancel': 'Cancel',
  'settings.password.error.updateFailed': 'Failed to update password: {message}',
```

- [ ] **Step 2: Add keys to `zhCN.ts`**

In `lib/packages/ui/src/i18n/locales/zhCN.ts`, add the corresponding block at the same position (immediately after the existing `settings.language.label`):

```ts
  'settings.password.button': '修改密码',
  'settings.password.dialog.title': '修改密码',
  'settings.password.dialog.field': '新密码',
  'settings.password.dialog.next': '下一步',
  'settings.password.confirm.title': '确认修改密码',
  'settings.password.confirm.message': '修改密码后您将退出登录，是否继续？',
  'settings.password.confirm.confirm': '确认修改',
  'settings.password.confirm.cancel': '取消',
  'settings.password.error.updateFailed': '修改密码失败：{message}',
```

- [ ] **Step 3: Run typecheck to confirm key alignment**

Run from `d:/project/aegis`:

```bash
pnpm -C lib/packages/ui typecheck
pnpm -C apps/desktop/aegis-desktop typecheck
```

Expected: both pass. (The `as const` declaration in each locale file makes TypeScript catch any missing/extra key when the locale is wired in.)

If a typecheck error says a key is missing in one locale, copy the matching entry from the other locale's table.

- [ ] **Step 4: Commit**

```bash
cd d:/project/aegis
git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(aegis-ui/i18n): add settings.password.* keys"
```

---

## Task 2: Add `useUpdatePassword` hook

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/settings/data/update-password.ts`
- Modify: `apps/desktop/aegis-desktop/src/features/settings/index.ts`

**Interfaces:**
- Consumes: `api.updateUserCredential` from `../../../shared/api`.
- Produces: `useUpdatePassword(): UseMutationResult<UserCredentialView, ApiError, { userCode: string; password: string }>` exported from `features/settings/index.ts`.

- [ ] **Step 1: Create the hook file**

Create `apps/desktop/aegis-desktop/src/features/settings/data/update-password.ts`:

```ts
import { useMutation } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type UpdateUserCredentialInput,
  type UserCredentialView,
} from "../../../shared/api";

/**
 * Update the current user's password. Wraps
 * `api.updateUserCredential` (the `update_user_credential` Tauri
 * command). No cache to invalidate — the SettingsPage calls
 * `useLogout()` immediately after a successful mutation, which clears
 * the entire cache and closes the auth session.
 */
export function useUpdatePassword() {
  return useMutation<UserCredentialView, ApiError, UpdateUserCredentialInput>({
    mutationFn: (input) => api.updateUserCredential(input),
  });
}
```

- [ ] **Step 2: Re-export from the feature index**

Edit `apps/desktop/aegis-desktop/src/features/settings/index.ts`. The existing file looks like:

```ts
// Public API of the settings feature.

export {
  useHydrateSettingsFromStore,
  useListenForSettingsChanges,
  persistSettings,
} from "./data/persist";
```

Add a new export block after the existing one:

```ts
// Public API of the settings feature.

export {
  useHydrateSettingsFromStore,
  useListenForSettingsChanges,
  persistSettings,
} from "./data/persist";

export { useUpdatePassword } from "./data/update-password";
```

- [ ] **Step 3: Run typecheck**

Run from `d:/project/aegis`:

```bash
pnpm -C apps/desktop/aegis-desktop typecheck
```

Expected: PASS. The hook is a one-line wrapper around `api.updateUserCredential`, so the only failure mode is a typo in the import path.

- [ ] **Step 4: Commit**

```bash
cd d:/project/aegis
git add apps/desktop/aegis-desktop/src/features/settings/data/update-password.ts apps/desktop/aegis-desktop/src/features/settings/index.ts
git commit -m "feat(aegis-desktop): add useUpdatePassword hook"
```

---

## Task 3: Add the dialog flow to SettingsPage (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/test/features/settings/settings-page.test.tsx`
- Modify: `apps/desktop/aegis-desktop/src/features/settings/pages/SettingsPage.tsx`

**Interfaces:**
- Consumes: `useUpdatePassword()` from `../../data/update-password`; `useCurrentUser()` from `../../auth/data/current-user`; `useLogout()` from `../../auth/data/logout`; `useNavigate()` from `@tanstack/react-router`.
- Produces: a `SettingsPage` that renders (1) the existing theme + language controls, (2) a new `Update password` button, (3) a password-entry dialog, (4) a confirm dialog.

### Step A — Write the failing tests

- [ ] **Step 1: Create the test file with all eight cases**

Create `apps/desktop/aegis-desktop/src/test/features/settings/settings-page.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { SettingsPage } from "../../../features/settings/pages/SettingsPage";
import { renderInRouter } from "../../../test/helpers/file-route-utils";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import { TestQueryProvider } from "../../../test/helpers/test-query-provider";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

afterEach(() => {
  cleanup();
});

function renderSettings() {
  return renderInRouter(
    <AegisThemeProvider>
      <TestQueryProvider>
        <AegisI18nProvider>
          <SettingsPage />
        </AegisI18nProvider>
      </TestQueryProvider>
    </AegisThemeProvider>,
  );
}

function userViewFixture() {
  return {
    id: 1,
    code: "alice",
    name: "Alice",
    role: "general" as const,
    active: true,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  };
}

describe("SettingsPage — update password", () => {
  it("renders the Update password button", async () => {
    mockCommands({
      current_user: () => userViewFixture(),
    });
    await renderSettings();
    expect(
      await screen.findByRole("button", { name: /update password/i }),
    ).toBeInTheDocument();
  });

  it("opens the password dialog when the button is clicked", async () => {
    mockCommands({
      current_user: () => userViewFixture(),
    });
    await renderSettings();
    await userEvent.click(
      await screen.findByRole("button", { name: /update password/i }),
    );
    expect(screen.getByRole("dialog", { name: /update password/i })).toBeInTheDocument();
    expect(screen.getByLabelText(/new password/i)).toBeInTheDocument();
  });

  it("disables Next when the password field is empty", async () => {
    mockCommands({
      current_user: () => userViewFixture(),
    });
    await renderSettings();
    await userEvent.click(
      await screen.findByRole("button", { name: /update password/i }),
    );
    const next = screen.getByRole("button", { name: /^next$/i });
    expect(next).toBeDisabled();
    await userEvent.type(screen.getByLabelText(/new password/i), "hunter2");
    expect(next).toBeEnabled();
  });

  it("advances from the password dialog to the confirm dialog", async () => {
    mockCommands({
      current_user: () => userViewFixture(),
    });
    await renderSettings();
    await userEvent.click(
      await screen.findByRole("button", { name: /update password/i }),
    );
    await userEvent.type(screen.getByLabelText(/new password/i), "hunter2");
    await userEvent.click(screen.getByRole("button", { name: /^next$/i }));
    // Password dialog gone, confirm dialog present.
    await waitFor(() =>
      expect(
        screen.queryByRole("dialog", { name: /update password/i }),
      ).not.toBeInTheDocument(),
    );
    expect(
      screen.getByRole("dialog", { name: /confirm password update/i }),
    ).toBeInTheDocument();
  });

  it("cancels the password dialog and clears the field on re-open", async () => {
    mockCommands({
      current_user: () => userViewFixture(),
    });
    await renderSettings();
    await userEvent.click(
      await screen.findByRole("button", { name: /update password/i }),
    );
    await userEvent.type(screen.getByLabelText(/new password/i), "hunter2");
    await userEvent.click(screen.getByRole("button", { name: /^cancel$/i }));
    await waitFor(() =>
      expect(
        screen.queryByRole("dialog", { name: /update password/i }),
      ).not.toBeInTheDocument(),
    );
    // Re-open and confirm the field is empty.
    await userEvent.click(
      await screen.findByRole("button", { name: /update password/i }),
    );
    expect(screen.getByLabelText(/new password/i)).toHaveValue("");
  });

  it("cancels the confirm dialog without calling update_user_credential", async () => {
    const updateCred = vi.fn().mockResolvedValue({
      userCode: "alice",
      passwordHash: "h",
      tokenVersion: 2,
    });
    const logout = vi.fn().mockResolvedValue(undefined);
    mockCommands({
      current_user: () => userViewFixture(),
      update_user_credential: updateCred,
      logout,
    });
    await renderSettings();
    await userEvent.click(
      await screen.findByRole("button", { name: /update password/i }),
    );
    await userEvent.type(screen.getByLabelText(/new password/i), "hunter2");
    await userEvent.click(screen.getByRole("button", { name: /^next$/i }));
    await userEvent.click(screen.getByRole("button", { name: /^cancel$/i }));
    expect(updateCred).not.toHaveBeenCalled();
    expect(logout).not.toHaveBeenCalled();
  });

  it("calls update_user_credential, then logout, then navigates to /login", async () => {
    const updateCred = vi.fn().mockResolvedValue({
      userCode: "alice",
      passwordHash: "h",
      tokenVersion: 2,
    });
    const logout = vi.fn().mockResolvedValue(undefined);
    const calls: string[] = [];
    updateCred.mockImplementation(() => {
      calls.push("update");
      return Promise.resolve({
        userCode: "alice",
        passwordHash: "h",
        tokenVersion: 2,
      });
    });
    logout.mockImplementation(() => {
      calls.push("logout");
      return Promise.resolve(undefined);
    });
    mockCommands({
      current_user: () => userViewFixture(),
      update_user_credential: updateCred,
      logout,
    });
    const { router } = await renderSettings();
    await userEvent.click(
      await screen.findByRole("button", { name: /update password/i }),
    );
    await userEvent.type(screen.getByLabelText(/new password/i), "hunter2");
    await userEvent.click(screen.getByRole("button", { name: /^next$/i }));
    await userEvent.click(screen.getByRole("button", { name: /^update$/i }));
    await waitFor(() => expect(updateCred).toHaveBeenCalledWith({
      userCode: "alice",
      password: "hunter2",
    }));
    await waitFor(() => expect(logout).toHaveBeenCalled());
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/login"),
    );
    expect(calls).toEqual(["update", "logout"]);
  });

  it("keeps the confirm dialog open and shows an error when update fails", async () => {
    const updateCred = vi.fn().mockRejectedValue({
      kind: "http",
      status: 400,
      code: "weak_password",
      message: "too weak",
    });
    const logout = vi.fn();
    mockCommands({
      current_user: () => userViewFixture(),
      update_user_credential: updateCred,
      logout,
    });
    await renderSettings();
    await userEvent.click(
      await screen.findByRole("button", { name: /update password/i }),
    );
    await userEvent.type(screen.getByLabelText(/new password/i), "x");
    await userEvent.click(screen.getByRole("button", { name: /^next$/i }));
    await userEvent.click(screen.getByRole("button", { name: /^update$/i }));
    await waitFor(() =>
      expect(
        screen.getByRole("dialog", { name: /confirm password update/i }),
      ).toBeInTheDocument(),
    );
    expect(await screen.findByRole("alert")).toHaveTextContent(
      /weak_password: too weak/,
    );
    expect(logout).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Run the tests to confirm they fail**

Run from `d:/project/aegis`:

```bash
pnpm -C apps/desktop/aegis-desktop test -- settings-page.test.tsx
```

Expected: ALL 8 tests fail with messages like `Unable to find a button with the name "Update password"` (the button does not exist yet). The component file is still the pre-feature version.

### Step B — Implement the dialog flow

- [ ] **Step 3: Replace `SettingsPage.tsx`**

Overwrite `apps/desktop/aegis-desktop/src/features/settings/pages/SettingsPage.tsx` with the following content. The structure is:
- existing theme + language controls (unchanged),
- a new `Update password` button below them,
- a password-entry dialog,
- a confirm dialog with the `<Alert>` error surface.

```tsx
import { useState, type ChangeEvent } from "react";
import { useNavigate } from "@tanstack/react-router";
import {
  Alert,
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  FormControl,
  FormControlLabel,
  InputLabel,
  MenuItem,
  Select,
  Switch,
  TextField,
  Typography,
  type SelectChangeEvent,
} from "@aegis/ui/mui";
import { useI18n, type Locale } from "@aegis/ui/i18n";
import { useThemeMode } from "@aegis/ui/theme";

import { useCurrentUser } from "../../auth/data/current-user";
import { useLogout } from "../../auth/data/logout";
import { useUpdatePassword } from "../data/update-password";
import { errorMessage } from "../../../shared/api/error";

export function SettingsPage() {
  const { mode, setMode } = useThemeMode();
  const { locale, setLocale, t } = useI18n();
  const navigate = useNavigate();
  const currentUser = useCurrentUser();
  const updatePassword = useUpdatePassword();
  const logout = useLogout();

  const [passwordDialogOpen, setPasswordDialogOpen] = useState(false);
  const [confirmDialogOpen, setConfirmDialogOpen] = useState(false);
  const [password, setPassword] = useState("");

  const userCode = currentUser.data?.code;

  const handleThemeChange = (event: ChangeEvent<HTMLInputElement>) => {
    setMode(event.target.checked ? "dark" : "light");
  };
  const handleLanguageChange = (event: SelectChangeEvent<Locale>) => {
    setLocale(event.target.value as Locale);
  };

  // Open the password dialog with a guaranteed-fresh field. Called
  // from the page button, not from any post-update path.
  function openPasswordDialog() {
    setPassword("");
    setPasswordDialogOpen(true);
  }

  // Close the password dialog for any reason — typed input must NOT
  // survive, so the field is always reset.
  function closePasswordDialog() {
    setPasswordDialogOpen(false);
    setPassword("");
  }

  // Move from the password dialog to the confirm dialog. The password
  // stays in state so the confirm step can submit it.
  function advanceToConfirm() {
    setPasswordDialogOpen(false);
    setConfirmDialogOpen(true);
  }

  // Confirm-dialog cancel: discard the password and close.
  function cancelConfirm() {
    setConfirmDialogOpen(false);
    setPassword("");
  }

  // Confirm-dialog confirm: run the credential update, then logout
  // and navigate. The confirm dialog stays open if the update fails
  // so the user can read the error.
  async function onConfirmUpdate() {
    if (userCode === undefined) return;
    try {
      await updatePassword.mutateAsync({ userCode, password });
      setConfirmDialogOpen(false);
      setPassword("");
      await logout.mutateAsync();
      await navigate({ to: "/login" });
    } catch (e) {
      // Leave the dialog open and let the Alert render the error.
      // updatePassword.error is read by the Alert below.
      void e;
    }
  }

  const themeLabel = t("settings.theme.label", {
    mode: t(mode === "dark" ? "settings.theme.dark" : "settings.theme.light"),
  });

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <Typography variant="h4" gutterBottom>
        {t("settings.heading")}
      </Typography>
      <FormControlLabel
        control={
          <Switch checked={mode === "dark"} onChange={handleThemeChange} />
        }
        label={themeLabel}
      />
      <FormControl size="small" sx={{ minWidth: 160 }}>
        <InputLabel id="language-label">
          {t("settings.language.label")}
        </InputLabel>
        <Select<Locale>
          labelId="language-label"
          value={locale}
          label={t("settings.language.label")}
          onChange={handleLanguageChange}
        >
          <MenuItem value="en">{t("language.english")}</MenuItem>
          <MenuItem value="zh-CN">{t("language.simplifiedChinese")}</MenuItem>
        </Select>
      </FormControl>

      <Box>
        <Button
          variant="outlined"
          color="warning"
          onClick={openPasswordDialog}
        >
          {t("settings.password.button")}
        </Button>
      </Box>

      <Dialog
        open={passwordDialogOpen}
        onClose={closePasswordDialog}
        aria-label={t("settings.password.dialog.title")}
      >
        <DialogTitle>{t("settings.password.dialog.title")}</DialogTitle>
        <DialogContent>
          <TextField
            autoFocus
            margin="dense"
            label={t("settings.password.dialog.field")}
            type="password"
            fullWidth
            value={password}
            onChange={(event) => setPassword(event.target.value)}
          />
        </DialogContent>
        <DialogActions>
          <Button onClick={closePasswordDialog}>
            {t("settings.password.confirm.cancel")}
          </Button>
          <Button
            onClick={advanceToConfirm}
            variant="contained"
            disabled={password === ""}
          >
            {t("settings.password.dialog.next")}
          </Button>
        </DialogActions>
      </Dialog>

      <Dialog
        open={confirmDialogOpen}
        onClose={cancelConfirm}
        aria-label={t("settings.password.confirm.title")}
      >
        <DialogTitle>{t("settings.password.confirm.title")}</DialogTitle>
        <DialogContent>
          <DialogContentText>
            {t("settings.password.confirm.message")}
          </DialogContentText>
          {updatePassword.isError && (
            <Alert severity="error" sx={{ mt: 2 }}>
              {t("settings.password.error.updateFailed", {
                message: errorMessage(updatePassword.error),
              })}
            </Alert>
          )}
        </DialogContent>
        <DialogActions>
          <Button onClick={cancelConfirm} disabled={updatePassword.isPending}>
            {t("settings.password.confirm.cancel")}
          </Button>
          <Button
            onClick={() => void onConfirmUpdate()}
            variant="contained"
            disabled={updatePassword.isPending}
          >
            {t("settings.password.confirm.confirm")}
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}
```

- [ ] **Step 4: Run the new tests to confirm they pass**

Run from `d:/project/aegis`:

```bash
pnpm -C apps/desktop/aegis-desktop test -- settings-page.test.tsx
```

Expected: all 8 tests pass.

If `screen.getByRole("dialog", { name: /update password/i })` does not match in Steps 1 / 2 / 5 of the test file, drop the `name:` argument and rely on `getByRole("dialog")` (the title is the accessible name; MUI's `DialogTitle` is rendered as a heading inside the dialog, so the dialog's accessible name is the title text — but the matcher varies by version). If the matcher fails, fall back to `screen.getByRole("dialog")` and assert on the title text with `screen.getByText(...)` instead.

If the test that asserts `cancels the password dialog and clears the field on re-open` is flaky because of MUI's exit animation, add `await waitFor(...)` before the re-open click.

- [ ] **Step 5: Run the existing settings tests to confirm no regressions**

```bash
pnpm -C apps/desktop/aegis-desktop test -- features/settings
```

Expected: all tests in `features/settings` pass (the existing `settings-persist.test.tsx` and `settings-route.test.tsx` are unaffected; the new `settings-page.test.tsx` is the only addition).

- [ ] **Step 6: Run typecheck across the workspace**

```bash
pnpm -C apps/desktop/aegis-desktop typecheck
pnpm -C lib/packages/ui typecheck
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
cd d:/project/aegis
git add apps/desktop/aegis-desktop/src/features/settings/pages/SettingsPage.tsx apps/desktop/aegis-desktop/src/test/features/settings/settings-page.test.tsx
git commit -m "feat(aegis-desktop): add update password flow on settings page"
```
