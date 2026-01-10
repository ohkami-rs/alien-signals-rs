<div align="center">
    <h1>
        👾 Rust port of alien-signals
    </h1>
</div>

Rust port of [alien-signals](https://github.com/stackblitz/alien-signals),
the lightest signal library.

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
    Finished `bench` profile [optimized] target(s) in 0.00s
     Running unittests src/lib.rs (target/release/deps/alien_signals-94b75a42132c2eac)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running benches/propagate.rs (target/release/deps/propagate-b10b33e3850cf411)

running 15 tests
test propagate_1000x1   ... bench:      39,261.97 ns/iter (+/- 6,405.87)
test propagate_1000x10  ... bench:     373,183.79 ns/iter (+/- 8,104.54)
test propagate_1000x100 ... bench:   3,621,530.55 ns/iter (+/- 232,668.73)
test propagate_100x1    ... bench:       3,921.90 ns/iter (+/- 102.03)
test propagate_100x10   ... bench:      30,253.56 ns/iter (+/- 940.06)
test propagate_100x100  ... bench:     377,884.20 ns/iter (+/- 8,133.70)
test propagate_100x1000 ... bench:   3,510,062.10 ns/iter (+/- 80,081.13)
test propagate_10x1     ... bench:         504.37 ns/iter (+/- 22.74)
test propagate_10x10    ... bench:       3,432.22 ns/iter (+/- 83.34)
test propagate_10x100   ... bench:      34,663.05 ns/iter (+/- 269.54)
test propagate_10x1000  ... bench:     434,908.12 ns/iter (+/- 32,387.63)
test propagate_1x1      ... bench:          86.46 ns/iter (+/- 7.62)
test propagate_1x10     ... bench:         397.96 ns/iter (+/- 13.46)
test propagate_1x100    ... bench:       3,598.66 ns/iter (+/- 183.45)
test propagate_1x1000   ... bench:      38,550.69 ns/iter (+/- 10,542.25)

test result: ok. 0 passed; 0 failed; 0 ignored; 15 measured; 0 filtered out; finished in 32.82s

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
