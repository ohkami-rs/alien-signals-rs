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
   Compiling alien-signals v0.1.3 (/home/kanarus/projects/ohkami-rs/alien-signals-rs)
    Finished `bench` profile [optimized] target(s) in 0.79s
     Running unittests src/lib.rs (target/release/deps/alien_signals-f7e64dac92c4dbf6)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running benches/propagate.rs (target/release/deps/propagate-e7c35173b626601b)

running 15 tests
test propagate_1000x1   ... bench:      35,966.36 ns/iter (+/- 407.67)
test propagate_1000x10  ... bench:     342,868.96 ns/iter (+/- 6,842.63)
test propagate_1000x100 ... bench:   3,485,965.60 ns/iter (+/- 68,293.28)
test propagate_100x1    ... bench:       3,693.75 ns/iter (+/- 106.61)
test propagate_100x10   ... bench:      28,446.80 ns/iter (+/- 653.14)
test propagate_100x100  ... bench:     344,254.59 ns/iter (+/- 5,106.58)
test propagate_100x1000 ... bench:   3,458,156.85 ns/iter (+/- 151,837.81)
test propagate_10x1     ... bench:         437.23 ns/iter (+/- 11.92)
test propagate_10x10    ... bench:       3,169.95 ns/iter (+/- 98.84)
test propagate_10x100   ... bench:      32,334.04 ns/iter (+/- 555.22)
test propagate_10x1000  ... bench:     383,309.50 ns/iter (+/- 2,949.69)
test propagate_1x1      ... bench:          79.02 ns/iter (+/- 2.82)
test propagate_1x10     ... bench:         357.39 ns/iter (+/- 15.32)
test propagate_1x100    ... bench:       3,178.23 ns/iter (+/- 54.87)
test propagate_1x1000   ... bench:      31,861.69 ns/iter (+/- 1,070.36)

test result: ok. 0 passed; 0 failed; 0 ignored; 15 measured; 0 filtered out; finished in 30.08s

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
use alien_signals::{signal, computed, effect};

fn main() {
    let count = signal(1);
    let double_count = computed(move |_| count.get() * 2);
    
    effect(move || {
        println!("Count is: {}", count.get());
    }); // Count is: 1
    
    assert_eq!(double_count.get(), 2);
    
    count.set(2); // Count is: 2
    
    assert_eq!(double_count.get(), 4);
}
```

### Effect Scope

```rust
use alien_signals::{signal, effect, effect_scope};

fn main() {
    let count = signal(1);
    
    let scope = effect_scope(move || {
        effect(move || {
            println!("Count in scope: {}", count.get());
        }); // Count in scope: 1
    });
    
    count.set(2); // Count in scope: 2
    
    scope.dispose();
    
    count.set(3); // prints nothing
}
```

### Nested Effects

```rust
use alien_signals::{signal, effect};

fn main() {
    let show = signal(true);
    let count = signal(1);
    
    effect(move || {
        if show.get() {
            // This inner effect is created when `show` is true
            effect(move || {
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
use alien_signals::{Signal, computed, trigger};
use std::{sync::Mutex, rc::Rc};

fn main() {
    let arr = Signal::new_with_eq_fn(
        Rc::new(Mutex::new(vec![])),
        |a, b| *a.lock().unwrap() == *b.lock().unwrap()
    );
    let length = computed(move |_| arr.get().lock().unwrap().len());
    
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
