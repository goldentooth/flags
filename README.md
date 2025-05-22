# Flags

Flags is a self-organizing high-performance feature flag server cluster.

## Wait, what?

Some weeks back, I applied for a backend position focused on feature flags at a well-known small-insectivorous-non-rodent-based company.

Recently, after concluding a pleasant call with their internal recruiter, I casually glanced back at the job description... and was a bit horrified. I didn't actually have any experience developing or maintaining feature flags at the scale or performance levels mentioned as required in the job description.

I take great pains to be honest in my cover letters, résumés, and interviews, so this is kind of a weird and uncomfortable situation to be in. I couldn't really reconcile the fact that they were interested in me with my lack of a history of operating successfully at that scale.

So I felt a momentary urge to email the recruiter in a panic and say, "er... just for my records... would you mind sending me back a copy of my cover letter?" I was picturing, perhaps, a moment of realization like Wendy Torrance's in _The Shining_ (1980) or Harry Angel's in _Angel Heart_ (1987) or something like that, where I'd find out that I had been prattling on about working for Netflix or AWS or eBay or something.

So anyway, this is an experiment to find out what sort of performance I can get out of a feature flag app with a reasonable set of features and an interesting architecture in a very short amount of time (i.e. a few days).

# How could you possibly make this process more stressful and high-pressure?

Because I don't want to compile a Rust app on my Pis, I guess I'm going to learn about modern cross-compilation too. I haven't cross-compiled anything in about twenty years, since I built a Gentoo desktop system for an SGI Indy on an AMD Athlon. That experience, you might reasonably conclude, explains why I haven't cross-compiled anything in the intervening decades.

Also, I guess I need to benchmark this system in some way, so I'm going to build an app to do that as well. Let's see how that goes. That project's gonna be called [Flood](https://github.com/goldentooth/flood/).

## Instructions

To run a Flags mesh, just check out the repository and enter:

```bash
RUST_LOG=debug cargo run
```

(Some release process will manifest at some point.)

That launches a single node, but a mesh of one node isn't particularly interesting.

If you launch a second, third, fourth, etc, then those nodes should discover each other and begin gossipping with one another. You won't see much of this unless you set the `RUST_LOG` environment variable as described above.

Alternatively, you can run `./easy_cluster.sh x`, where `x` is some number of nodes you would like to run simultaneously in a self-configuring cluster, e.g. `32`.

## Cross-Compilation

To cross-compile for a Raspberry Pi:

```bash
export DOCKER_DEFAULT_PLATFORM=linux/x86_64/v2
cross build --release --target=aarch64-unknown-linux-gnu
```