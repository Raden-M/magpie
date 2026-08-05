# Packaging

Ways to keep Magpie running that aren't `make dev`.

## systemd (Linux, per-user)

Runs Magpie as your own user, on loopback, with no token - the local-first setup.
Every step is safe to repeat.

```sh
make build
install -Dm755 backend/target/release/magpie ~/.local/bin/magpie
install -Dm644 packaging/systemd/magpie.service ~/.config/systemd/user/magpie.service
systemctl --user daemon-reload
systemctl --user enable --now magpie
loginctl enable-linger "$USER"
```

`enable-linger` is the step people miss: without it the unit stops when your login
session ends, so Magpie is gone after a reboot until you log in again.

Check it and read the logs:

```sh
systemctl --user status magpie
journalctl --user -u magpie -f
```

The unit sets no `MAGPIE_DATA_DIR`, so the collection lives wherever the binary
defaults to - `~/.local/share/magpie` on Linux. To point it elsewhere, or to bind
somewhere other than loopback, use an override rather than editing the unit:

```sh
systemctl --user edit magpie
```

```ini
[Service]
Environment=MAGPIE_BIND=0.0.0.0:8787
Environment=MAGPIE_TOKEN=...
```

### Upgrading

Replace the binary and restart. Migrations run forward automatically at startup:

```sh
make build && install -Dm755 backend/target/release/magpie ~/.local/bin/magpie
systemctl --user restart magpie
```

Migrations are forward-only, so a *downgrade* across a schema change won't start -
it refuses with a message naming the data directory rather than touching your data.

## Docker

See [`docker-compose.yml`](../docker-compose.yml) and `make docker-up`.

## macOS and Windows

No packaging yet. The binary runs fine on both - `magpie` with no arguments serves
<http://127.0.0.1:8787> and stores its collection in the platform data directory
(`~/Library/Application Support/magpie`, `%APPDATA%\magpie`). Wiring it into
`launchd` or Task Scheduler is left to you for now.
