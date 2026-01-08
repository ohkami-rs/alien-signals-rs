use alien_signals::{Computed, Signal};

#[test]
fn should_correctly_propagate_changes_through_computed_signals() {
    let src = Signal::new(0);
    let c1 = Computed::new(move |_| {
        println!("before src.get()");
        let src_mod_2 = src.get() % 2;
        println!("after src.get()");
        src_mod_2
    });
    let c2 = Computed::new(move |_| {
        println!("before c1.get()");
        let c1 = c1.get();
        println!("after c1.get()");
        c1
    });
    let c3 = Computed::new(move |_| {
        println!("before c2.get()");
        let c2 = c2.get();
        println!("after c2.get()");
        c2
    });

    let _ = c3.get();
    src.set(1);
    let _ = c2.get();
    src.set(3);

    assert_eq!(c3.get(), 1);
}

#[test]
fn should_propagate_updated_source_value_through_chained_computation() {
    let src = Signal::new(0);
    let a = Computed::new(move |_| src.get());
    let b = Computed::new(move |_| a.get() % 2);
    let c = Computed::new(move |_| src.get());
    let d = Computed::new(move |_| b.get() + c.get());

    assert_eq!(d.get(), 0);
    src.set(2);
    assert_eq!(d.get(), 2);
}

#[test]
fn should_not_update_if_the_signal_value_is_reverted() {
    let times = std::rc::Rc::new(std::sync::Mutex::new(0));

    let src = Signal::new(0);
    let c1 = Computed::new({
        let times = times.clone();
        move |_| {
            *times.lock().unwrap() += 1;
            src.get()
        }
    });

    let _ = c1.get();
    assert_eq!(*times.lock().unwrap(), 1);
    src.set(1);
    src.set(0);
    let _ = c1.get();
    assert_eq!(*times.lock().unwrap(), 1);
}
