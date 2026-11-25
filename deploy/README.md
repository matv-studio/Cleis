# Cleis server — installation

One program, `cleis-server`, for Linux, Windows and macOS, or a container. The apps trust it by
the certificate pinned in each invitation code: any network works (VPN address, local network,
Internet), with no certificate authority and no domain name.

## 1. Create the organization

```
cleis-server init --data DATA --org "Example Studio" [--listen 0.0.0.0:8443]
```

It prints the first administrator's invitation code (valid 7 days). In Cleis: first launch →
*Add an organization server*, or Settings › Organization, with the server's address and the code.

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
service and keeps the data.

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

Items are sealed by the apps: the server and its copies never hold a key that opens them.
