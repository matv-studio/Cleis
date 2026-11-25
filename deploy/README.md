# Cleis server — installation

One program, `cleis-server`, for Linux, Windows and macOS, or a container. The apps trust it by
the certificate pinned in each invitation code: any network works (VPN address, local network,
Internet), with no certificate authority and no domain name.

## 1. Create the organization

```
cleis-server init --data DATA --org "Example Studio" [--listen 0.0.0.0:8443]
```

It prints the first administrator's invitation code (valid 7 days). In Cleis: first launch →
*Join an organization*, or Settings › Organization, with the server's address and the code.

`DATA` holds `server.db` and `cleis-server.toml` (addresses, automatic backups, e-mail, SMS,
Google / Microsoft sign-in). Without `--data`: `CLEIS_DATA`, else `/var/lib/cleis-server`
(Linux), `/Library/Application Support/Cleis Server` (macOS), `%ProgramData%\Cleis Server`
(Windows).

## 2. Start it with the system

| System | Command (as administrator) |
| --- | --- |
| Linux (systemd) | `sudo cleis-server install --data DATA` |
| macOS (launchd) | `sudo cleis-server install --data DATA` |
| Windows (service) | `cleis-server install --data DATA` in an administrator terminal |
| Docker | `docker compose -f deploy/compose.yaml up -d` |

`--print` shows the service file without installing it. `cleis-server uninstall` removes the
service and keeps the data. Windows: the service runs a copy in `%ProgramFiles%\Cleis Server`,
and the data folder keeps only SYSTEM, the Administrators and the account that made it. Linux:
when the data folder belongs to root, the service runs as a `cleis-server` system account that
`install` creates. `cleis-server status --data DATA` fails when nothing answers on this computer
(Docker's health check).

To try without installing: `cleis-server run --data DATA`.

## 3. Network

Open the TCP port of `listen` (8443 by default) to the apps: on the VPN only, on the local
network, or to the Internet. A firewall rule or a port forward is enough; no reverse proxy (the
apps check the server's own certificate).

## 4. Backups

Automatic copies of the server file every 24 h in `DATA/backups` (30 kept), set in
`cleis-server.toml`. By hand: `cleis-server backup --data DATA [--to DIR]`. To put one back:
stop the server, `cleis-server restore --data DATA FILE`, start it again. Admins can also back up
and restore from the app (Settings › Sync & backups › the organization).

Items are sealed by the apps: the server and its copies never hold a key that opens them. The
server file itself holds the server's key and the members' authenticator keys: keep `backups`
where only the server's account reads it. From the app, a restore is refused when the copy's
activity log does not verify, and members removed since the copy stay removed.

## 5. Second sign-in step

The authenticator app always works. The other methods appear in Security policy once the server
can carry them out; add the section to `cleis-server.toml`, then restart the server.

- **E-mail**: `[smtp]` with `host`, `port` (587), `security` (`starttls`, `tls` or `none`),
  `username`, `password`, `from`. The code goes to the member's address (Profile).
- **SMS**: `[sms]` with the gateway's `url`, `method`, `headers` and `body`; `{to}`, `{code}` and
  `{text}` are replaced (JSON or form bodies are escaped). Any gateway reached by HTTPS works.
- **Google**: create an OAuth client of type *Desktop app* in Google Cloud, then
  `[[oidc]] name = "google"`, `issuer = "https://accounts.google.com"`, `client_id`,
  `client_secret` (a desktop client's secret is not secret: it is handed to the apps).
- **Microsoft**: register an application in Microsoft Entra with the redirect address
  `http://localhost` (public client), then `[[oidc]] name = "microsoft"`,
  `issuer = "https://login.microsoftonline.com/<tenant id>/v2.0"`, `client_id`. With
  `{tenantid}` in place of the id, any work account is accepted: then do not rely on `domains`,
  since Microsoft does not guarantee that an account's address is verified.
- `domains = ["example-studio.com"]` in an `[[oidc]]` section accepts only those addresses.
- `max_token_age` (seconds, 600 by default): an ID token older than this is refused.

Codes last 10 minutes, five wrong tries cancel one, a new one goes out at most once a minute and
ten a day per member. An ID token is checked against the provider's published keys, must be
recent and issued to Cleis, and is used only once.

With `[smtp]`, invitations also go by e-mail, with the server's address as the admin's app knows
it; *Resend* sends a new code the same way.
