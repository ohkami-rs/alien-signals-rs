use alien_signals::{Signal, Computed, Effect, EffectScope};

#[test]
fn should_clear_subscription_when_untracked_by_all_subscribers() {
    let b_run_times = std::rc::Rc::new(std::sync::Mutex::new(0));
    
    let a = Signal::new(0);
    let b = Computed::new({
        let b_run_times = b_run_times.clone();
        move |_| {
            *b_run_times.lock().unwrap() += 1;
            a.get() * 2
        }
    });
    let effect = Effect::new(move || {
        println!("[effect] b: {}", b.get());
    });
    
    assert_eq!(*b_run_times.lock().unwrap(), 1);
    a.set(2);
    assert_eq!(*b_run_times.lock().unwrap(), 2);
    effect.dispose();
    a.set(3);
    assert_eq!(*b_run_times.lock().unwrap(), 2);
}

#[test]
fn should_not_run_untracked_inner_effect() {
    let a = Signal::new(3);
    let b = Computed::new(move |_| a.get() > 0);
    Effect::new(move || {
        if b.get() {
            Effect::new(move || {
                if a.get() == 0 {
                    panic!("bad");
                }
            });
        }
    });
    
    a.set(2);
    a.set(1);
    a.set(0);
}

#[test]
fn should_run_outer_effect_first() {
    let a = Signal::new(1);
    let b = Signal::new(1);
    
    Effect::new(move || {
        if a.get() > 0 {
            Effect::new(move || {
                let _ = b.get();
                if a.get() == 0 {
                    panic!("bad");
                }
            });
        }
    });
    
    alien_signals::start_batch();
    b.set(0);
    a.set(0);
    alien_signals::end_batch();
}

#[test]
fn should_not_trigger_inner_effect_when_resolve_maybe_dirty() {
    let a = Signal::new(0);
    let b = Computed::new(move |_| a.get() % 2);
    
    let inner_trigger_times = std::rc::Rc::new(std::sync::Mutex::new(0));
    
    Effect::new(move || {
        let inner_trigger_times = inner_trigger_times.clone();
        Effect::new(move || {
            let _ = b.get();
            *inner_trigger_times.lock().unwrap() += 1;
            if *inner_trigger_times.lock().unwrap() >= 2 {
                panic!("bad");
            }
        });
    });
    
    a.set(2);
}

#[test]
fn should_notify_inner_effects_in_the_same_order_as_non_inner_effects() {
    let a = Signal::new(0);
    let b = Signal::new(0);
    let c = Computed::new(move |_| a.get() - b.get());
    let order1 = std::rc::Rc::new(std::sync::Mutex::new(vec![]));
    let order2 = std::rc::Rc::new(std::sync::Mutex::new(vec![]));
    let order3 = std::rc::Rc::new(std::sync::Mutex::new(vec![]));
    
    Effect::new({
        let order1 = order1.clone();
        move || {
            order1.lock().unwrap().push("effect1");
            let _ = a.get();
        }
    });
    Effect::new({
        let order1 = order1.clone();
        move || {
            order1.lock().unwrap().push("effect2");
            let _ = a.get();
            let _ = b.get();
        }
    });
    
    Effect::new({
        let _ = c.get();
        let order2 = order2.clone();
        move || {
            Effect::new({
                let order2 = order2.clone();
                move || {
                    order2.lock().unwrap().push("effect1");
                    let _ = a.get();
                }
            });
            Effect::new({
                let order2 = order2.clone();
                move || {
                    order2.lock().unwrap().push("effect2");
                    let _ = a.get();
                    let _ = b.get();
                }
            });
        }
    });
    
    EffectScope::new({
        let order3 = order3.clone();
        move || {
            Effect::new({
                let order3 = order3.clone();
                move || {
                    order3.lock().unwrap().push("effect1");
                    let _ = a.get();
                }
            });
            Effect::new({
                let order3 = order3.clone();
                move || {
                    order3.lock().unwrap().push("effect2");
                    let _ = a.get();
                    let _ = b.get();
                }
            });
        }
    });
    
    order1.lock().unwrap().clear();
    order2.lock().unwrap().clear();
    order3.lock().unwrap().clear();
    
    alien_signals::start_batch();
    b.set(1);
    a.set(1);
    alien_signals::end_batch();
    
    assert_eq!(order1.lock().unwrap().as_slice(), ["effect2", "effect1"]);
    assert_eq!(order2.lock().unwrap().as_slice(), order1.lock().unwrap().as_slice());
    assert_eq!(order3.lock().unwrap().as_slice(), order1.lock().unwrap().as_slice());
}

#[test]
fn should_custom_effect_support_batch() {
    fn batch_effect<F: Fn() + 'static>(f: F) -> Effect {
        Effect::new(move || {
            alien_signals::start_batch();
            f();
            alien_signals::end_batch();
        })
    }
    
    let logs = std::rc::Rc::new(std::sync::Mutex::new(vec![]));
    let a = Signal::new(0);
    let b = Signal::new(0);
    
    let aa = Computed::new({
        let logs = logs.clone();
        move |_| {
            logs.lock().unwrap().push("aa-0");
            if a.get() == 0 {
                b.set(1);
            }
            logs.lock().unwrap().push("aa-1");
        }
    });
    let bb = Computed::new({
        let logs = logs.clone();
        move |_| {
            logs.lock().unwrap().push("bb");
            b.get()
        }
    });
    
    batch_effect(move || {
        let _ = bb.get();
    });
    batch_effect(move || {
        let _ = aa.get();
    });
    
    assert_eq!(logs.lock().unwrap().as_slice(), ["bb", "aa-0", "aa-1", "bb"]);
}

#[test]
fn should_duplicate_subscribers_do_not_affect_the_notify_order() {
    let src1 = Signal::new(0);
    let src2 = Signal::new(0);
    let order = std::rc::Rc::new(std::sync::Mutex::new(vec![]));
    
    Effect::new({
        let order = order.clone();
        move || {
            order.lock().unwrap().push("a");
            let current_sub = alien_signals::set_active_sub(None);
            let is_one = src2.get() == 1;
            alien_signals::set_active_sub(current_sub);
            if is_one {
                let _ = src1.get();
            }
            let _ = src2.get();
            let _ = src1.get();
        }
    });
    Effect::new({
        let order = order.clone();
        move || {
            order.lock().unwrap().push("b");
            let _ = src1.get();
        }
    });
    src2.set(1);
    
    order.lock().unwrap().clear();
    src1.set(src1.get() + 1);
    
    assert_eq!(order.lock().unwrap().as_slice(), ["a", "b"]);
}

#[test]
fn should_handle_side_effect_with_inner_effects() {
    let a = Signal::new(0);
    let b = Signal::new(0);
    let order = std::rc::Rc::new(std::sync::Mutex::new(vec![]));
    
    Effect::new({
        let order = order.clone();
        move || {
            Effect::new({
                let order = order.clone();
                move || {
                    let _ = a.get();
                    order.lock().unwrap().push("a");
                }
            });
            Effect::new({
                let order = order.clone();
                move || {
                    let _ = b.get();
                    order.lock().unwrap().push("b");
                }
            });
            assert_eq!(order.lock().unwrap().as_slice(), ["a", "b"]);
            
            order.lock().unwrap().clear();
            b.set(1);
            a.set(1);
            assert_eq!(order.lock().unwrap().as_slice(), ["b", "a"]);
        }
    });
}

#[test]
fn should_handle_flags_are_indirectly_updated_dyring_check_dirty() {
    let a = Signal::new(false);
    let b = Computed::new(move |_| a.get());
    let c = Computed::new(move |_| {
        let _ = b.get();
        0
    });
    let d = Computed::new(move |_| {
        let _ = c.get();
        b.get()
    });
    
    let triggers = std::rc::Rc::new(std::sync::Mutex::new(0));
    
    Effect::new({
        let triggers = triggers.clone();
        move || {
            let _ = d.get();
            *triggers.lock().unwrap() += 1;
        }
    });
    assert_eq!(*triggers.lock().unwrap(), 1);
    a.set(true);
    assert_eq!(*triggers.lock().unwrap(), 2);
}

#[test]
fn should_handle_effect_recursion_for_the_first_execution() {
    let src1 = Signal::new(0);
    let src2 = Signal::new(0);
    
    let triggers1 = std::rc::Rc::new(std::sync::Mutex::new(0));
    let triggers2 = std::rc::Rc::new(std::sync::Mutex::new(0));
    
    Effect::new({
        let triggers1 = triggers1.clone();
        move || {
            *triggers1.lock().unwrap() += 1;
            src1.set(i32::min(src1.get() + 1, 5));
        }
    });
    Effect::new({
        let triggers2 = triggers2.clone();
        move || {
            *triggers2.lock().unwrap() += 1;
            src2.set(i32::min(src2.get() + 1, 5));
            let _ = src2.get();
        }
    });
    
    assert_eq!(*triggers1.lock().unwrap(), 1);
    assert_eq!(*triggers2.lock().unwrap(), 1);
}

#[test]
fn should_support_custom_recurse_effect() {
    let src = Signal::new(0);
    
    let triggers = std::rc::Rc::new(std::sync::Mutex::new(0));
    
    Effect::new({
        let triggers = triggers.clone();
        move || {
            alien_signals::get_active_sub().unwrap().update_flags(
                |f| *f &= !alien_signals::Flags::RECURSED_CHECK
            );
            *triggers.lock().unwrap() += 1;
            src.set(i32::min(src.get() + 1, 5));
        }
    });
    
    assert_eq!(*triggers.lock().unwrap(), 6);
}
