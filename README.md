# Whispers

Whispers is a self-organizing, belief-driven mesh where nodes propose, verify, and evolve solutions through dynamic, decentralized consensus.

It's a simple idea, but I think it's rather beautiful: nodes find one another via [Multicast DNS (mDNS)](https://en.wikipedia.org/wiki/Multicast_DNS) and exchange information via [Conflict-free Replicated Data Types (CRDTs)](https://en.wikipedia.org/wiki/Conflict-free_replicated_data_type).

There's more to it than that, a lot more, but we'll get there when we get there 🙂

## Instructions

To run a Whispers mesh, just check out the repository and enter:

```bash
RUST_LOG=debug cargo run
```

(Some release process will manifest at some point.)

That launches a single node, but a mesh of one node isn't particularly interesting.

If you launch a second, third, fourth, etc, then those nodes should discover each other and begin gossipping with one another. You won't see much of this unless you set the `RUST_LOG` environment variable as described above.
