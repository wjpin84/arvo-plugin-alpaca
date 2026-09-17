# arvo-plugin-alpaca

Alpaca as an [Arvo](https://github.com/wjpin84/arvo-desktop) source plugin: a
process that serves `arvo.source.v1.Source` over gRPC, so Arvo lists its four
Alpaca sources (IEX and SIP, each split-adjusted and total return, under venues
`AIEX`, `ASIP`, `AIEXTR`, `ASIPTR`) exactly as it did when they were compiled in.

This is the second source to live outside the app, after
[arvo-plugin-yahoo](https://github.com/wjpin84/arvo-plugin-yahoo), and the one
that shows the credential pattern. Everything Alpaca-specific stays in
`arvo-alpaca` inside `arvo-desktop`; this repository is the process boundary
and nothing else. `src/main.rs` is the whole plugin.

## The keys never live here

Arvo reads your Alpaca key pair from its own keychain and sends it with each
call, as that call's grant. The sources in this process are built from exactly
what the call brought and never look past it: a call that arrives with no keys
is a call with no session, whatever this machine's keychain or environment
might hold. Nothing is kept between calls (ADR-0022 point 4).

So there is nothing to configure here. Connect Alpaca in Arvo's Accounts view
as before; the plugin sees the keys only inside each request.

## Run it

```sh
cargo run --release
```

It listens on `127.0.0.1:50053`. Set `ARVO_PLUGIN_ADDR` to change that.

Then tell Arvo where it is. Arvo reads `plugins.toml` from its configuration
directory:

| OS | Path |
|---|---|
| Windows | `%APPDATA%\com.arvo.desktop\plugins.toml` |
| macOS | `~/Library/Application Support/com.arvo.desktop/plugins.toml` |
| Linux | `~/.config/com.arvo.desktop/plugins.toml` |

```toml
[[plugin]]
id = "alpaca"
address = "http://127.0.0.1:50053"
```

Or let Arvo start it: `command = "path/to/arvo-plugin-alpaca"` in place of
`address`, and Arvo launches it on a port of its own choosing, restarts it if
it dies, and stops it when Arvo quits. Either way it is the same plugin in the
same registry.

Arvo probes it at launch and every thirty seconds. The Extensions view shows it
under Providers as Reachable, and Fetch offers the four Alpaca sources. When the
plugin is running, it serves in place of the compiled-in Alpaca; when it is
not, the compiled-in one still answers.

## What the boundary carries

The proto mirrors `arvo_data::source::Source` method for method. A description
carries the id, the label, the venue, the credential kind, and the **basis**
(which feed, which adjustment), which is the declaration the boundary exists
to keep: IEX prices agree with the consolidated tape and IEX volume does not,
and no check on the bars can tell you which you have.

Errors cross as one gRPC status code per `SourceError` variant, and the host
rebuilds the variant on its side, so `needs_sign_in` survives the boundary.

## Installing from GitHub

`arvo-extension.json` declares this repository as a provider extension with
its recipe (ADR-0025):

```json
"build": "cargo build --release",
"run": "target/release/arvo-plugin-alpaca"
```

Paste `wjpin84/arvo-plugin-alpaca` into Arvo's Extensions view. Install clones
the repository at a pinned commit and runs nothing. The extension then shows
the recipe verbatim with a Build button; confirming runs the build in the
extension's own folder with its output in the Output panel, and copies the
binary into a cache Arvo owns. The Extensions view says whether it built.

Once built, Arvo starts it itself (ADR-0023): the binary is launched from the
cache on a port of Arvo's choosing, appears under Providers, is restarted a few
times if it dies and then reported, and is stopped when the extension is
disabled or removed and when Arvo quits. Nothing goes in `plugins.toml`.

## Building this repository

The host crates are git dependencies on `arvo-desktop`, pinned to a tag.
`arvo-desktop` is private, so building needs read access to it; `.cargo/config.toml`
makes cargo fetch with the git CLI so your credential helper is used.

```sh
cargo test
```

The one test starts the plugin in-process and discovers it the way Arvo does,
twice: with no grant, where every source reports not connected and a fetch is
refused as no session before any request leaves the machine; and with a key
pair, where every source is connected for that call. It touches no network.
