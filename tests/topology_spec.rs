use alien_signals::{Computed, Effect, Signal};

struct SpyFn<T> {
    f: std::rc::Rc<dyn Fn() -> T>,
    count: std::rc::Rc<std::sync::Mutex<usize>>,
    last_called_time: std::rc::Rc<std::sync::Mutex<Option<std::time::Instant>>>,
    last_returned_value: std::rc::Rc<std::sync::Mutex<Option<T>>>,
}
// not requiring `T: Clone`
impl<T> Clone for SpyFn<T> {
    fn clone(&self) -> Self {
        Self {
            f: self.f.clone(),
            count: self.count.clone(),
            last_called_time: self.last_called_time.clone(),
            last_returned_value: self.last_returned_value.clone(),
        }
    }
}
impl<T> SpyFn<T> {
    fn new<F: Fn() -> T + 'static>(f: F) -> Self {
        Self {
            f: std::rc::Rc::new(f),
            count: std::rc::Rc::new(std::sync::Mutex::new(0)),
            last_called_time: std::rc::Rc::new(std::sync::Mutex::new(None)),
            last_returned_value: std::rc::Rc::new(std::sync::Mutex::new(None)),
        }
    }

    fn call(&self) -> T
    where
        T: Clone,
    {
        let result = (self.f)();
        *self.count.lock().unwrap() += 1;
        *self.last_called_time.lock().unwrap() = Some(std::time::Instant::now());
        *self.last_returned_value.lock().unwrap() = Some(result.clone());
        result
    }

    fn mock_clear(&self) {
        *self.count.lock().unwrap() = 0;
        *self.last_called_time.lock().unwrap() = None;
        *self.last_returned_value.lock().unwrap() = None;
    }

    fn to_have_been_called_times(&self, times: usize) -> bool {
        *self.count.lock().unwrap() == times
    }

    fn to_have_been_called_once(&self) -> bool {
        self.to_have_been_called_times(1)
    }

    fn to_have_been_called(&self) -> bool {
        *self.count.lock().unwrap() > 0
    }

    fn to_have_been_called_before(&self, other: &Self) -> bool {
        let self_time = self.last_called_time.lock().unwrap();
        let other_time = other.last_called_time.lock().unwrap();
        match (*self_time, *other_time) {
            (Some(self_time), Some(other_time)) => self_time < other_time,
            _ => false,
        }
    }

    fn to_have_returned_with<V>(&self, value: V) -> bool
    where
        T: PartialEq<V>,
    {
        self.last_returned_value
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|it| *it == value)
    }
}

#[test]
fn should_drop_a_b_a_updates() {
    let a = Signal::new(2);

    let b = Computed::new(move |_| a.get() - 1);
    let c = Computed::new(move |_| a.get() + b.get());

    let compute = SpyFn::new(move || format!("d: {}", c.get()));
    let d = Computed::new({
        let compute = compute.clone();
        move |_| compute.call()
    });

    assert_eq!(d.get(), "d: 3");
    assert!(compute.to_have_been_called_once());
    compute.mock_clear();

    a.set(4);
    let _ = d.get();
    assert!(compute.to_have_been_called_once());
}

#[test]
fn should_only_update_every_signal_once_in_diamond_graph() {
    let a = Signal::new("a");
    let b = Computed::new(move |_| a.get());
    let c = Computed::new(move |_| a.get());

    let spy = SpyFn::new(move || format!("{} {}", b.get(), c.get()));
    let d = Computed::new({
        let spy = spy.clone();
        move |_| spy.call()
    });

    assert_eq!(d.get(), "a a");
    assert!(spy.to_have_been_called_once());

    a.set("aa");
    assert_eq!(d.get(), "aa aa");
    assert!(spy.to_have_been_called_times(2));
}

#[test]
fn should_only_update_every_signal_once_in_diamond_graph_with_tail() {
    let a = Signal::new("a");
    let b = Computed::new(move |_| a.get());
    let c = Computed::new(move |_| a.get());

    let d = Computed::new(move |_| format!("{} {}", b.get(), c.get()));

    let spy = SpyFn::new(move || d.get());
    let e = Computed::new({
        let spy = spy.clone();
        move |_| spy.call()
    });

    assert_eq!(e.get(), "a a");
    assert!(spy.to_have_been_called_once());

    a.set("aa");
    assert_eq!(e.get(), "aa aa");
    assert!(spy.to_have_been_called_times(2));
}

#[test]
fn should_bail_out_if_result_is_the_same() {
    let a = Signal::new("a");
    let b = Computed::new(move |_| {
        let _ = a.get();
        "foo"
    });

    let spy = SpyFn::new(move || b.get());
    let c = Computed::new({
        let spy = spy.clone();
        move |_| spy.call()
    });

    assert_eq!(c.get(), "foo");
    assert!(spy.to_have_been_called_once());

    a.set("aa");
    assert_eq!(c.get(), "foo");
    assert!(spy.to_have_been_called_once());
}

#[test]
fn should_only_update_every_signal_once_in_jagged_diamond_graph_with_tails() {
    let a = Signal::new("a");

    let b = Computed::new(move |_| a.get());
    let c = Computed::new(move |_| a.get());

    let d = Computed::new(move |_| c.get());

    let e_spy = SpyFn::new(move || format!("{} {}", b.get(), d.get()));
    let e = Computed::new({
        let e_spy = e_spy.clone();
        move |_| e_spy.call()
    });

    let f_spy = SpyFn::new(move || e.get());
    let f = Computed::new({
        let f_spy = f_spy.clone();
        move |_| f_spy.call()
    });

    let g_spy = SpyFn::new(move || e.get());
    let g = Computed::new({
        let g_spy = g_spy.clone();
        move |_| g_spy.call()
    });

    assert_eq!(f.get(), "a a");
    assert!(f_spy.to_have_been_called_times(1));

    assert_eq!(g.get(), "a a");
    assert!(g_spy.to_have_been_called_times(1));

    e_spy.mock_clear();
    f_spy.mock_clear();
    g_spy.mock_clear();

    a.set("b");

    assert_eq!(e.get(), "b b");
    assert!(e_spy.to_have_been_called_times(1));

    assert_eq!(f.get(), "b b");
    assert!(f_spy.to_have_been_called_times(1));

    assert_eq!(g.get(), "b b");
    assert!(g_spy.to_have_been_called_times(1));

    e_spy.mock_clear();
    f_spy.mock_clear();
    g_spy.mock_clear();

    a.set("c");

    assert_eq!(e.get(), "c c");
    assert!(e_spy.to_have_been_called_times(1));

    assert_eq!(f.get(), "c c");
    assert!(f_spy.to_have_been_called_times(1));

    assert_eq!(g.get(), "c c");
    assert!(g_spy.to_have_been_called_times(1));

    assert!(e_spy.to_have_been_called_before(&f_spy));
    assert!(f_spy.to_have_been_called_before(&g_spy));
}

#[test]
fn should_only_subscribe_to_signals_listened_to() {
    let a = Signal::new("a");

    let b = Computed::new(move |_| a.get());
    let spy = SpyFn::new(move || a.get());
    let _ = Computed::new({
        let spy = spy.clone();
        move |_| spy.call()
    });

    assert_eq!(b.get(), "a");
    assert!(!spy.to_have_been_called());

    a.set("aa");
    assert_eq!(b.get(), "aa");
    assert!(!spy.to_have_been_called());
}

#[test]
fn should_only_subscribe_to_signals_listened_to_2() {
    let a = Signal::new("a");
    let spy_b = SpyFn::new(move || a.get());
    let b = Computed::new({
        let spy_b = spy_b.clone();
        move |_| spy_b.call()
    });

    let spy_c = SpyFn::new(move || b.get());
    let c = Computed::new({
        let spy_c = spy_c.clone();
        move |_| spy_c.call()
    });

    let d = Computed::new(move |_| a.get());

    let result = std::rc::Rc::new(std::sync::Mutex::new(""));
    let effect = Effect::new({
        let result = result.clone();
        move || *result.lock().unwrap() = c.get()
    });

    assert_eq!(*result.lock().unwrap(), "a");
    assert_eq!(d.get(), "a");

    spy_b.mock_clear();
    spy_c.mock_clear();
    effect.dispose();

    a.set("aa");

    assert!(!spy_b.to_have_been_called());
    assert!(!spy_c.to_have_been_called());
    assert_eq!(d.get(), "aa");
}

#[test]
fn should_ensure_subs_update_even_if_one_dep_unmark_it() {
    let a = Signal::new("a");
    let b = Computed::new(move |_| a.get());
    let c = Computed::new(move |_| {
        let _ = a.get();
        "c"
    });
    let spy = SpyFn::new(move || format!("{} {}", b.get(), c.get()));
    let d = Computed::new({
        let spy = spy.clone();
        move |_| spy.call()
    });

    assert_eq!(d.get(), "a c");
    spy.mock_clear();

    a.set("aa");
    let _ = d.get();
    assert!(spy.to_have_returned_with("aa c"));
}

#[test]
fn should_ensure_subs_update_even_if_two_deps_unmark_it() {
    let a = Signal::new("a");
    let b = Computed::new(move |_| a.get());
    let c = Computed::new(move |_| {
        let _ = a.get();
        "c"
    });
    let d = Computed::new(move |_| {
        let _ = a.get();
        "d"
    });
    let spy = SpyFn::new(move || format!("{} {} {}", b.get(), c.get(), d.get()));
    let e = Computed::new({
        let spy = spy.clone();
        move |_| spy.call()
    });

    assert_eq!(e.get(), "a c d");
    spy.mock_clear();

    a.set("aa");
    let _ = e.get();
    assert!(spy.to_have_returned_with("aa c d"));
}

#[test]
fn should_support_lazy_branches() {
    let a = Signal::new(0);
    let b = Computed::new(move |_| a.get());
    let c = Computed::new(move |_| if a.get() > 0 { a.get() } else { b.get() });

    assert_eq!(c.get(), 0);
    a.set(1);
    assert_eq!(c.get(), 1);

    a.set(0);
    assert_eq!(c.get(), 0);
}

#[test]
fn should_not_update_a_sub_if_all_deps_unmark_it() {
    let a = Signal::new("a");
    let b = Computed::new(move |_| {
        let _ = a.get();
        "b"
    });
    let c = Computed::new(move |_| {
        let _ = a.get();
        "c"
    });
    let spy = SpyFn::new(move || format!("{} {}", b.get(), c.get()));
    let d = Computed::new({
        let spy = spy.clone();
        move |_| spy.call()
    });

    assert_eq!(d.get(), "b c");
    spy.mock_clear();

    a.set("aa");
    assert!(!spy.to_have_been_called());
}

#[test]
fn should_keep_graph_consistent_on_errors_during_activation() {
    /* TODO */
}

#[test]
fn should_keep_graph_consistent_on_errors_during_computeds() {
    /* TODO */
}
