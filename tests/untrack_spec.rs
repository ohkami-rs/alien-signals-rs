use alien_signals::{Computed, Effect, EffectScope, Signal, set_active_sub};

#[test]
fn should_pause_tracking_in_computed() {
    let src = Signal::new(0);

    let computed_trigger_times = std::rc::Rc::new(std::sync::Mutex::new(0));
    let c = Computed::new({
        let computed_trigger_times = computed_trigger_times.clone();
        move |_| {
            *computed_trigger_times.lock().unwrap() += 1;
            let current_sub = set_active_sub(None);
            let value = src.get();
            set_active_sub(current_sub);
            value
        }
    });

    assert_eq!(c.get(), 0);
    assert_eq!(*computed_trigger_times.lock().unwrap(), 1);

    src.set(1);
    src.set(2);
    src.set(3);

    assert_eq!(c.get(), 0);
    assert_eq!(*computed_trigger_times.lock().unwrap(), 1);
}

#[test]
fn should_pause_tracking_in_effect() {
    let src = Signal::new(0);
    let is = Signal::new(0);

    let effect_trigger_times = std::rc::Rc::new(std::sync::Mutex::new(0));
    Effect::new({
        let effect_trigger_times = effect_trigger_times.clone();
        move || {
            *effect_trigger_times.lock().unwrap() += 1;
            if is.get() > 0 {
                let current_sub = set_active_sub(None);
                let _ = src.get();
                set_active_sub(current_sub);
            }
        }
    });

    assert_eq!(*effect_trigger_times.lock().unwrap(), 1);

    is.set(1);
    assert_eq!(*effect_trigger_times.lock().unwrap(), 2);

    src.set(1);
    src.set(2);
    src.set(3);
    assert_eq!(*effect_trigger_times.lock().unwrap(), 2);

    is.set(2);
    assert_eq!(*effect_trigger_times.lock().unwrap(), 3);

    src.set(4);
    src.set(5);
    src.set(6);
    assert_eq!(*effect_trigger_times.lock().unwrap(), 3);

    is.set(0);
    assert_eq!(*effect_trigger_times.lock().unwrap(), 4);

    src.set(7);
    src.set(8);
    src.set(9);
    assert_eq!(*effect_trigger_times.lock().unwrap(), 4);
}

#[test]
fn should_dispose_computed_in_effect_scope() {
    let src = Signal::new(0);

    let effect_trigger_times = std::rc::Rc::new(std::sync::Mutex::new(0));
    EffectScope::new({
        let effect_trigger_times = effect_trigger_times.clone();
        move || {
            Effect::new({
                let effect_trigger_times = effect_trigger_times.clone();
                move || {
                    *effect_trigger_times.lock().unwrap() += 1;
                    let current_sub = set_active_sub(None);
                    let _ = src.get();
                    set_active_sub(current_sub);
                }
            });
        }
    });

    assert_eq!(*effect_trigger_times.lock().unwrap(), 1);

    src.set(1);
    src.set(2);
    src.set(3);
    assert_eq!(*effect_trigger_times.lock().unwrap(), 1);
}
