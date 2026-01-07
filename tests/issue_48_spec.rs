use alien_signals::{Computed, Effect, set_active_sub, Signal};

#[test]
fn issue48() {
    let source = Signal::new(0);
    let dispose_inner: std::rc::Rc<std::sync::Mutex<Option<Box<dyn FnOnce()>>>>
        = std::rc::Rc::new(std::sync::Mutex::new(None));
    
    let _ = reaction(
        move || source.get(),
        move |val, _| {
            if *val == 1 {
                let dispose_reaction = reaction(
                    move || source.get(),
                    |_, _| {},
                    Default::default(),
                );
                dispose_inner.lock().unwrap().replace(Box::new(dispose_reaction));
            } else if *val == 2 {
                if let Some(dispose_inner) = dispose_inner.lock().unwrap().take() {
                    dispose_inner();
                }
            }
        },
        Default::default()
    );
    
    source.set(1);
    source.set(2);
    source.set(3);
}

struct ReactionOptions<T> {
    fire_immediately: bool,
    equals: std::rc::Rc<dyn Fn(&T, &T) -> bool>,
    scheduler: std::rc::Rc<dyn Fn(Box<dyn Fn()>)>,
    once: bool,
}
impl<T: PartialEq + 'static> Default for ReactionOptions<T> {
    fn default() -> Self {
        Self {
            fire_immediately: false,
            equals: std::rc::Rc::new(T::eq),
            scheduler: std::rc::Rc::new(|f| f()),
            once: false,
        }
    }
}

fn reaction<T: PartialEq + Clone + 'static>(
    data_fn: impl Fn() -> T + 'static,
    effect_fn: impl Fn(&T, Option<&T>) + 'static,
    ReactionOptions {
        fire_immediately,
        equals,
        scheduler,
        once,
    }: ReactionOptions<T>,
) -> impl FnOnce() {
    let effect_fn = std::rc::Rc::new(effect_fn);
    
    let prev_value = std::rc::Rc::new(std::sync::Mutex::new(None));
    let version = std::rc::Rc::new(std::sync::Mutex::new(0));
    
    let tracked = Computed::new(move |_| {
        data_fn()
    });
    
    let mut dispose: std::rc::Rc<std::sync::Mutex<Option<Box<dyn FnOnce()>>>>
        = std::rc::Rc::new(std::sync::Mutex::new(None));
    let effect = Effect::new(move || {
        let current = tracked.get();
        if !fire_immediately && *version.lock().unwrap() == 0 {
            *prev_value.lock().unwrap() = Some(current.clone());
        }
        *version.lock().unwrap() += 1;
        if equals(&current, prev_value.lock().unwrap().as_ref().unwrap()) {
            return;
        }
        let old_value = prev_value.lock().unwrap().replace(current.clone());
        untracked({
            let scheduler = std::rc::Rc::clone(&scheduler);
            let effect_fn = effect_fn.clone();
            let version = version.clone();
            let dispose = dispose.clone();
            move || {
                scheduler(Box::new(move || {
                    effect_fn(&current, old_value.as_ref());
                    if once {
                        if (fire_immediately && *version.lock().unwrap() > 1)
                        || (!fire_immediately && *version.lock().unwrap() > 0) {
                            if let Some(dispose) = dispose.lock().unwrap().take() {
                                dispose();
                            }
                        }
                    }
                }));
            }
        });
    });
    dispose = std::rc::Rc::new(std::sync::Mutex::new(Some(Box::new(move || {
        effect.dispose();
    }))));
    
    move || {
        if let Some(dispose) = dispose.lock().unwrap().take() {
            dispose();
        }
    }
}

fn untracked<T>(callback: impl FnOnce() -> T) -> T {
    let current_sub = set_active_sub(None);
    let result = callback();
    set_active_sub(current_sub);
    result
}
