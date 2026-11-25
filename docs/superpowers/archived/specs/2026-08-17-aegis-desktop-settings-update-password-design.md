# Add an Update Password button to the Settings page

Date: 2026-08-17
Status: Approved (brainstorming)

## Goal

Add a destructive "Update password" button to the existing Settings
page in the Aegis desktop app. Pressing it opens a dialog for entering
the new password, then a confirmation dialog that warns the user they
will be logged out. On confirm the app calls the existing credential
update endpoint, logs the user out, and navigates to `/login`.

The new control sits alongside the existing theme switch and language
selector on `SettingsPage`. No new server route, no new domain types,
no new auth flow — the credential update endpoint
`api.updateUserCredential` (`update_user_credential` Tauri command)
and the `useLogout` hook already exist per the
[2026-08-06 credential and logout spec](2026-08-06-apis-auth-credential-and-logout-design.md)
and the
[2026-08-14 sidebar user-footer spec](2026-08-14-aegis-desktop-sidebar-user-footer-design.md).

The only data-layer addition is one new `useUpdatePassword` hook plus
its re-export through the settings feature index.

## Approach

Keep all UI in `SettingsPage.tsx` for now (the page is still small);
add a single `useUpdatePassword` hook in
`features/settings/data/update-password.ts`; re-export it from the
feature index. Drive two MUI `Dialog`s via local `useState` flags.
Reuse the existing dialog / Alert / Button patterns from
`UserFooter.tsx` and `LoginPage.tsx` rather than introducing a new
shared component. The dialog flow is short and one-off — a shared
`ConfirmDialog` abstraction would be premature.

### Why single password field (not a "confirm password" pair)

The user explicitly asked for a single "enter the new password"
dialog. The two-step flow (password entry → confirm) already provides
a soft re-check, and the server-side validation is the source of truth
for actual password policy. A double-entry field would inflate the
dialog for no real gain.

### Why the confirm dialog is separate from the password dialog

The user explicitly asked for a confirm step. Splitting it out makes
the destructive consequence ("you will be logged out") impossible to
miss and matches the project's existing pattern in `UserFooter`.

### Why the confirm dialog stays open on error

If the credential update fails (e.g. weak password or network error),
auto-dismissing the dialog would force the user to re-enter the
password before they could see the error. Keeping it open with an
`Alert` lets the user read the failure and retry from the password
dialog (Cancel → re-open from the button).

### Why `useUpdatePassword` lives in `features/settings/data/`

The hook is owned by the only feature that uses it (the Settings
page). Co-locating it with `useThemeMode` / `useI18n` migration of
the page is the simplest mirror. Re-exporting from
`features/settings/index.ts` keeps the import paths consistent with
the rest of the feature.

## File layout

```
apps/desktop/aegis-desktop/src/
├── features/
│   └── settings/
│       ├── data/
│       │   └── update-password.ts        # new — useUpdatePassword hook
│       ├── index.ts                       # re-export useUpdatePassword
│       └── pages/
│           └── SettingsPage.tsx          # edit — add button + two dialogs
└── test/
    └── features/
        └── settings/
            └── settings-page.test.tsx     # new — dialog flow tests
```

`lib/packages/ui/src/i18n/locales/en.ts` and `zhCN.ts` get new
`settings.password.*` keys.

## Data flow

```
[Update password button]
        │ click
        ▼ opens passwordDialogOpen, password = ""
[Password dialog]
        │ Cancel  → close, password = ""
        │ Next    → close password dialog, open confirm dialog
        ▼
[Confirm dialog: "Updating your password will log you out."]
        │ Cancel  → close (password discarded)
        │ Confirm → useUpdatePassword.mutateAsync({ userCode, password })
        │           success → close, password = "",
        │                     useLogout.mutateAsync(),
        │                     navigate("/login")
        │           error   → stay open, show <Alert> with errorMessage(e)
        ▼
```

## State

`SettingsPage` local state:

- `passwordDialogOpen: boolean` — gates the password-entry dialog.
- `confirmDialogOpen: boolean` — gates the confirmation dialog.
- `password: string` — bound to the password `TextField`. Cleared on
  every close of the password dialog and on successful update.

Hooks consumed:

- `useCurrentUser()` (already imported by the page if needed; new
  here) → provides `user.code` for the mutation input.
- `useUpdatePassword()` (new) → `mutateAsync({ userCode, password })`.
- `useLogout()` (existing) → `mutateAsync()` post-success.
- `useNavigate()` (already used elsewhere; new here) → `/login`.

## Error handling

- The confirm dialog stays open on update failure and shows an
  `<Alert severity="error">` with `errorMessage(e)`. The user can
  Cancel → re-enter a password if needed.
- The password input is cleared any time the password dialog closes,
  so a stale password never leaks into a retry.
- Logout errors after a successful update are not surfaced. The
  credential has already changed; the user explicitly chose to log
  out. The `useLogout` hook already handles the in-memory cleanup.

## Validation

- "Next" is disabled while `password === ""`.
- "Confirm" is disabled while the mutation is pending.
- No length / complexity check on the client; server-side validation
  is the source of truth and will surface via the error alert.

## i18n keys

Add to both `en.ts` and `zhCN.ts`:

| key | en | zh-CN |
|---|---|---|
| `settings.password.button` | `Update password` | `修改密码` |
| `settings.password.dialog.title` | `Update password` | `修改密码` |
| `settings.password.dialog.field` | `New password` | `新密码` |
| `settings.password.dialog.next` | `Next` | `下一步` |
| `settings.password.confirm.title` | `Confirm password update` | `确认修改密码` |
| `settings.password.confirm.message` | `Updating your password will log you out. Continue?` | `修改密码后您将退出登录，是否继续？` |
| `settings.password.confirm.confirm` | `Update` | `确认修改` |
| `settings.password.confirm.cancel` | `Cancel` | `取消` |
| `settings.password.error.updateFailed` | `Failed to update password: {message}` | `修改密码失败：{message}` |

## Tests

New file `apps/desktop/aegis-desktop/src/test/features/settings/settings-page.test.tsx`
following the established `user-footer.test.tsx` style (Vitest +
`@testing-library/react` + `renderInRouter` + `mockCommands` +
`TestQueryProvider` + `AegisI18nProvider` + `AegisThemeProvider`).

Cases:

1. Renders the **Update password** button.
2. Clicking the button opens the password dialog.
3. **Next** is disabled when the password field is empty; enabled
   once the user types.
4. **Next** dismisses the password dialog and opens the confirm
   dialog; the password is preserved across the transition.
5. **Cancel** on the confirm dialog closes it without calling
   `update_user_credential` or `logout`.
6. **Cancel** on the password dialog clears the password field when
   the user re-opens the dialog.
7. **Confirm** calls `update_user_credential` with
   `{ userCode, password }`, then `logout`, then navigates to
   `/login`.
8. When `update_user_credential` fails, the confirm dialog stays
   open and an error `Alert` is rendered.

## Out of scope

- "Confirm password" double-entry field (per requirements).
- Client-side password complexity rules (server enforces).
- Updating any other user field.
- Any change to the credential update endpoint or the logout flow.
- Showing the password value in the confirm dialog.
