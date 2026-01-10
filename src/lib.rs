// for doctest inclusion
#![cfg_attr(all(doc, not(docsrs)), doc = include_str!("../../README.md"))]

mod node;
mod primitive;
mod system;

use node::{ComputedContext, EffectContext, Node, NodeContext, NodeContextKind, SignalContext};
use primitive::{SmallAny, Version};

pub use primitive::Flags;
pub use system::{end_batch, get_active_sub, get_batch_depth, set_active_sub, start_batch};

#[inline]
fn update(signal_or_computed: Node) -> bool {
    match signal_or_computed.kind() {
        NodeContextKind::Signal => update_signal(signal_or_computed.try_into().unwrap()),
        NodeContextKind::Computed => update_computed(signal_or_computed.try_into().unwrap()),
        _ => panic!("BUG: `update` target is neither signal nor computed"),
    }
}

/// enqueue chanined effects in REVERSED order
fn notify(mut effect: Node<EffectContext>) {
    // to avoid other allocation than `queued`,
    // just push directly to `ququed` and finally reverse newly-pushed part
    let chain_head_index = system::with_queued(|q| q.length());
    loop {
        effect.update_flags(|f| *f &= !Flags::WATCHING);
        system::with_queued(|q| q.push(effect));
        match effect.subs().map(|s| s.sub()) {
            Some(subs_sub) if (subs_sub.flags() & Flags::WATCHING).is_nonzero() => {
                effect = subs_sub
                    .try_into()
                    .expect("BUG: `subs.sub` of an effect is not effect");
            }
            _ => break,
        }
    }
    system::with_queued(|q| q.as_slice_mut()[chain_head_index..].reverse());
}

fn unwatched(node: Node) {
    if (node.flags() & Flags::MUTABLE).is_zero() {
        effect_scope_oper(node);
    } else if node.deps_tail().is_some() {
        node.set_deps_tail(None);
        node.set_flags(Flags::MUTABLE | Flags::DIRTY);
        purge_deps(node);
    }
}

pub struct Signal<T>(Node<SignalContext>, std::marker::PhantomData<T>);
// not requiring `T: Clone`
impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Signal<T> {}
impl<T: Clone + 'static> Signal<T> {
    pub fn new(init: T) -> Self
    where
        T: PartialEq,
    {
        let node = Node::<SignalContext>::new(init);
        Self(node, std::marker::PhantomData)
    }
    pub fn new_with_eq_fn(init: T, eq_fn: impl Fn(&T, &T) -> bool + 'static) -> Self {
        let node = Node::<SignalContext>::new_with_eq_fn(init, eq_fn);
        Self(node, std::marker::PhantomData)
    }

    #[inline]
    pub fn get(&self) -> T {
        get_signal_oper(self.0)
    }

    #[inline]
    pub fn set(&self, value: T) {
        set_signal_oper(self.0, value);
    }

    /// set with current value
    pub fn set_with(&self, f: impl FnOnce(&T) -> T) {
        set_with_signal_oper(self.0, f);
    }

    #[inline]
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        update_signal_oper(self.0, f);
    }
}

pub struct Computed<T>(Node<ComputedContext>, std::marker::PhantomData<T>);
// not requiring `T: Clone`
impl<T> Clone for Computed<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Computed<T> {}
impl<T: Clone + 'static> Computed<T> {
    pub fn new(getter: impl Fn(Option<&T>) -> T + 'static) -> Self
    where
        T: PartialEq,
    {
        let node = Node::<ComputedContext>::new(getter);
        Self(node, std::marker::PhantomData)
    }
    pub fn new_with_eq(
        getter: impl Fn(Option<&T>) -> T + 'static,
        eq_fn: impl Fn(&T, &T) -> bool + 'static,
    ) -> Self {
        let node = Node::<ComputedContext>::new_with_eq_fn(getter, eq_fn);
        Self(node, std::marker::PhantomData)
    }

    #[inline]
    pub fn get(&self) -> T {
        computed_oper(self.0)
    }
}

pub struct Effect {
    dispose: Box<dyn FnOnce()>,
}
impl Effect {
    pub fn new(f: impl Fn() + 'static) -> Self {
        let e = Node::<EffectContext>::new(f);
        let prev_sub = system::set_active_sub(Some(e.into()));
        if let Some(prev_sub) = prev_sub {
            system::link(e.into(), prev_sub, Version::new());
        }
        (e.context().run)();
        system::set_active_sub(prev_sub);
        e.update_flags(|f| *f &= !Flags::RECURSED_CHECK);
        Self {
            dispose: Box::new(move || effect_oper(e)),
        }
    }

    pub fn dispose(self) {
        (self.dispose)();
    }
}

pub struct EffectScope {
    dispose: Box<dyn FnOnce()>,
}
impl EffectScope {
    pub fn new(f: impl FnOnce() + 'static) -> Self {
        let e = Node::<NodeContext>::new(Flags::NONE);
        let prev_sub = system::set_active_sub(Some(e));
        if let Some(prev_sub) = prev_sub {
            system::link(e, prev_sub, Version::new());
        }
        f();
        system::set_active_sub(prev_sub);
        Self {
            dispose: Box::new(move || effect_scope_oper(e)),
        }
    }

    pub fn dispose(self) {
        (self.dispose)();
    }
}

pub fn trigger(f: impl FnOnce() + 'static) {
    let sub = Node::<NodeContext>::new(Flags::WATCHING);
    let prev_sub = system::set_active_sub(Some(sub));
    f();
    system::set_active_sub(prev_sub);
    let mut link = sub.deps();
    while let Some(some_link) = link {
        let dep = some_link.dep();
        link = system::unlink(some_link, sub);
        if let Some(subs) = dep.subs() {
            sub.set_flags(Flags::NONE);
            system::propagate(subs);
            system::shallow_propagate(subs);
        }
    }
    if system::get_batch_depth() == 0 {
        flush();
    }
}

#[inline]
fn update_computed(c: Node<ComputedContext>) -> bool {
    system::increment_cycle();
    c.set_deps_tail(None);
    c.set_flags(Flags::MUTABLE | Flags::RECURSED_CHECK);
    let prev_sub = system::set_active_sub(Some(c.into()));
    let ComputedContext {
        value: old_value,
        get,
        eq,
    } = c.context();

    let new_value = get(old_value.as_ref());

    let is_changed = match old_value {
        None => true, // initial update
        Some(old_value) => !eq(old_value, &new_value),
    };

    c.update_context(|ctx| ctx.value = Some(new_value));
    system::set_active_sub(prev_sub);
    c.update_flags(|f| *f &= !Flags::RECURSED_CHECK);
    purge_deps(c.into());

    is_changed
}

#[inline]
fn update_signal(s: Node<SignalContext>) -> bool {
    s.set_flags(Flags::MUTABLE);
    let SignalContext {
        current_value,
        pending_value,
        eq,
    } = s.context();
    let is_changed = !eq(current_value, pending_value);
    s.update_context(|c| c.current_value = c.pending_value.clone());
    is_changed
}

fn run(e: Node<EffectContext>) {
    let flags = e.flags();
    if (flags & Flags::DIRTY).is_nonzero()
        || ((flags & Flags::PENDING).is_nonzero()
            && system::check_dirty(
                e.deps().expect("BUG: effect node has no `deps` in `run`"),
                e.into(),
            ))
    {
        system::increment_cycle();
        e.set_deps_tail(None);
        e.set_flags(Flags::WATCHING | Flags::RECURSED_CHECK);
        let prev_sub = system::set_active_sub(Some(e.into()));
        let EffectContext { run } = e.context();

        run();

        system::set_active_sub(prev_sub);
        e.update_flags(|f| *f &= !Flags::RECURSED_CHECK);
        purge_deps(e.into());
    } else {
        e.set_flags(Flags::WATCHING);
    }
}

fn flush() {
    while let Some(effect) = system::with_queued(|q| q.pop()) {
        run(effect);
    }
}

fn computed_oper<T: Clone + 'static>(this: Node<ComputedContext>) -> T {
    let flags = this.flags();
    if (flags & Flags::DIRTY).is_nonzero()
        || ((flags & Flags::PENDING).is_nonzero() && {
            if system::check_dirty(
                this.deps().expect("BUG: `deps` is None in `computed_oper`"),
                this.into(),
            ) {
                true
            } else {
                this.set_flags(flags & !Flags::PENDING);
                false
            }
        })
    {
        if update_computed(this) {
            if let Some(subs) = this.subs() {
                system::shallow_propagate(subs);
            }
        }
    } else if flags.is_zero() {
        this.set_flags(Flags::MUTABLE | Flags::RECURSED_CHECK);
        let prev_sub = system::set_active_sub(Some(this.into()));
        let ComputedContext { value, get, eq: _ } = this.context();

        let new_value = get(value.as_ref());

        this.update_context(|ctx| ctx.value = Some(new_value));
        system::set_active_sub(prev_sub);
        this.update_flags(|f| *f &= !Flags::RECURSED_CHECK);
    }

    if let Some(sub) = system::get_active_sub() {
        system::link(this.into(), sub, system::get_cycle());
    }

    this.context()
        .value
        .as_ref()
        .expect("BUG: computed value is None")
        .downcast_ref::<T>()
        .unwrap_or_else(|| panic!("BUG: computed type is not {}", std::any::type_name::<T>()))
        .clone()
}

fn _set_signal_oper_core<T: 'static>(context: &SignalContext, this: Node<SignalContext>, value: T) {
    let value = SmallAny::new(value);
    let is_changed = !(context.eq)(&context.pending_value, &value);
    this.update_context(|c| c.pending_value = value);
    if is_changed {
        this.set_flags(Flags::MUTABLE | Flags::DIRTY);
        if let Some(subs) = this.subs() {
            system::propagate(subs);
            if system::get_batch_depth() == 0 {
                flush();
            }
        }
    }
}

fn set_signal_oper<T: 'static>(this: Node<SignalContext>, value: T) {
    _set_signal_oper_core(this.context(), this, value);
}

fn set_with_signal_oper<T: 'static>(this: Node<SignalContext>, set_with: impl FnOnce(&T) -> T) {
    let context = this.context();
    let current_value = context
        .current_value
        .downcast_ref::<T>()
        .unwrap_or_else(|| {
            panic!(
                "BUG: signal node is not of type {}",
                std::any::type_name::<T>()
            )
        });
    let value = set_with(current_value);
    _set_signal_oper_core(context, this, value);
}

fn update_signal_oper<T: Clone + 'static>(this: Node<SignalContext>, update: impl FnOnce(&mut T)) {
    let context = this.context();
    let mut current_value = context
        .current_value
        .downcast_ref::<T>()
        .unwrap_or_else(|| {
            panic!(
                "BUG: signal node is not of type {}",
                std::any::type_name::<T>()
            )
        })
        .clone();
    update(&mut current_value);
    _set_signal_oper_core(context, this, current_value);
}

fn get_signal_oper<T: Clone + 'static>(this: Node<SignalContext>) -> T {
    if (this.flags() & Flags::DIRTY).is_nonzero() {
        if update_signal(this) {
            if let Some(subs) = this.subs() {
                system::shallow_propagate(subs);
            }
        }
    }

    let mut sub = system::get_active_sub();
    while let Some(some_sub) = sub {
        if (some_sub.flags() & (Flags::MUTABLE | Flags::WATCHING)).is_nonzero() {
            system::link(this.into(), some_sub, system::get_cycle());
            break;
        }
        sub = some_sub.subs().map(|it| it.sub());
    }

    this.context()
        .current_value
        .downcast_ref::<T>()
        .unwrap_or_else(|| {
            panic!(
                "BUG: signal node is not of type {}",
                std::any::type_name::<T>()
            )
        })
        .clone()
}

fn effect_oper(this: Node<EffectContext>) {
    effect_scope_oper(this.into());
}

fn effect_scope_oper(this: Node) {
    this.set_deps_tail(None);
    this.set_flags(Flags::NONE);
    purge_deps(this);
    if let Some(sub) = this.subs() {
        system::unlink(sub, sub.sub());
    }
}

fn purge_deps(sub: Node) {
    let mut dep = match sub.deps_tail() {
        Some(deps_tail) => deps_tail.next_dep(),
        None => sub.deps(),
    };
    while let Some(some_dep) = dep {
        dep = system::unlink(some_dep, sub);
    }
}
