# Releasing

Everything is automated behind a tag. The parts that aren't - and can't be - are the
credentials, which have to be created by a person once.

## One-time setup

### AMO signing credentials

Release Firefox refuses unsigned add-ons, so an unsigned build disappears on restart.
Signing on the **unlisted** channel means Mozilla signs the file and hands it back:
no public listing, no review queue, and a tag can go out in minutes.

1. Sign in at [addons.mozilla.org](https://addons.mozilla.org) with the account that
   should permanently own the add-on id `magpie@raden.me`.
2. Go to **Tools → Manage API Keys** and generate a credential pair.
3. Add both to the repository under **Settings → Secrets and variables → Actions**:

   | Secret | From |
   | --- | --- |
   | `AMO_JWT_ISSUER` | the JWT issuer |
   | `AMO_JWT_SECRET` | the JWT secret |

Until these exist the release still runs - it just skips signing and logs a warning,
so the binaries, image and Chromium zip publish as usual.

Two constraints worth knowing before the first signed build: **the add-on id is
permanently tied to the signing account**, and **a version can never be re-signed**.
A botched `0.2.0` is retired by shipping `0.2.1`, not by fixing `0.2.0`.

### Packages

`GITHUB_TOKEN` covers both `ghcr.io` and the release itself, so nothing else is needed.
The first push creates the package private - make it public under **Packages → Package
settings** or `docker pull` will ask people to authenticate.

## Cutting a release

The version is declared in three files and nothing keeps them in step, so bump all
three together:

- `backend/Cargo.toml`
- `extension/package.json`
- `extension/manifest.json`

```sh
node scripts/check-versions.mjs      # they agree
make test && make lint-firefox       # green
cd backend && cargo update --workspace && cargo test   # Cargo.lock is committed and the release builds --locked
```

Then tag and push:

```sh
git commit -am "chore: release v0.2.0"
git tag v0.2.0
git push origin main v0.2.0
```

The release workflow re-checks the versions against the tag before anything is
published, so a mistyped tag fails in the first job rather than half-way through.

## What comes out

| Asset | Where |
| --- | --- |
| `magpie-<version>-{linux,macos}-{x86_64,aarch64}.tar.gz` | Release |
| `magpie-<version>.xpi` (signed) | Release |
| `updates.json` | Release - this is what `update_url` points at |
| `magpie-chromium-<version>.zip` | Release |
| `checksums.txt` | Release |
| `ghcr.io/<owner>/magpie:<version>` and `:latest` | GHCR, multi-arch |

Binaries are built on native runners, including arm64 Linux. That isn't an
optimisation: `rusqlite`'s `bundled` feature compiles SQLite from C, so plain
`cargo build --target` won't cross-compile it. Intel macOS is the one exception,
cross-compiled from the arm64 runner because the Xcode SDK handles it cleanly.

## After a release

Check that `updates.json` is reachable, since it's what every installed copy follows:

```sh
curl -sL https://github.com/Raden-M/magpie/releases/latest/download/updates.json
```

Installed Firefox copies pick the new version up on their own update cycle; **Check for
Updates** in `about:addons` forces it.

## Rolling back

Don't downgrade across a schema change. Migrations are forward-only, and an older
binary meeting a newer collection refuses to start rather than touching it. Roll
forward with a fix release instead.
