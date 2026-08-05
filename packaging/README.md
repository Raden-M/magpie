# Packaging

Ways to keep Magpie running that aren't `make dev`.

## systemd (Linux, per-user)

Runs Magpie as your own user, on loopback, with no token - the local-first setup. Every step is safe
to repeat.

```sh
make build
install -Dm755 backend/target/release/magpie ~/.local/bin/magpie
install -Dm644 packaging/systemd/magpie.service ~/.config/systemd/user/magpie.service
systemctl --user daemon-reload
systemctl --user enable --now magpie
loginctl enable-linger "$USER"
```

`enable-linger` is the step people miss: without it the unit stops when your login session ends, so
Magpie is gone after a reboot until you log in again.

```sh
systemctl --user status magpie     # check it
journalctl --user -u magpie -f     # read the logs
```

The unit sets no environment, so the collection lands in `~/.local/share/magpie`. To put it
elsewhere, or to bind somewhere other than loopback, override rather than edit the unit -
`systemctl --user edit magpie`:

```ini
[Service]
Environment=MAGPIE_BIND=0.0.0.0:8787
Environment=MAGPIE_TOKEN=...
```

The full list of variables is in [`docs/usage.md`](../docs/usage.md#configuration).

To upgrade, replace the binary and restart - migrations run forward automatically at startup:

```sh
make build && install -Dm755 backend/target/release/magpie ~/.local/bin/magpie
systemctl --user restart magpie
```

## Docker

`make docker-up`, or [`docker-compose.yml`](../docker-compose.yml) by hand. See
[`docs/usage.md`](../docs/usage.md#docker).

## macOS and Windows

No packaging yet. The binary runs fine on both - `magpie` with no arguments serves
<http://127.0.0.1:8787> and stores the collection in the platform data directory. Wiring it into
`launchd` or Task Scheduler is left to you for now.
