use crate::node::{EffectContext, Link, LinkInit, Node};
use crate::primitive::{Flags, Queue, Stack, Version, ThreadLocalUnsafeCellExt};

#[derive(Default)]
struct System {
    cycle: Version,
    batch_depth: usize,
    active_sub: Option<Node>,
    queued: Queue<Node<EffectContext>>,
}

thread_local! {
    static SYSTEM: std::cell::UnsafeCell<System> = std::cell::UnsafeCell::new(System::default());
}

#[inline(always)]
pub fn set_active_sub(sub: Option<Node>) -> Option<Node> {
    SYSTEM.with_borrow_mut(|sys| std::mem::replace(&mut sys.active_sub, sub))
}
#[inline]
pub fn get_active_sub() -> Option<Node> {
    SYSTEM.with_borrow(|sys| sys.active_sub)
}

#[inline]
pub fn get_batch_depth() -> usize {
    SYSTEM.with_borrow(|sys| sys.batch_depth)
}
#[inline]
pub fn start_batch() {
    SYSTEM.with_borrow_mut(|sys| sys.batch_depth += 1);
}
#[inline]
pub fn end_batch() {
    let is_zero = SYSTEM.with_borrow_mut(|sys| {
        sys.batch_depth -= 1;
        sys.batch_depth == 0
    });
    if is_zero {
        super::flush();
    }
}

#[inline]
pub(crate) fn increment_cycle() {
    SYSTEM.with_borrow_mut(|sys| sys.cycle.increment());
}
#[inline]
pub(crate) fn get_cycle() -> Version {
    SYSTEM.with_borrow(|sys| sys.cycle)
}

#[inline]
pub(crate) fn with_queued<T>(f: impl Fn(&mut Queue<Node<EffectContext>>) -> T) -> T {
    SYSTEM.with_borrow_mut(|sys| f(&mut sys.queued))
}

pub(crate) fn link(dep: Node, sub: Node, version: Version) {
    let prev_dep = sub.deps_tail();
    if let Some(prev_dep) = prev_dep
        && prev_dep.dep() == dep
    {
        return;
    }

    let next_dep = match prev_dep {
        Some(it) => it.next_dep(),
        None => sub.deps(),
    };
    if let Some(next_dep) = next_dep
        && next_dep.dep() == dep
    {
        next_dep.set_version(version);
        sub.set_deps_tail(Some(next_dep));
        return;
    }

    let prev_sub = dep.subs_tail();
    if let Some(prev_sub) = prev_sub
        && prev_sub.version() == version
        && prev_sub.sub() == sub
    {
        return;
    }

    let new_link = Link::new(LinkInit {
        version,
        dep,
        sub,
        prev_dep,
        next_dep,
        prev_sub,
        next_sub: None,
    });
    dep.set_subs_tail(Some(new_link));
    sub.set_deps_tail(Some(new_link));

    if let Some(next_dep) = next_dep {
        next_dep.set_prev_dep(Some(new_link));
    }

    if let Some(prev_dep) = prev_dep {
        prev_dep.set_next_dep(Some(new_link));
    } else {
        sub.set_deps(Some(new_link));
    }

    if let Some(prev_sub) = prev_sub {
        prev_sub.set_next_sub(Some(new_link));
    } else {
        dep.set_subs(Some(new_link));
    }
}

pub(crate) fn unlink(link: Link, sub: Node) -> Option<Link> {
    let (dep, prev_dep, next_dep, next_sub, prev_sub) = (
        link.dep(),
        link.prev_dep(),
        link.next_dep(),
        link.next_sub(),
        link.prev_sub(),
    );

    if let Some(next_dep) = next_dep {
        next_dep.set_prev_dep(prev_dep);
    } else {
        sub.set_deps_tail(prev_dep);
    }

    if let Some(prev_dep) = prev_dep {
        prev_dep.set_next_dep(next_dep);
    } else {
        sub.set_deps(next_dep);
    }

    if let Some(next_sub) = next_sub {
        next_sub.set_prev_sub(prev_sub);
    } else {
        dep.set_subs_tail(prev_sub);
    }

    if let Some(prev_sub) = prev_sub {
        prev_sub.set_next_sub(next_sub);
    } else if {
        dep.set_subs(next_sub);
        next_sub.is_none()
    } {
        super::unwatched(dep);
    }

    next_dep
}

pub(crate) fn propagate(mut link: Link) {
    let mut next = link.next_sub();
    let mut stack = Stack::<Option<Link>>::new();

    'top: loop {
        let sub = link.sub();
        let mut flags = sub.flags();

        if (flags & (Flags::RECURSED_CHECK | Flags::RECURSED | Flags::DIRTY | Flags::PENDING))
            .is_zero()
        {
            sub.set_flags(flags | Flags::PENDING);
        } else if (flags & (Flags::RECURSED_CHECK | Flags::RECURSED)).is_zero() {
            flags = Flags::NONE;
        } else if (flags & Flags::RECURSED_CHECK).is_zero() {
            sub.set_flags((flags & !Flags::RECURSED) | Flags::PENDING);
        } else if (flags & (Flags::DIRTY | Flags::PENDING)).is_zero() && is_valid_link(link, sub) {
            sub.set_flags(flags | (Flags::RECURSED | Flags::PENDING));
            flags &= Flags::MUTABLE;
        } else {
            flags = Flags::NONE;
        }

        if (flags & Flags::WATCHING).is_nonzero() {
            super::notify(
                sub.try_into()
                    .expect("BUG: `sub` is not effect in `propagate`"),
            );
        }

        if (flags & Flags::MUTABLE).is_nonzero() {
            if let Some(sub_subs) = sub.subs() {
                link = sub_subs;
                if let Some(next_sub) = sub_subs.next_sub() {
                    stack.push(next);
                    next = Some(next_sub);
                }
                continue;
            }
        }

        if let Some(some_next) = next {
            link = some_next;
            next = link.next_sub();
            continue;
        }

        while let Some(popped_link_opt) = stack.pop() {
            if let Some(popped_link) = popped_link_opt {
                link = popped_link;
                next = link.next_sub();
                continue 'top;
            }
        }

        break;
    }
}

pub(crate) fn check_dirty(mut link: Link, mut sub: Node) -> bool {
    let mut stack = Stack::<Link>::new();
    let mut check_depth = 0;
    let mut dirty = false;

    'top: loop {
        let dep = link.dep();
        let flags = dep.flags();

        if (sub.flags() & Flags::DIRTY).is_nonzero() {
            dirty = true;
        } else if (flags & (Flags::MUTABLE | Flags::DIRTY)) == (Flags::MUTABLE | Flags::DIRTY) {
            if super::update(dep) {
                let subs = dep
                    .subs()
                    .expect("BUG: no `dep.subs` in `MUTABLE | DIRTY` path");
                if subs.next_sub().is_some() {
                    shallow_propagate(subs);
                }
                dirty = true;
            }
        } else if (flags & (Flags::MUTABLE | Flags::PENDING)) == (Flags::MUTABLE | Flags::PENDING) {
            if link.next_sub().is_some() || link.prev_sub().is_some() {
                stack.push(link);
            }
            link = dep
                .deps()
                .expect("BUG: no `dep.deps` in `MUTABLE | PENDING` path");
            sub = dep;
            check_depth += 1;
            continue;
        }

        if !dirty {
            if let Some(next_dep) = link.next_dep() {
                link = next_dep;
                continue;
            }
        }

        while check_depth > 0 {
            let first_sub = sub.subs().expect("BUG: no `sub.subs` while check_deps > 0");
            let has_multiple_subs = first_sub.next_sub().is_some();

            if has_multiple_subs {
                link = stack
                    .pop()
                    .expect("BUG: no `stack` item in `has_multiple_subs` path");
            } else {
                link = first_sub;
            }

            if dirty {
                if super::update(sub) {
                    if has_multiple_subs {
                        shallow_propagate(first_sub);
                    }
                    sub = link.sub();

                    check_depth -= 1;
                    continue;
                }
                dirty = false;
            } else {
                sub.update_flags(|f| *f &= !Flags::PENDING);
            }

            sub = link.sub();
            if let Some(next_dep) = link.next_dep() {
                link = next_dep;

                check_depth -= 1;
                continue 'top;
            }

            check_depth -= 1;
            continue;
        }

        return dirty;
    }
}

#[inline]
pub(crate) fn shallow_propagate(mut link: Link) {
    loop {
        let sub = link.sub();
        let flags = sub.flags();

        if (flags & (Flags::PENDING | Flags::DIRTY)) == Flags::PENDING {
            sub.update_flags(|f| *f |= Flags::DIRTY);
            if (flags & (Flags::WATCHING | Flags::RECURSED_CHECK)) == Flags::WATCHING {
                super::notify(
                    sub.try_into()
                        .expect("BUG: `sub` is not effect in `shallow_propagate`"),
                );
            }
        }

        if let Some(next_sub) = link.next_sub() {
            link = next_sub;
        } else {
            break;
        }
    }
}

#[inline]
pub(crate) fn is_valid_link(check_link: Link, sub: Node) -> bool {
    let mut link = sub.deps_tail();
    while let Some(some_link) = link {
        if some_link == check_link {
            return true;
        }
        link = some_link.prev_dep();
    }
    false
}
