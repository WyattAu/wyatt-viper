# wyatt-viper

WyattAu's Battlesnake entry — [`play.battlesnake.com`](https://play.battlesnake.com).
Axum webhook server (`/`, `/start`, `/move`, `/end`) over a pure, unit-tested
game core: tail-aware safety → head-to-head avoidance → flood-fill space →
food attraction.

Self-contained by estate policy: shared crates are extracted when a second
consumer appears, not before (a `-kit` crate needs two consumers and a pin to
earn its existence).

## Run

```sh
cargo run -p viper        # PORT=8000 default
cargo test -p viper
```

Deploy anywhere Rust runs (fly.io / render free tier), then register the URL at
play.battlesnake.com and join the arena — 24/7 ladder.

## Logic ladder

1. never hit a wall / body (tails vacate unless health == 100)
2. never lose a head-to-head (only contest cells adjacent to shorter enemies)
3. maximize flood-fill space, then nearest food

## Related

- `WyattAu/bot-kit` — shared bot infrastructure (C++ side; Rust kit extracts here-from when warranted)
- `WyattAu/competitions` — the registry
