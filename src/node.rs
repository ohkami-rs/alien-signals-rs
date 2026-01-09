use crate::primitive::{Flags, SmallAny, Version, ChunkedArena, ThreadLocalUnsafeCellExt};

pub enum NodeContext {
    Signal(SignalContext),
    Computed(ComputedContext),
    Effect(EffectContext),
    None,
}

#[derive(Clone, Copy)]
pub(crate) enum NodeContextKind {
    Signal,
    Computed,
    Effect,
    None,
}
impl NodeContext {
    pub(crate) fn kind(&self) -> NodeContextKind {
        match self {
            NodeContext::Signal(_) => NodeContextKind::Signal,
            NodeContext::Computed(_) => NodeContextKind::Computed,
            NodeContext::Effect(_) => NodeContextKind::Effect,
            NodeContext::None => NodeContextKind::None,
        }
    }
}

pub struct SignalContext {
    pub(crate) current_value: SmallAny,
    pub(crate) pending_value: SmallAny,
    pub(crate) eq: Box<dyn Fn(&SmallAny, &SmallAny) -> bool>,
}

pub struct ComputedContext {
    pub(crate) value: Option<SmallAny>,
    pub(crate) get: Box<dyn Fn(Option<&SmallAny>) -> SmallAny>,
    pub(crate) eq: Box<dyn Fn(&SmallAny, &SmallAny) -> bool>,
}

pub struct EffectContext {
    pub(crate) run: Box<dyn Fn()>,
}

struct LinkFields {
    version: Version,
    dep: Node,
    sub: Node,
    prev_sub: Option<Link>,
    next_sub: Option<Link>,
    prev_dep: Option<Link>,
    next_dep: Option<Link>,
}
const _: () = assert!(std::mem::size_of::<LinkFields>() == 7 * std::mem::size_of::<usize>());

struct NodeFields {
    flags: Flags,
    deps: Option<Link>,
    deps_tail: Option<Link>,
    subs: Option<Link>,
    subs_tail: Option<Link>,
    context: NodeContext,
}

#[derive(Default)]
struct Arena {
    link: ChunkedArena<LinkFields, 1024>,
    node: ChunkedArena<NodeFields, 1024>,
}

thread_local! {
    static ARENA: std::cell::UnsafeCell<Arena> = std::cell::UnsafeCell::new(Arena::default());
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Link(std::ptr::NonNull<LinkFields>);
const _: () = assert!(std::mem::size_of::<Link>() == std::mem::size_of::<usize>());
const _: () = assert!(std::mem::size_of::<Option<Link>>() == std::mem::size_of::<usize>());

pub struct Node<C = NodeContext>(std::ptr::NonNull<NodeFields>, std::marker::PhantomData<C>);
const _: () = assert!(std::mem::size_of::<Node>() == std::mem::size_of::<usize>());
const _: () = assert!(std::mem::size_of::<Option<Node>>() == std::mem::size_of::<usize>());

/// not requiring `C: Clone`
impl<C> Clone for Node<C> {
    fn clone(&self) -> Self {
        Node(self.0, std::marker::PhantomData)
    }
}
impl<C> Copy for Node<C> {}
/// not requiring `C: PartialEq`
impl<C> PartialEq for Node<C> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<C> Eq for Node<C> {}

pub(crate) struct LinkInit {
    pub(crate) version: Version,
    pub(crate) dep: Node,
    pub(crate) sub: Node,
    pub(crate) prev_sub: Option<Link>,
    pub(crate) next_sub: Option<Link>,
    pub(crate) prev_dep: Option<Link>,
    pub(crate) next_dep: Option<Link>,
}

impl Link {
    pub(crate) fn new(init: LinkInit) -> Self {
        ARENA.with_borrow_mut(|arena| {
            let ptr = arena.link.alloc(LinkFields {
                version: init.version,
                dep: init.dep,
                sub: init.sub,
                prev_sub: init.prev_sub,
                next_sub: init.next_sub,
                prev_dep: init.prev_dep,
                next_dep: init.next_dep,
            });
            Link(ptr)
        })
    }

    pub(crate) fn version(&self) -> Version {
        unsafe { &*self.0.as_ptr() }.version
    }
    pub(crate) fn set_version(&self, version: Version) {
        unsafe { &mut *self.0.as_ptr() }.version = version;
    }

    #[inline]
    pub(crate) fn dep(&self) -> Node {
        unsafe { &*self.0.as_ptr() }.dep
    }

    #[inline]
    pub(crate) fn sub(&self) -> Node {
        unsafe { &*self.0.as_ptr() }.sub
    }

    #[inline]
    pub(crate) fn prev_sub(&self) -> Option<Link> {
        unsafe { &*self.0.as_ptr() }.prev_sub
    }
    #[inline]
    pub(crate) fn set_prev_sub(&self, link: Option<Link>) {
        unsafe { &mut *self.0.as_ptr() }.prev_sub = link;
    }

    #[inline]
    pub(crate) fn next_sub(&self) -> Option<Link> {
        unsafe { &*self.0.as_ptr() }.next_sub
    }
    #[inline]
    pub(crate) fn set_next_sub(&self, link: Option<Link>) {
        unsafe { &mut *self.0.as_ptr() }.next_sub = link;
    }

    #[inline]
    pub(crate) fn prev_dep(&self) -> Option<Link> {
        unsafe { &*self.0.as_ptr() }.prev_dep
    }
    #[inline]
    pub(crate) fn set_prev_dep(&self, link: Option<Link>) {
        unsafe { &mut *self.0.as_ptr() }.prev_dep = link;
    }

    #[inline]
    pub(crate) fn next_dep(&self) -> Option<Link> {
        unsafe { &*self.0.as_ptr() }.next_dep
    }
    #[inline]
    pub(crate) fn set_next_dep(&self, link: Option<Link>) {
        unsafe { &mut *self.0.as_ptr() }.next_dep = link;
    }
}

impl<C> Node<C> {
    #[inline(always)]
    pub fn flags(&self) -> Flags {
        unsafe { &*self.0.as_ptr() }.flags
    }
    #[inline(always)]
    pub fn set_flags(&self, flags: Flags) {
        unsafe { &mut *self.0.as_ptr() }.flags = flags;
    }
    /// ```rust,no_run
    /// alien_signals::get_active_sub().unwrap().update_flags(
    ///     |f| *f &= !alien_signals::Flags::RECURSED_CHECK
    /// );
    /// ```
    #[inline(always)]
    pub fn update_flags(&self, f: impl FnOnce(&mut Flags)) {
        f(&mut unsafe { &mut *self.0.as_ptr() }.flags);
    }

    #[inline]
    pub(crate) fn deps(&self) -> Option<Link> {
        unsafe { &*self.0.as_ptr() }.deps
    }
    #[inline]
    pub(crate) fn set_deps(&self, link: Option<Link>) {
        unsafe { &mut *self.0.as_ptr() }.deps = link;
    }

    #[inline]
    pub(crate) fn deps_tail(&self) -> Option<Link> {
        unsafe { &*self.0.as_ptr() }.deps_tail
    }
    #[inline]
    pub(crate) fn set_deps_tail(&self, link: Option<Link>) {
        unsafe { &mut *self.0.as_ptr() }.deps_tail = link;
    }

    #[inline]
    pub(crate) fn subs(&self) -> Option<Link> {
        unsafe { &*self.0.as_ptr() }.subs
    }
    #[inline]
    pub(crate) fn set_subs(&self, link: Option<Link>) {
        unsafe { &mut *self.0.as_ptr() }.subs = link;
    }

    #[inline]
    pub(crate) fn subs_tail(&self) -> Option<Link> {
        unsafe { &*self.0.as_ptr() }.subs_tail
    }
    #[inline]
    pub(crate) fn set_subs_tail(&self, link: Option<Link>) {
        unsafe { &mut *self.0.as_ptr() }.subs_tail = link;
    }
}

impl Node<NodeContext> {
    pub(crate) fn new(flags: Flags) -> Self {
        ARENA.with_borrow_mut(|arena| {
            let ptr = arena.node.alloc(NodeFields {
                flags,
                deps: None,
                deps_tail: None,
                subs: None,
                subs_tail: None,
                context: NodeContext::None,
            });
            Node(ptr, std::marker::PhantomData)
        })
    }

    pub(crate) fn kind(&self) -> NodeContextKind {
        unsafe { &*self.0.as_ptr() }.context.kind()
    }
}

impl Node<SignalContext> {
    pub(crate) fn new<T: PartialEq + 'static>(init: T) -> Self {
        Self::new_with_eq_fn(init, T::eq)
    }
    pub(crate) fn new_with_eq_fn<T: 'static>(
        init: T,
        eq_fn: impl Fn(&T, &T) -> bool + 'static,
    ) -> Self {
        ARENA.with_borrow_mut(|arena| {
            let init = SmallAny::new(init);
            let context = NodeContext::Signal(SignalContext {
                current_value: init.clone(),
                pending_value: init,
                eq: Box::new(move |a, b| {
                    let a = a.downcast_ref::<T>().unwrap_or_else(|| {
                        panic!("BUG: signal type is not {}", std::any::type_name::<T>())
                    });
                    let b = b.downcast_ref::<T>().unwrap_or_else(|| {
                        panic!("BUG: signal type is not {}", std::any::type_name::<T>())
                    });
                    eq_fn(a, b)
                }),
            });
            let ptr = arena.node.alloc(NodeFields {
                flags: Flags::MUTABLE,
                deps: None,
                deps_tail: None,
                subs: None,
                subs_tail: None,
                context,
            });
            Node(ptr, std::marker::PhantomData)
        })
    }

    pub(crate) fn context(&self) -> &SignalContext {
        match unsafe { &*self.0.as_ptr() }.context {
            NodeContext::Signal(ref ctx) => ctx,
            _ => panic!("BUG: Node is not a Signal"),
        }
    }
    pub(crate) fn update_context(&self, f: impl FnOnce(&mut SignalContext)) {
        match unsafe { &mut *self.0.as_ptr() }.context {
            NodeContext::Signal(ref mut ctx) => f(ctx),
            _ => panic!("BUG: Node is not a Signal"),
        }
    }
}
impl Node<ComputedContext> {
    pub(crate) fn new<T: PartialEq + 'static>(getter: impl Fn(Option<&T>) -> T + 'static) -> Self {
        Self::new_with_eq_fn(getter, T::eq)
    }
    pub(crate) fn new_with_eq_fn<T: 'static>(
        getter: impl Fn(Option<&T>) -> T + 'static,
        eq_fn: impl Fn(&T, &T) -> bool + 'static,
    ) -> Self {
        ARENA.with_borrow_mut(|arena| {
            let context = NodeContext::Computed(ComputedContext {
                value: None,
                get: Box::new(move |prev_any| {
                    let prev_t = prev_any.map(|any| {
                        any.downcast_ref::<T>().unwrap_or_else(|| {
                            panic!("BUG: computed type is not {}", std::any::type_name::<T>())
                        })
                    });
                    let new_t = getter(prev_t);
                    SmallAny::new(new_t)
                }),
                eq: Box::new(move |a, b| {
                    let a = a.downcast_ref::<T>().unwrap_or_else(|| {
                        panic!("BUG: computed type is not {}", std::any::type_name::<T>())
                    });
                    let b = b.downcast_ref::<T>().unwrap_or_else(|| {
                        panic!("BUG: computed type is not {}", std::any::type_name::<T>())
                    });
                    eq_fn(a, b)
                }),
            });
            let ptr = arena.node.alloc(NodeFields {
                flags: Flags::NONE,
                deps: None,
                deps_tail: None,
                subs: None,
                subs_tail: None,
                context,
            });
            Node(ptr, std::marker::PhantomData)
        })
    }

    pub(crate) fn context(&self) -> &ComputedContext {
        match unsafe { &*self.0.as_ptr() }.context {
            NodeContext::Computed(ref ctx) => ctx,
            _ => panic!("BUG: Node is not a Computed"),
        }
    }
    pub(crate) fn update_context(&self, f: impl FnOnce(&mut ComputedContext)) {
        match unsafe { &mut *self.0.as_ptr() }.context {
            NodeContext::Computed(ref mut ctx) => f(ctx),
            _ => panic!("BUG: Node is not a Computed"),
        }
    }
}

impl Node<EffectContext> {
    pub(crate) fn new(f: impl Fn() + 'static) -> Self {
        ARENA.with_borrow_mut(|arena| {
            let context = NodeContext::Effect(EffectContext {
                run: Box::new(f),
            });
            let ptr = arena.node.alloc(NodeFields {
                flags: Flags::WATCHING | Flags::RECURSED_CHECK,
                deps: None,
                deps_tail: None,
                subs: None,
                subs_tail: None,
                context,
            });
            Node(ptr, std::marker::PhantomData)
        })
    }

    pub(crate) fn context(&self) -> &EffectContext {
        match unsafe { &*self.0.as_ptr() }.context {
            NodeContext::Effect(ref ctx) => ctx,
            _ => panic!("BUG: Node is not an Effect"),
        }
    }
}

impl From<Node<SignalContext>> for Node<NodeContext> {
    fn from(node: Node<SignalContext>) -> Self {
        Node(node.0, std::marker::PhantomData)
    }
}
impl From<Node<ComputedContext>> for Node<NodeContext> {
    fn from(node: Node<ComputedContext>) -> Self {
        Node(node.0, std::marker::PhantomData)
    }
}
impl From<Node<EffectContext>> for Node<NodeContext> {
    fn from(node: Node<EffectContext>) -> Self {
        Node(node.0, std::marker::PhantomData)
    }
}

impl TryFrom<Node<NodeContext>> for Node<SignalContext> {
    type Error = ();
    fn try_from(node: Node<NodeContext>) -> Result<Self, Self::Error> {
        match unsafe { &*node.0.as_ptr() }.context.kind() {
            NodeContextKind::Signal => Ok(Node(node.0, std::marker::PhantomData)),
            _ => Err(()),
        }
    }
}
impl TryFrom<Node<NodeContext>> for Node<ComputedContext> {
    type Error = ();
    fn try_from(node: Node<NodeContext>) -> Result<Self, Self::Error> {
        match unsafe { &*node.0.as_ptr() }.context.kind() {
            NodeContextKind::Computed => Ok(Node(node.0, std::marker::PhantomData)),
            _ => Err(()),
        }
    }
}
impl TryFrom<Node<NodeContext>> for Node<EffectContext> {
    type Error = ();
    fn try_from(node: Node<NodeContext>) -> Result<Self, Self::Error> {
        match unsafe { &*node.0.as_ptr() }.context.kind() {
            NodeContextKind::Effect => Ok(Node(node.0, std::marker::PhantomData)),
            _ => Err(()),
        }
    }
}
