use alien_signals::{Computed, Effect, Signal, trigger};

#[test]
fn should_not_throw_when_triggering_with_no_dependencies() {
    trigger(|| {});
}

#[test]
fn should_trigger_updates_for_dependent_computed_signals() {
    let arr = Signal::new(vec![]);
    let length = Computed::new(move |_| arr.get().len());

    assert_eq!(length.get(), 0);
    arr.update(|arr| arr.push(1));
    trigger(move || {
        let _ = arr.get();
    });
    assert_eq!(length.get(), 1);
}

#[test]
fn should_trigger_updates_for_the_second_source_signal() {
    let src1 = Signal::<Vec<i32>>::new(vec![]);
    let src2 = Signal::<Vec<i32>>::new(vec![]);
    let length = Computed::new(move |_| src2.get().len());

    assert_eq!(length.get(), 0);
    src2.update(|arr| arr.push(1));
    trigger(move || {
        let _ = src1.get();
        let _ = src2.get();
    });
    assert_eq!(length.get(), 1);
}

#[test]
fn should_trigger_effect_once() {
    let src1 = Signal::<Vec<i32>>::new(vec![]);
    let src2 = Signal::<Vec<i32>>::new(vec![]);

    let triggers = std::rc::Rc::new(std::sync::Mutex::new(0));

    Effect::new({
        let triggers = triggers.clone();
        move || {
            *triggers.lock().unwrap() += 1;
            let _ = src1.get();
            let _ = src2.get();
        }
    });

    assert_eq!(*triggers.lock().unwrap(), 1);
    trigger(move || {
        let _ = src1.get();
        let _ = src2.get();
    });
    assert_eq!(*triggers.lock().unwrap(), 2);
}

#[test]
fn should_not_notify_the_trigger_function_sub() {
    let src1 = Signal::<Vec<i32>>::new(vec![]);
    let src2 = Computed::new(move |_| src1.get());

    Effect::new(move || {
        let _ = src1.get();
        let _ = src2.get();
    });
    trigger(move || {
        let _ = src1.get();
        let _ = src2.get();
    });
}
