use crate::primitive::{Flags, LinkVersion, SmallAny};

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

#[derive(Clone)]
pub struct SignalContext {
    pub(crate) current_value: SmallAny,
    pub(crate) pending_value: SmallAny,
    pub(crate) eq: std::rc::Rc<dyn Fn(&SmallAny, &SmallAny) -> bool>,
}

#[derive(Clone)]
pub struct ComputedContext {
    pub(crate) value: Option<SmallAny>,
    pub(crate) get: std::rc::Rc<dyn Fn(Option<&SmallAny>) -> SmallAny>,
    pub(crate) eq: std::rc::Rc<dyn Fn(&SmallAny, &SmallAny) -> bool>,
}

#[derive(Clone)]
pub struct EffectContext {
    pub(crate) run: std::rc::Rc<dyn Fn()>,
}

/// SoA representation of a series of links
#[derive(Default)]
struct LinkArena {
    version: Vec<LinkVersion>,
    dep: Vec<Node>,
    sub: Vec<Node>,
    prev_sub: Vec<Option<Link>>,
    next_sub: Vec<Option<Link>>,
    prev_dep: Vec<Option<Link>>,
    next_dep: Vec<Option<Link>>,
}

/// SoA representation of a series of nodes
#[derive(Default)]
struct NodeArena {
    flags: Vec<Flags>,
    deps: Vec<Option<Link>>,
    deps_tail: Vec<Option<Link>>,
    subs: Vec<Option<Link>>,
    subs_tail: Vec<Option<Link>>,
    context_indices: Vec<(NodeContextKind, usize)>,
    /* */
    signals: Vec<SignalContext>,
    computeds: Vec<ComputedContext>,
    effects: Vec<EffectContext>,
}

#[derive(Default)]
struct Arena {
    link: LinkArena,
    node: NodeArena,
}

thread_local! {
    static ARENA: std::cell::RefCell<Arena> = std::cell::RefCell::new(Arena::default());
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Link(usize);

pub struct Node<C = NodeContext>(usize, std::marker::PhantomData<C>);
// not requiring `C: Clone`
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
    pub(crate) version: LinkVersion,
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
            let index = arena.link.version.len();
            arena.link.version.push(init.version);
            arena.link.dep.push(init.dep);
            arena.link.sub.push(init.sub);
            arena.link.prev_sub.push(init.prev_sub);
            arena.link.next_sub.push(init.next_sub);
            arena.link.prev_dep.push(init.prev_dep);
            arena.link.next_dep.push(init.next_dep);
            Link(index)
        })
    }
    
    pub(crate) fn version(&self) -> LinkVersion {
        ARENA.with_borrow(|arena| arena.link.version[self.0].clone())
    }
    pub(crate) fn set_version(&self, version: LinkVersion) {
        ARENA.with_borrow_mut(|arena| arena.link.version[self.0] = version);
    }
    
    pub(crate) fn dep(&self) -> Node {
        ARENA.with_borrow(|arena| Node(arena.link.dep[self.0].0, std::marker::PhantomData))
    }
    
    pub(crate) fn sub(&self) -> Node {
        ARENA.with_borrow(|arena| Node(arena.link.sub[self.0].0, std::marker::PhantomData))
    }
    
    pub(crate) fn prev_sub(&self) -> Option<Link> {
        ARENA.with_borrow(|arena| arena.link.prev_sub[self.0])
    }
    pub(crate) fn set_prev_sub(&self, link: Option<Link>) {
        ARENA.with_borrow_mut(|arena| arena.link.prev_sub[self.0] = link);
    }
    
    pub(crate) fn next_sub(&self) -> Option<Link> {
        ARENA.with_borrow(|arena| arena.link.next_sub[self.0])
    }
    pub(crate) fn set_next_sub(&self, link: Option<Link>) {
        ARENA.with_borrow_mut(|arena| arena.link.next_sub[self.0] = link);
    }
    
    pub(crate) fn prev_dep(&self) -> Option<Link> {
        ARENA.with_borrow(|arena| arena.link.prev_dep[self.0])
    }
    pub(crate) fn set_prev_dep(&self, link: Option<Link>) {
        ARENA.with_borrow_mut(|arena| arena.link.prev_dep[self.0] = link);
    }
    
    pub(crate) fn next_dep(&self) -> Option<Link> {
        ARENA.with_borrow(|arena| arena.link.next_dep[self.0])
    }
    pub(crate) fn set_next_dep(&self, link: Option<Link>) {
        ARENA.with_borrow_mut(|arena| arena.link.next_dep[self.0] = link);
    }
}

impl<C> Node<C> {
    pub fn flags(&self) -> Flags {
        ARENA.with_borrow(|arena| arena.node.flags[self.0])
    }
    pub fn set_flags(&self, flags: Flags) {
        ARENA.with_borrow_mut(|arena| arena.node.flags[self.0] = flags);
    }
    /// ```rust
    /// alien_signals::get_active_sub().unwrap().update_flags(
    ///     |f| *f &= !alien_signals::Flags::RECURSED_CHECK
    /// );
    /// ```
    pub fn update_flags(&self, f: impl FnOnce(&mut Flags)) {
        ARENA.with_borrow_mut(|arena| f(&mut arena.node.flags[self.0]));
    }
    
    pub(crate) fn deps(&self) -> Option<Link> {
        ARENA.with_borrow(|arena| arena.node.deps[self.0])
    }
    pub(crate) fn set_deps(&self, link: Option<Link>) {
        ARENA.with_borrow_mut(|arena| arena.node.deps[self.0] = link);
    }
    
    pub(crate) fn deps_tail(&self) -> Option<Link> {
        ARENA.with_borrow(|arena| arena.node.deps_tail[self.0])
    }
    pub(crate) fn set_deps_tail(&self, link: Option<Link>) {
        ARENA.with_borrow_mut(|arena| arena.node.deps_tail[self.0] = link);
    }
    
    pub(crate) fn subs(&self) -> Option<Link> {
        ARENA.with_borrow(|arena| arena.node.subs[self.0])
    }
    pub(crate) fn set_subs(&self, link: Option<Link>) {
        ARENA.with_borrow_mut(|arena| arena.node.subs[self.0] = link);
    }
    
    pub(crate) fn subs_tail(&self) -> Option<Link> {
        ARENA.with_borrow(|arena| arena.node.subs_tail[self.0])
    }
    pub(crate) fn set_subs_tail(&self, link: Option<Link>) {
        ARENA.with_borrow_mut(|arena| arena.node.subs_tail[self.0] = link);
    }
}

impl Node<NodeContext> {
    pub(crate) fn new(flags: Flags) -> Self {
        ARENA.with_borrow_mut(|arena| {
            let index = arena.node.flags.len();
            arena.node.flags.push(flags);
            arena.node.deps.push(None);
            arena.node.deps_tail.push(None);
            arena.node.subs.push(None);
            arena.node.subs_tail.push(None);
            arena.node.context_indices.push((NodeContextKind::None, 0));
            Node(index, std::marker::PhantomData)
        })
    }
    
    pub(crate) fn kind(&self) -> NodeContextKind {
        ARENA.with_borrow(|arena| arena.node.context_indices[self.0].0)
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
            let index = arena.node.flags.len();
            let context_index = arena.node.signals.len();
            arena.node.flags.push(Flags::MUTABLE);
            arena.node.deps.push(None);
            arena.node.deps_tail.push(None);
            arena.node.subs.push(None);
            arena.node.subs_tail.push(None);
            arena.node.context_indices.push((NodeContextKind::Signal, context_index));
            arena.node.signals.push(SignalContext {
                current_value: init.clone(),
                pending_value: init,
                eq: std::rc::Rc::new(move |a, b| {
                    let a = a.downcast_ref::<T>().unwrap_or_else(|| panic!("BUG: signal type is not {}", std::any::type_name::<T>()));
                    let b = b.downcast_ref::<T>().unwrap_or_else(|| panic!("BUG: signal type is not {}", std::any::type_name::<T>()));
                    eq_fn(a, b)
                }),
            });
            Node(index, std::marker::PhantomData)
        })
    }
    
    pub(crate) fn context(&self) -> SignalContext {
        ARENA.with_borrow(|arena| {
            let (kind, index) = arena.node.context_indices[self.0];
            match kind {
                NodeContextKind::Signal => arena.node.signals[index].clone(),
                _ => panic!("BUG: Node is not a Signal"),
            }
        })
    }
    pub(crate) fn update_context(&self, f: impl FnOnce(&mut SignalContext)) {
        ARENA.with_borrow_mut(|arena| {
            let (kind, index) = arena.node.context_indices[self.0];
            match kind {
                NodeContextKind::Signal => f(&mut arena.node.signals[index]),
                _ => panic!("BUG: Node is not a Signal"),
            }
        });
    }
}
impl Node<ComputedContext> {
    pub(crate) fn new<T: PartialEq + 'static>(
        getter: impl Fn(Option<&T>) -> T + 'static,
    ) -> Self {
        Self::new_with_eq_fn(getter, T::eq)
    }
    pub(crate) fn new_with_eq_fn<T: 'static>(
        getter: impl Fn(Option<&T>) -> T + 'static,
        eq_fn: impl Fn(&T, &T) -> bool + 'static,
    ) -> Self {
        ARENA.with_borrow_mut(|arena| {
            let index = arena.node.flags.len();
            let context_index = arena.node.computeds.len();
            arena.node.computeds.push(ComputedContext {
                value: None,
                get: std::rc::Rc::new(move |prev_any| {
                    let prev_t = prev_any.map(|any| any
                        .downcast_ref::<T>()
                        .unwrap_or_else(|| panic!("BUG: computed type is not {}", std::any::type_name::<T>()))
                    );
                    let new_t = getter(prev_t);
                    SmallAny::new(new_t)
                }),
                eq: std::rc::Rc::new(move |a, b| {
                    let a = a.downcast_ref::<T>().unwrap_or_else(|| panic!("BUG: computed type is not {}", std::any::type_name::<T>()));
                    let b = b.downcast_ref::<T>().unwrap_or_else(|| panic!("BUG: computed type is not {}", std::any::type_name::<T>()));
                    eq_fn(a, b)
                }),
            });
            arena.node.context_indices.push((NodeContextKind::Computed, context_index));
            arena.node.flags.push(Flags::NONE);
            arena.node.deps.push(None);
            arena.node.deps_tail.push(None);
            arena.node.subs.push(None);
            arena.node.subs_tail.push(None);
            Node(index, std::marker::PhantomData)
        })
    }
    
    pub(crate) fn context(&self) -> ComputedContext {
        ARENA.with_borrow(|arena| {
            let (kind, index) = arena.node.context_indices[self.0];
            match kind {
                NodeContextKind::Computed => arena.node.computeds[index].clone(),
                _ => panic!("BUG: Node is not a Computed"),
            }
        })
    }
    pub(crate) fn update_context(&self, f: impl FnOnce(&mut ComputedContext)) {
        ARENA.with_borrow_mut(|arena| {
            let (kind, index) = arena.node.context_indices[self.0];
            match kind {
                NodeContextKind::Computed => f(&mut arena.node.computeds[index]),
                _ => panic!("BUG: Node is not a Computed"),
            }
        });
    }
}

impl Node<EffectContext> {
    pub(crate) fn new(f: impl Fn() + 'static) -> Self {
        ARENA.with_borrow_mut(|arena| {            
            let index = arena.node.flags.len();
            let context_index = arena.node.effects.len();
            arena.node.effects.push(EffectContext {
                run: std::rc::Rc::new(f),
            });
            arena.node.context_indices.push((NodeContextKind::Effect, context_index));
            arena.node.flags.push(Flags::WATCHING | Flags::RECURSED_CHECK);
            arena.node.deps.push(None);
            arena.node.deps_tail.push(None);
            arena.node.subs.push(None);
            arena.node.subs_tail.push(None);
            Node(index, std::marker::PhantomData)
        })
    }
    
    pub(crate) fn context(&self) -> EffectContext {
        ARENA.with_borrow(|arena| {
            let (kind, index) = arena.node.context_indices[self.0];
            match kind {
                NodeContextKind::Effect => arena.node.effects[index].clone(),
                _ => panic!("BUG: Node is not an Effect"),
            }
        })
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
        match ARENA.with_borrow(|arena| arena.node.context_indices[node.0].0) {
            NodeContextKind::Signal => Ok(Node(node.0, std::marker::PhantomData)),
            _ => Err(()),
        }
    }
}
impl TryFrom<Node<NodeContext>> for Node<ComputedContext> {
    type Error = ();
    fn try_from(node: Node<NodeContext>) -> Result<Self, Self::Error> {
        match ARENA.with_borrow(|arena| arena.node.context_indices[node.0].0) {
            NodeContextKind::Computed => Ok(Node(node.0, std::marker::PhantomData)),
            _ => Err(()),
        }
    }
}
impl TryFrom<Node<NodeContext>> for Node<EffectContext> {
    type Error = ();
    fn try_from(node: Node<NodeContext>) -> Result<Self, Self::Error> {
        match ARENA.with_borrow(|arena| arena.node.context_indices[node.0].0) {
            NodeContextKind::Effect => Ok(Node(node.0, std::marker::PhantomData)),
            _ => Err(()),
        }
    }
}
