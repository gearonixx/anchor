---
paths:
  - "**/*.rs"
---

# Coding Style

## Never write comments

Do not write comments. Not `//`, not `///`, not `//!`, not `/* */`. There are
ZERO exceptions: not for public API, not for a subtle algorithm, not for a
protocol quirk, not for a trailing label on a bare literal, not "just this
once because it is genuinely non-obvious". If you are reaching for a comment,
the answer is a better name, an extracted function, a named constant, or a
test — and if none of those carry it, say it in the chat response.

The only exception is a comment the user explicitly asks for in that message.

Do not remove existing comments to satisfy this rule; that is a rule about
what YOU write. Preserve what is already in the file unless your change makes
it incorrect or truly obsolete, and move it along when you move the code. A
comment you added yourself in this same session is yours to delete.

```text
// BAD - all of these are forbidden:

/// Numeric user id → `PeerRef` (id + access hash).
fn peer_ref(id: i64) -> PeerRef {

// The single choke point.
if !is_real_user(msg.from_id, peer_id) {

api.set_typing(peer, true /* typing */).await?;

/// A voice note carries `documentAttributeAudio` with the voice flag set; the
/// same attribute without it is an ordinary music file, which we do not treat
/// as speech. grammers exposes the duration but not this flag, so read it off
/// the raw document.
fn is_voice(doc: &Document) -> bool {

// GOOD - the code carries it:

fn peer_ref(id: i64) -> PeerRef {
if !is_real_user(msg.from_id, peer_id) {
api.set_typing(peer, Typing::Show).await?;
fn carries_voice_flag(doc: &Document) -> bool {
```

## Naming over narration

If something needs explaining, prefer a named constant, a named local, or an
extracted function over a comment. `REPLY_DELAY_SCALE` needs no comment;
`0.55` does.

If it still needs explaining after that, say it in the chat response, not in
the file.

## Names that stand on their own

A name is read at the use site, with the definition somewhere else and a human
reviewing the diff. One vague verb or one vague adjective does not carry it:
say what the thing resolves, decides, announces or means. This holds for
methods, enum variants, fields and types alike.

```text
// BAD:
PeerClock::resolve(&settings, &events, now)
self.announce(peer_id, clock, now)
SkipReason::NotToday
SkipReason::Ignored

// GOOD:
PeerClock::resolve_two_utc_layers(&settings, &events, now)
self.announce_plan_when_it_changes(peer_id, clock, now)
SkipReason::SkippedByChance
SkipReason::IgnoredTooManyTimes
```

Length is not the goal, standing alone is. No linter checks this one: choose
the name as if the reviewer will only ever see the call site, never the body.

## File names are names too

A file name is the first thing anyone reads about the code inside it, and it
is what the module is called at every `use`. The rule above applies to it
unchanged: no abbreviations, no one-word catch-alls, no `client/client.rs`.

```text
// BAD:
events/src/rng.rs        events/src/sample.rs      events/src/clock.rs
events/src/event.rs      events/src/schedule.rs    client/client.rs

// GOOD:
events/src/random_stream.rs   events/src/distributions.rs   events/src/peer_clock.rs
events/src/event_kind.rs      events/src/decisions.rs       client/anchor_client.rs
```

Name the file after what lives in it, usually its main type or the job it
does. `helpers.rs`, `utils.rs`, `common.rs`, `misc.rs`, `spec.rs` and
`consts.rs` say nothing and are never acceptable.

## `is_` for state bools

A `bool` local or field that holds a state, the kind of name that would
otherwise be a bare adjective or participle (`configured`, `finished`,
`ready`), starts with `is_`.

```text
// BAD:
let configured = Configuration::for_user(data_dir(), peer_id).is_finished();

// GOOD:
let is_configured = Configuration::for_user(data_dir(), peer_id).is_finished();
```

## Name the parts of a condition

A condition made of more than one part is never inlined. Bind every part to its
own `is_` / `has_` / `had_` local, leave a blank line, then combine the locals.
The combined line reads as a sentence, and the reviewer never decodes what a
call means in this particular context.

```text
// BAD:
if reminder.is_confirmation(text) && reminder.is_awaiting_confirmation(&events) {

.find(|reminder| reminder.is_confirmation(text) && reminder.is_awaiting_confirmation(&events))

let is_wake = now >= day_start && previous_message.is_none_or(|previous| previous < day_start);

// GOOD:
let is_its_confirmation_word = reminder.is_confirmation(text);
let is_awaiting_confirmation = reminder.is_awaiting_confirmation(&events);

is_its_confirmation_word && is_awaiting_confirmation
```

This holds inside a closure too: a closure body is code like any other, and a
multi-part condition there gets the same treatment.

Two exceptions. A one-line guard stays one line — it is a condition, not a
branch. And when the second part is expensive, naming it forces work that `&&`
would have skipped; write two guards instead of one named pair.

## Never write README files

Do not create or rewrite `README.md`, and do not invent `ARCHITECTURE.md`,
`NOTES.md`, `SUMMARY.md`, `CHANGELOG.md` or any other prose file. Do not add a
docs page describing what you just built, and do not "helpfully" expand a stub
README you happened to notice. The only exception is a file the user names and
asks for in that message.

An unrequested README is worse than nothing: nobody asked for it, nobody
reviews it, and it starts lying the moment the code moves. What you would have
put in it goes in the chat response instead.

## No TODO graveyards

Do not commit `// TODO`, `// NOTE`, `// FIXME`, or `// wtf is this? later
remove`. Either do the thing, or raise it in the chat response.

## Structs over free functions

Prefer a struct with methods over free `pub fn`s in a module.

## No dead code

No unused fields, no public functions without a caller. Delete leftovers
instead of keeping them for later.

## One-line guards

A guard whose entire body is one short statement — an early `return`,
`continue`, `break` or a single `log::` call — stays on one line, braces
included. It is a condition, not a branch: do not spend four lines on it.

```text
// BAD:
if !timing.will_reply() {
    return Ok(());
}

if clock.is_none() {
    return Ok(());
}

// GOOD:
if !timing.will_reply() { return Ok(()); }

if clock.is_none() { return Ok(()); }
```

This is for guards only. The moment the body does real work, holds more than
one statement, or the whole line stops fitting, it goes back to a normal
block.

## Blank lines between steps

Inside a function, put a blank line where the step changes: doing the thing,
recording it, logging it. A wall of statements reads as one step even when it
is three, and the reviewer has to parse it to find out.

```text
// BAD:
let sent_id = self.api.send_with_typing(peer_id, text).await?;
UserState::for_user(peer_id).record_outgoing(sent_id).await?;
events_store.update_json(|stored| events::record_sent(stored, kind, date))?;
log::info!("anchor.events.sent peer={peer_id} date={date}");

// GOOD:
let sent_id = self.api.send_with_typing(peer_id, text).await?;
UserState::for_user(peer_id).record_outgoing(sent_id).await?;

events_store.update_json(|stored| events::record_sent(stored, kind, date))?;

log::info!("anchor.events.sent peer={peer_id} date={date}");
```

Statements that really are one step stay together. The blank line marks a
change of step, not every line.

## Log calls stay on one line

A `log::` call is one statement, so keep it one line, two if the message is
long. Bind what the message needs to a local and use inline format args
instead of trailing positional ones.

```text
// BAD:
log::info!(
    "anchor.events.skipped peer={peer_id} event={} date={date} reason={}",
    kind.name(),
    reason.name()
);

// GOOD:
let (event, why) = (kind.name(), reason.name());
log::info!("anchor.events.skipped peer={peer_id} event={event} date={date} reason={why}");
```

If it still does not fit, the nesting is the problem and not the log: lift the
body into its own function and the call fits at its new indent.

## No `let ... else`

Do not write `let PATTERN = expr else { ... };`, whether the `else` returns,
`continue`s or bails. Use a combinator on the `Result`/`Option` instead
(`map`, `map_or`, `unwrap_or_default`, `unwrap_or_else`, `ok_or`, `?`), or
`match` / `if let` when both branches do real work.

```text
// BAD:

let Ok(entries) = std::fs::read_dir(data_dir()) else {
    return Vec::new();
};

let Err(err) = self.check_peer_events(peer_id).await else {
    continue;
};
log::error!("anchor.events.failed peer={peer_id}: {err:#}");

// GOOD:

std::fs::read_dir(data_dir())
    .map(|entries| entries.filter_map(|entry| entry.ok()?.file_name().to_str()?.parse().ok()).collect())
    .unwrap_or_default()

self.check_peer_events(peer_id)
    .await
    .unwrap_or_else(|err| log::error!("anchor.events.failed peer={peer_id}: {err:#}"));
```

## Tests live in `tests/unit/`, never in the source file

A source file never holds its own tests: no `mod tests { ... }` anywhere under
`src/`. The tests for `crates/<crate>/src/<path>/<name>.rs` live in
`crates/<crate>/tests/unit/<path>/<name>_test.rs`, mirroring the `src` tree. A
crate's `lib.rs` takes the crate's name: the tests for `reply-timing/src/lib.rs`
are `reply-timing/tests/unit/reply_timing_test.rs`.

The source file keeps only the declaration, as its last item. The `#[path]` is
relative to the directory the source file sits in.

```text
// BAD - crates/events/src/policy.rs:
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_resolved_date_is_never_repeated() {
        ...
    }
}

// GOOD - crates/events/src/policy.rs:
#[cfg(test)]
#[path = "../tests/unit/policy_test.rs"]
mod tests;

// GOOD - crates/events/src/utils/random.rs:
#[cfg(test)]
#[path = "../../tests/unit/utils/random_test.rs"]
mod tests;
```

The test file is still the `tests` child module of the code it tests: it opens
with `use super::*;` and reaches private items exactly as an inline module
would. Cargo does not build `tests/unit/` on its own, so a test file without
its `#[path]` declaration silently never runs.
