use alien_signals::{Effect, EffectScope, Signal};

#[test]
fn should_not_trigger_after_stop() {
    let count = Signal::new(0);

    let triggers = std::rc::Rc::new(std::sync::Mutex::new(0));

    let mut _effect1 = None;

    #[allow(unused_assignments)]
    let effect_scope = EffectScope::new({
        let triggers = triggers.clone();
        move || {
            _effect1 = Some(Effect::new({
                let triggers = triggers.clone();
                move || {
                    *triggers.lock().unwrap() += 1;
                    let _ = count.get();
                }
            }));
            assert_eq!(*triggers.lock().unwrap(), 1);

            count.set(2);
            assert_eq!(*triggers.lock().unwrap(), 2);
        }
    });

    count.set(3);
    assert_eq!(*triggers.lock().unwrap(), 3);
    effect_scope.dispose();
    count.set(4);
    assert_eq!(*triggers.lock().unwrap(), 3);
}

#[test]
fn should_dispose_inner_effects_if_created_in_an_effect() {
    let source = Signal::new(1);

    let triggers = std::rc::Rc::new(std::sync::Mutex::new(0));

    Effect::new({
        let triggers = triggers.clone();
        move || {
            let effect_scope = EffectScope::new({
                let triggers = triggers.clone();
                move || {
                    Effect::new(move || {
                        let _ = source.get();
                        *triggers.lock().unwrap() += 1;
                    });
                }
            });
            assert_eq!(*triggers.lock().unwrap(), 1);

            source.set(2);
            assert_eq!(*triggers.lock().unwrap(), 2);
            effect_scope.dispose();
            source.set(3);
            assert_eq!(*triggers.lock().unwrap(), 2);
        }
    });
}

#[test]
fn should_track_signal_updates_in_an_inner_scope_when_accessed_by_an_outer_effect() {
    let source = Signal::new(1);

    let triggers = std::rc::Rc::new(std::sync::Mutex::new(0));

    Effect::new({
        let triggers = triggers.clone();
        move || {
            EffectScope::new(move || {
                let _ = source.get();
            });
            *triggers.lock().unwrap() += 1;
        }
    });

    assert_eq!(*triggers.lock().unwrap(), 1);
    source.set(2);
    assert_eq!(*triggers.lock().unwrap(), 2);
}
