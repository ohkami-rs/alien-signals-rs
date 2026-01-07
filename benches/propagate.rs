#![feature(test)]
extern crate test;

use alien_signals::{Computed, Effect, Signal};

#[derive(Clone, Copy)]
enum SignalOrComputed {
    Signal(Signal<i32>),
    Computed(Computed<i32>),
}
impl SignalOrComputed {
    fn get(&self) -> i32 {
        match self {
            SignalOrComputed::Signal(s) => s.get(),
            SignalOrComputed::Computed(c) => c.get(),
        }
    }
}

macro_rules! propagate_w_h {
    ($bench_name:ident: $w:literal x $h:literal) => {
        #[bench]
        fn $bench_name(b: &mut test::Bencher) {
            let src = Signal::new(1);
            for _ in 0..$w {
                let mut last = SignalOrComputed::Signal(src);
                for _ in 0..$h {
                    let prev = last;
                    last = SignalOrComputed::Computed(Computed::new(move |_| prev.get() + 1));
                }
                Effect::new(move || {let _ = last.get();});
            }
            b.iter(|| {
                src.set(src.get() + 1);
            });
        }        
    };
}
propagate_w_h! {propagate_1x1: 1 x 1}
propagate_w_h! {propagate_1x10: 1 x 10}
propagate_w_h! {propagate_1x100: 1 x 100}
propagate_w_h! {propagate_1x1000: 1 x 1000}
//propagate_w_h! {propagate_1x10000: 1 x 10000}
propagate_w_h! {propagate_10x1: 10 x 1}
propagate_w_h! {propagate_10x10: 10 x 10}
propagate_w_h! {propagate_10x100: 10 x 100}
propagate_w_h! {propagate_10x1000: 10 x 1000}
//propagate_w_h! {propagate_10x10000: 10 x 10000}
propagate_w_h! {propagate_100x1: 100 x 1}
propagate_w_h! {propagate_100x10: 100 x 10}
propagate_w_h! {propagate_100x100: 100 x 100}
propagate_w_h! {propagate_100x1000: 100 x 1000}
//propagate_w_h! {propagate_100x10000: 100 x 10000}
propagate_w_h! {propagate_1000x1: 1000 x 1}
propagate_w_h! {propagate_1000x10: 1000 x 10}
propagate_w_h! {propagate_1000x100: 1000 x 100}
// propagate_w_h! {propagate_1000x1000: 1000 x 1000}
//propagate_w_h! {propagate_1000x10000: 1000 x 10000}
//propagate_w_h! {propagate_10000x1: 10000 x 1}
//propagate_w_h! {propagate_10000x10: 10000 x 10}
//propagate_w_h! {propagate_10000x100: 10000 x 100}
//propagate_w_h! {propagate_10000x1000: 10000 x 1000}
//propagate_w_h! {propagate_10000x10000: 10000 x 10000}
