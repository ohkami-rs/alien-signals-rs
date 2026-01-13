<div align="center">
    <h1>
        👾 alien-signals
    </h1>
    <p>
        Rust port of <a href="https://github.com/stackblitz/alien-signals">alien-signals</a>,
        the lightest signal library.
    </p>
</div>

<div align="right">
    <a href="https://github.com/ohkami-rs/alien-signals-rs/blob/main/LICENSE"><img alt="License" src="https://img.shields.io/crates/l/alien-signals.svg" /></a>
    <a href="https://github.com/ohkami-rs/alien-signals-rs/actions"><img alt="CI status" src="https://github.com/ohkami-rs/alien-signals-rs/actions/workflows/CI.yml/badge.svg"/></a>
    <a href="https://crates.io/crates/alien-signals"><img alt="crates.io" src="https://img.shields.io/crates/v/alien-signals" /></a>
</div>

**NOTE**: This is not thread-safe.

## Benchmarks against original implementation

### original (Node.js)

```sh
pnpm run bench

> alien-signals@3.1.2 bench /home/kanarus/projects/ohkami-rs/alien-signals-rs/vs/alien-signals
> npm run build && node --jitless --expose-gc benchs/propagate.mjs


> alien-signals@3.1.2 build
> node ./build.js

clk: ~0.06 GHz
cpu: 12th Gen Intel(R) Core(TM) i7-1280P
runtime: node 24.0.0 (x64-linux)

| benchmark            |              avg |         min |         p75 |         p99 |         max |
| -------------------- | ---------------- | ----------- | ----------- | ----------- | ----------- |
| propagate: 1 * 1     | `831.94 ns/iter` | `789.59 ns` | `845.55 ns` | `876.03 ns` | `879.31 ns` |
| propagate: 1 * 10    | `  3.45 µs/iter` | `  3.41 µs` | `  3.46 µs` | `  3.49 µs` | `  3.49 µs` |
| propagate: 1 * 100   | ` 28.92 µs/iter` | ` 28.59 µs` | ` 29.00 µs` | ` 29.26 µs` | ` 29.27 µs` |
| propagate: 10 * 1    | `  6.37 µs/iter` | `  6.28 µs` | `  6.42 µs` | `  6.46 µs` | `  6.48 µs` |
| propagate: 10 * 10   | ` 32.55 µs/iter` | ` 32.34 µs` | ` 32.70 µs` | ` 32.78 µs` | ` 32.83 µs` |
| propagate: 10 * 100  | `286.97 µs/iter` | `255.01 µs` | `287.06 µs` | `328.85 µs` | `339.39 µs` |
| propagate: 100 * 1   | ` 60.26 µs/iter` | ` 59.36 µs` | ` 60.67 µs` | ` 61.29 µs` | ` 61.29 µs` |
| propagate: 100 * 10  | `323.37 µs/iter` | `290.19 µs` | `326.50 µs` | `365.46 µs` | `391.99 µs` |
| propagate: 100 * 100 | `  3.01 ms/iter` | `  2.86 ms` | `  3.04 ms` | `  3.19 ms` | `  3.21 ms` |
```

### this port (Rust)

```sh
cargo +nightly bench --benches
Compiling alien-signals v0.1.2 (/home/kanarus/projects/ohkami-rs/alien-signals-rs)
 Finished `bench` profile [optimized] target(s) in 0.74s
  Running unittests src/lib.rs (target/release/deps/alien_signals-c1d8f41e346bb6d2)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

  Running benches/propagate.rs (target/release/deps/propagate-f6f937cb450c5a9e)

running 15 tests
test propagate_1000x1   ... bench:      36,234.27 ns/iter (+/- 500.33)
test propagate_1000x10  ... bench:     338,003.96 ns/iter (+/- 7,161.70)
test propagate_1000x100 ... bench:   3,465,057.00 ns/iter (+/- 94,077.96)
test propagate_100x1    ... bench:       3,751.76 ns/iter (+/- 67.53)
test propagate_100x10   ... bench:      29,595.09 ns/iter (+/- 432.33)
test propagate_100x100  ... bench:     355,101.43 ns/iter (+/- 7,331.37)
test propagate_100x1000 ... bench:   3,445,382.20 ns/iter (+/- 115,326.96)
test propagate_10x1     ... bench:         419.20 ns/iter (+/- 51.41)
test propagate_10x10    ... bench:       3,292.87 ns/iter (+/- 145.36)
test propagate_10x100   ... bench:      33,133.96 ns/iter (+/- 406.60)
test propagate_10x1000  ... bench:     396,047.30 ns/iter (+/- 6,794.45)
test propagate_1x1      ... bench:          81.68 ns/iter (+/- 2.21)
test propagate_1x10     ... bench:         370.48 ns/iter (+/- 5.30)
test propagate_1x100    ... bench:       3,258.06 ns/iter (+/- 143.72)
test propagate_1x1000   ... bench:      32,223.24 ns/iter (+/- 650.91)

test result: ok. 0 passed; 0 failed; 0 ignored; 15 measured; 0 filtered out; finished in 41.16s

```

## Installation

```sh
cargo add alien-signals
```

or

```toml
# Cargo.toml

[dependencies]
alien-signals = "0.1"
```

## Usage

### Basic APIs

```rust
use alien_signals::{Signal, Computed, Effect};

fn main() {
    let count = Signal::new(1);
    let double_count = Computed::new(move |_| count.get() * 2);
    
    Effect::new(move || {
        println!("Count is: {}", count.get());
    }); // Count is: 1
    
    assert_eq!(double_count.get(), 2);
    
    count.set(2); // Count is: 2
    
    assert_eq!(double_count.get(), 4);
}
```

### Effect Scope

```rust
use alien_signals::{Signal, Effect, EffectScope};

fn main() {
    let count = Signal::new(1);
    
    let effect = EffectScope::new(move || {
        Effect::new(move || {
            println!("Count in scope: {}", count.get());
        }); // Count in scope: 1
    });
    
    count.set(2); // Count in scope: 2
    
    effect.dispose();
    
    count.set(3); // prints nothing
}
```

### Nested Effects

```rust
use alien_signals::{Signal, Effect};

fn main() {
    let show = Signal::new(true);
    let count = Signal::new(1);
    
    Effect::new(move || {
        if show.get() {
            // This inner effect is created when `show` is true
            Effect::new(move || {
                println!("Count is: {}", count.get());
            });
        }
    }); // Count is: 1
    
    count.set(2); // Count is: 2
    
    // When show becomes false, the inner effect is cleaned up
    show.set(false);
    
    count.set(3); // prints nothing (inner effect no longer exists)
}
```

### Manual Triggering

```rust
use alien_signals::{Signal, Computed, trigger};
use std::{sync::Mutex, rc::Rc};

fn main() {
    let arr = Signal::new_with_eq_fn(
        Rc::new(Mutex::new(vec![])),
        |a, b| *a.lock().unwrap() == *b.lock().unwrap()
    );
    let length = Computed::new(move |_| arr.get().lock().unwrap().len());
    
    assert_eq!(length.get(), 0);
    
    // Direct mutation forcibly without signal setter
    // doesn't automatically trigger updates
    arr.get().lock().unwrap().push(1);
    assert_eq!(length.get(), 0); // Still 0
    
    // Manually trigger updates
    trigger(move || {
        arr.get();
    });
    assert_eq!(length.get(), 1);
}
```
