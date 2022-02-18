---
title: Rust API
---

Solana's Rust crates are [published to crates.io][crates.io] and can be found
[on docs.rs with the "solana-" prefix][docs.rs].

[crates.io]: https://crates.io/search?q=solana-
[docs.rs]: https://docs.rs/releases/search?query=solana-

Some important crates:

- [`renec-program`] &mdash; Imported by programs running on Solana, compiled
  to BPF. This crate contains many fundamental data types and is re-exported from
  [`solana-sdk`], which cannot be imported from a Solana program.

- [`solana-sdk`] &mdash; The basic off-chain SDK, it re-exports
  [`renec-program`] and adds more APIs on top of that. Most Solana programs
  that do not run on-chain will import this.

- [`renec-client`] &mdash; For interacting with a Solana node via the
  [JSON RPC API](jsonrpc-api).

- [`renec-clap-utils`] &mdash; Routines for setting up a CLI, using [`clap`],
  as used by the main Solana CLI.

[`renec-program`]: https://docs.rs/renec-program
[`solana-sdk`]: https://docs.rs/solana-sdk
[`renec-client`]: https://docs.rs/renec-client
[`renec-clap-utils`]: https://docs.rs/renec-clap-utils
[`clap`]: https://docs.rs/clap
