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

Deploy (fly.io free tier):

    fly launch --no-deploy --copy-config    # name the app wyatt-viper
    fly deploy
    fly status                              # the https://wyatt-viper.fly.dev URL

Then register the URL at play.battlesnake.com and join the arena — 24/7
ladder. The config keeps one always-on machine (a cold start would blow
the ~500 ms per-move timeout). Docker: `docker build -t wyatt-viper .`
works anywhere else Rust containers run.

## Logic ladder

1. never hit a wall / body (tails vacate unless health == 100)
2. never lose a head-to-head (only contest cells adjacent to shorter enemies)
3. maximize flood-fill space, then nearest food

## Related

- `WyattAu/bot-kit` — shared bot infrastructure (C++ side; Rust kit extracts here-from when warranted)
- `WyattAu/competitions` — the registry
