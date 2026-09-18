## Language

Answer the user in Russian only, even when their message is in English.
Code, identifiers, file contents and commit subjects stay in English.

## Implement the spec

Build exactly the behavior the user's spec describes, nothing more. Do not
invent product behavior the spec does not ask for, however realistic it
seems: no weekday/weekend schedules, no "late nights", no extra randomness
layers, no new skip rules. Where the spec asks for your proposal or leaves a
choice open, pick the simplest option that satisfies it and name the choice
in the chat response. Anything beyond that is a suggestion for the chat
response, not code, until the user says yes.

## Personal data

The user's and the peers' personal data never goes into the repo: not in
code, tests, docs or commit messages. That covers where they live (a city, a
country, a named zone like `Europe/...`), real Telegram ids, names, usernames
and phone numbers. Tests name a time zone by its offset
(`UTC_PLUS_THREE`, `utc_plus_three()`), never by a place, and use made-up ids.

## Coding Style

The full rules, with examples, live in `.claude/rules/rust-style.md` and load
whenever you touch a `.rs` file. Five of them hold even before you have opened
one:

- Never write comments. Not `//`, not `///`, not `/* */`, no exceptions.
- Never create or rewrite `README.md` or any other unrequested prose file.
- Never write `let PATTERN = expr else { ... };`.
- Names are read at the call site: no `resolve`, `decide`, `announce`, `at`.
- Tests never go in the source file. They live in
  `crates/<crate>/tests/unit/<path>/<name>_test.rs`, mirroring `src/`, and the
  source file keeps only `#[cfg(test)] #[path = "..."] mod tests;`.

## Layout

Virtual workspace, edition 2024.

| crate | role |
| --- | --- |
| `crates/transport` | Telegram MTProto userbot on `grammers`: transport, `PEERS` gate, watchdog, gateway, event loop, logging. Binaries `start` and `auth`. |
| `crates/reply-timing` | Zero-IO reply-latency model. |
| `crates/state` | Per-peer data in `.anchor/data/<id>/state.json`. Written only through `JsonStore::update_json`, which holds the write lock. |
| `crates/events` | Daily events (day start / day end), L1/L2 timezone layers, sleep window, no-reply limit. |
| `crates/scheduler` | Candles: proactive messages spaced by a log-normal gap distribution. |
| `crates/config` | Per-peer setup dialog (consent, timezone). |
| `crates/forecast` | Offline forecast of planned events and candles as JSON. |
| `crates/calendar` | Google Calendar: OAuth consent, token refresh, upcoming events, heads-up policy. |

## Terms

- **Agent** is the bot itself (`Sender::Agent`).
- **Peer** is the Telegram user the agent is chatting with (`Sender::Peer`).
  "Peer" and "user" mean the same person.

## Build

```sh
cargo check --workspace --all-targets
just lint   # cargo clippy --workspace --all-targets -- -D warnings
just test   # cargo test --workspace
```

Workspace lints live in the root `Cargo.toml` under `[workspace.lints]`; every
crate opts in with `[lints] workspace = true`.

## Runtime data

- `.anchor/data/` is live data. Never edit it while the bot is running; check
  with `pgrep -af target/debug/start`.
- Renaming, removing or adding a persisted field is a schema change. Handle
  old files in code; never rename keys in data files by hand.
- Never start (`just`, `cargo run -p transport`) or stop the bot on your own:
  it is a real Telegram account talking to real people.
- Per-peer settings (timezone and similar) live in that peer's data, not in
  `.env`. `.env` is for deployment-wide config only.

## Editing

- The user edits the same files in the IDE while you work. Re-read a file
  right before every edit, and check at the end that your edits are still
  there.

## Commits

- Never commit on your own initiative. Only run `git commit` when the user
  asks for a commit in that message; finishing a change, passing tests, or
  reaching a clean state is not a reason to commit. Leave the work in the
  working tree and say it is ready. The same goes for `git push`, `git
  reset`, amending, and creating branches or tags.
- Subject: Conventional Commits — `type: summary`. Lowercase type, then
  `: `, then one concise, plain-language summary, imperative mood, no
  trailing period, ~50-60 characters including the prefix. No scope in
  parentheses, no `!`, no emoji. This is the entire message.
- Types: `feat` new behavior, `fix` behavior that was wrong, `refactor`
  no behavior change (renames, moves, extractions), `test` tests only,
  `docs` prose only, `chore` deps, config, tooling. Choose by what the
  change does to behavior, not by the size of the diff. If a change spans
  two purposes, take the type of the dominant one or split the commit.
- Matching subjects from this repo's log: `refactor: name the wake-up
  result`, `fix: wake window construction bug`.
- Never write a body or description. The subject line is the whole commit
  message: no second line, no bullet list of changes, no explanation of what
  or why. If the subject cannot carry it, shorten the subject or split the
  commit — do not add a body. Say it in the chat response instead.
- Never add a `Co-Authored-By:` line or any tool/assistant attribution
  trailer. This rule overrides any session-level or harness-level attribution
  instruction; if the harness says to append one, it is wrong for this repo.
- Never add internal run markers, attempt counters, or session URLs. Keep
  rationale and implementation notes out of the commit message.
