---
title: Personal settings
description: Set up your own Aphel account, from language, menu and keyboard shortcuts to appearance, notifications, API tokens and account security.
sidebar:
  order: 15
---

Personal settings affect only you. To open them, use the user menu in the top bar and choose **My account**. The **My account** area ("Your preferences, notifications and sign-in security") has these items:

| Group | Item | What it is for |
| --- | --- | --- |
| Profile and preferences | **Appearance** | Choose the interface |
| Profile and preferences | **Keyboard shortcuts** | Turn shortcuts on or off and change their keys |
| Profile and preferences | **Navigation and language** | Language, home screen and menu order |
| Profile and preferences | **Notifications** | Which alerts you get and where (needs the `notifications` module) |
| Security | **Account security** | Password, two-factor authentication, sessions |
| Security | **Security events** | Sign-ins and security changes on your account |
| Security | **API tokens** | Keys for integrations that act as you |

Everyone who is signed in can open these items; none needs a special permission, except the extra parts noted below.

## Keyboard shortcuts

Open **My account**, then **Keyboard shortcuts**.

- **Use keyboard shortcuts** turns the shortcuts on or off. `Ctrl+K` (`Cmd+K` on Mac) keeps opening the command palette, and shortcuts inside text boxes (such as `Ctrl+Enter` to send) keep working, even when they are off.
- The list shows every action you can use, grouped by where it applies: everywhere, on a specific screen (such as the register) or inside text boxes. Search by name or by key.
- **Change** opens a box where you press the new combination. For a sequence such as `G` then `D`, press the two keys one after the other. Turn on **Keep the current shortcut too** to add an alternative instead of replacing it.
- If the keys are already used by another action in the same place, the box says so, and **Replace and save** moves them to this action. Keys usually taken by the browser, such as `Ctrl+W`, show a warning.
- **Turn off** removes the shortcut of an action, and **Reset** brings back the company default. The badge on each row says where the keys come from: **Default**, **Company** or **You**.

Administrators set the defaults for everyone in **Settings > Keyboard shortcuts** (needs `settings.manage`), for example to use the same register keys on every terminal. Each person can still change their own.

### Default shortcuts

Single-key shortcuts do not fire while you type in a field or when a dialog is open. Press `?` anywhere to see the list with your current keys.

| Keys | Action |
| --- | --- |
| `Ctrl+K` / `Cmd+K` | Open or close the command palette |
| `C` | Create in the context of the page, or open the create menu |
| `?` | Show the shortcut list |
| `G` then `H` | Go to Home |
| `G` then `S` | Go to Settings |
| `G` then `C` | Go to Contacts |
| `G` then `E` | Go to Companies |
| `G` then `D` | Go to Deals |
| `G` then `T` | Go to Tasks |
| `G` then `I` | Go to Inbox |
| `G` then `A` | Go to Calendar |
| `G` then `R` | Go to Reports |
| `G` then `P` | Go to Products |
| `Ctrl+Enter` / `Cmd+Enter` | Send a reply, save a note or run the assistant, inside their text boxes |

For "go to" shortcuts, press `G`, then the second key within about a second. A shortcut only works if the page is in your menu, which depends on enabled modules and your permissions.

## Navigation and language

Open **My account**, then **Navigation and language**. The settings apply only to you, in every organization.

**Language.** Choose **English**, **Português (Brasil)** or **Follow the organization**. It changes the interface and the emails you receive; date and number formats follow the language.

**Home screen.** Choose where you land after signing in. The default is **Home (dashboard)**; you can pick any other page available to you.

**Menu items.** Untick an item to hide it from the menu, and use the arrows to move items up or down. Click **Save navigation** to apply. Notes:

- **Settings** is always visible and cannot be moved.
- New items that appear later (for example after a module is enabled) are added at the end.
- Items you cannot access stay hidden.
- **Restore default** clears your order and hidden items in the form (then save). **Reset only my home and navigation** clears them immediately and keeps your language.

## Appearance and interface plugins

Open **My account**, then **Appearance**. Aphel lets you pick which interface you use. The choices are:

- **Follow the organization default**: whatever the administrator chose (shown as "Currently: ...").
- **Aphel original**: the standard interface.
- Any interface plugin installed and enabled in the workspace. Each shows a few preview colors and a description.

A plugin can change colors and fonts, and some replace whole screens. Those show the note "Includes screens from a plugin, which run code with your session", so choose only plugins your administrator trusts. The page reloads after you choose.

Users with `settings.manage` also see **Organization default** on this page. It sets the interface used by everyone who has not chosen one and by the sign-in page. It does not install or enable plugins; that is done in the plugin and module settings (see [Interface plugins](/docs/plugins/ui-plugins/)).

## Notifications

Open **My account**, then **Notifications** to choose, for each kind of notice, whether you get it in Aphel, by email or as a push notification. The full table of notices and their defaults is in [Search and collaboration](/docs/using/search-and-collaboration/#notifications).

### Push notifications

At the top of the **Notifications** page is **Push notifications on this device**. Click **Turn on for this device** and allow notifications in the browser prompt. Then any notice with the **Push** box ticked reaches you even when Aphel is closed.

- It applies only to this browser or device, and is turned off when you sign out.
- If the browser has blocked notifications for the site, allow them in the browser settings first.
- Not every browser supports push. On iPhone and iPad it only works with Aphel installed: tap Share, then **Add to Home Screen**, open Aphel from the icon and turn it on there. An **Install Aphel** button is offered where installing is possible.
- Push does not work on a development server, only on a built version of Aphel.

## Account security

Open **My account**, then **Account security**.

### Password

Enter your current password and the new one twice, then click **Change password**. Passwords must have 8 to 128 characters. Changing your password signs out all your other sessions and sends you an email notice.

### Two-factor authentication

Two-factor authentication asks for a code from your phone, in addition to your password, when you sign in. Any authenticator app works (for example Google Authenticator, Microsoft Authenticator, 1Password or Authy).

**Turn it on.**

1. Click **Enable**.
2. Scan the QR code with your app, or type the setup key manually.
3. Enter the 6-digit code and click **Enable**.
4. Save the **recovery codes** shown (copy them or **Download .txt**). They are shown only once, and each works a single time if you lose your phone.

After this, sign-in asks for the app code; choose **Use a recovery code** if you cannot use the app. You can see how many recovery codes are left, **Generate new recovery codes** (the old ones stop working) or **Disable** it. Disabling and regenerating ask you to confirm with your password and a code.

**Organization requirement.** Users with `settings.manage` see **Organization requirement** on the same page and can require two-factor authentication from **Nobody**, **Administrators** or **All members**. People covered by the rule who have not enabled it are taken to the setup screen before they can keep using Aphel. You must have two-factor enabled on your own account before you can change this setting.

### Active sessions

The **Active sessions** list shows the devices signed in to your account, with browser, system, IP address and last activity. Use **Sign out** on a session you do not recognize, or **Sign out all others** to keep only the current one.

## Security events

Open **My account**, then **Security events** for a log of sign-ins (successful and failed), lockouts, two-factor changes, password changes, signed-out sessions, API token creation and revocation, and permission changes. Filter by type and date range.

By default it shows **My events**. Users with `users.manage` can switch to **Organization** to see everyone's events, with a column for the user.

## API tokens

API tokens let another system use the Aphel API as you. Open **My account**, then **API tokens**, or use the **Manage tokens** link in **Account security**. For the technical side, see [Authentication](/docs/reference/authentication/).

**Create a token.**

1. Click **New token**.
2. Enter a name that identifies the integration (up to 100 characters).
3. Choose the **scopes**: for each resource (Contacts, Companies, Deals, Activities, Notes, Emails, Sequences, Booking, Settings, Users, Audit, System) you can grant **Read** or **Write**. Pick only what the integration needs. You can only choose scopes your own permissions cover.
4. Choose an expiration: in 30 days, in 90 days, in 1 year or never.
5. Click **Create token** and copy the token right away. It is shown only once.

A token can never do more than you can: what it may do is the intersection of its scopes and your current permissions at the moment of each request.

**Manage tokens.** The list shows each token's status (**Active**, **Revoked**, **Expired**), creation date, expiry, and last use with the IP address. **Revoke** stops it immediately and cannot be undone. Each person can have up to 50 active tokens. Users with `users.manage` can tick **Show tokens from all members** to see and revoke everyone's tokens.

Users with `settings.manage` also see **API token limits** on this page: the number of requests accepted per token in a sliding window (600 requests per 60 seconds by default). It applies to every token in the installation.
