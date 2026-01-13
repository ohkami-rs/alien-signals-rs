#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Flags(u8);
impl Flags {
    pub const NONE: Self = Self(0);
    pub const MUTABLE: Self = Self(1 << 0);
    pub const WATCHING: Self = Self(1 << 1);
    pub const RECURSED_CHECK: Self = Self(1 << 2);
    pub const RECURSED: Self = Self(1 << 3);
    pub const DIRTY: Self = Self(1 << 4);
    pub const PENDING: Self = Self(1 << 5);
}
impl Flags {
    #[inline(always)]
    pub(crate) const fn is_zero(self) -> bool {
        self.0 == 0
    }
    #[inline(always)]
    pub(crate) const fn is_nonzero(self) -> bool {
        self.0 != 0
    }
}
impl std::ops::Not for Flags {
    type Output = Self;
    #[inline(always)]
    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}
impl std::ops::BitAnd for Flags {
    type Output = Self;
    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}
impl std::ops::BitAndAssign for Flags {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs
    }
}
impl std::ops::BitOr for Flags {
    type Output = Self;
    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
impl std::ops::BitOrAssign for Flags {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs
    }
}

pub(crate) struct Stack<T>(Vec<T>);
/// not requiring `T: Default`
impl<T> Default for Stack<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T> Stack<T> {
    pub(crate) const fn new() -> Self {
        Self(Vec::new())
    }

    #[inline]
    pub(crate) fn pop(&mut self) -> Option<T> {
        self.0.pop()
    }

    #[inline]
    pub(crate) fn push(&mut self, item: T) {
        self.0.push(item);
    }
}

pub(crate) struct Queue<T>(std::collections::VecDeque<T>);
// not requiring `T: Clone`
impl<T> Default for Queue<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T> Queue<T> {
    pub(crate) const fn new() -> Self {
        Self(std::collections::VecDeque::new())
    }

    pub(crate) fn pop(&mut self) -> Option<T> {
        self.0.pop_front()
    }

    pub(crate) fn push(&mut self, item: T) {
        self.0.push_back(item);
    }

    pub(crate) fn length(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn as_slice_mut(&mut self) -> &mut [T] {
        self.0.make_contiguous()
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Version(usize);
impl Version {
    pub(crate) const fn new() -> Self {
        Self(0)
    }

    pub(crate) fn increment(&mut self) {
        self.0 += 1;
    }
}

#[derive(Clone)]
pub(crate) enum SmallAny {
    Inline([u8; 16]),
    Heap(std::rc::Rc<dyn std::any::Any>),
}
impl SmallAny {
    pub(crate) fn new<T: std::any::Any + 'static>(value: T) -> Self {
        use std::mem::{align_of, needs_drop, size_of};
        if size_of::<T>() <= 16 && !needs_drop::<T>() && align_of::<T>() <= align_of::<[u8; 16]>() {
            let mut data = [0u8; 16];
            unsafe {
                let ptr = &value as *const T as *const u8;
                std::ptr::copy_nonoverlapping(ptr, data.as_mut_ptr(), size_of::<T>());
            }
            std::mem::forget(value);
            Self::Inline(data)
        } else {
            Self::Heap(std::rc::Rc::new(value))
        }
    }

    /// SAFETY: Caller must ensure that the stored type is `T`.
    #[inline]
    pub(crate) unsafe fn downcast_ref_unchecked<T: std::any::Any + 'static>(&self) -> &T {
        match self {
            Self::Inline(data) => unsafe { &*(data.as_ptr() as *const T) },
            Self::Heap(rc_any) => unsafe { rc_any.downcast_ref::<T>().unwrap_unchecked() },
        }
    }
}

pub(crate) struct SyncUnsafeCell<T>(std::cell::UnsafeCell<T>);
unsafe impl<T: Sync> Sync for SyncUnsafeCell<T> {}
impl<T> SyncUnsafeCell<T> {
    pub(crate) const fn new(value: T) -> Self {
        Self(std::cell::UnsafeCell::new(value))
    }

    #[inline(always)]
    pub(crate) fn with_borrow<R>(&'static self, f: impl FnOnce(&T) -> R) -> R {
        let borrow = unsafe { &*self.0.get() };
        f(borrow)
    }
    #[inline(always)]
    pub(crate) fn with_borrow_mut<R>(&'static self, f: impl FnOnce(&mut T) -> R) -> R {
        let borrow_mut = unsafe { &mut *self.0.get() };
        f(borrow_mut)
    }
}

pub(crate) struct ChunkedArena<T, const CHUNK_SIZE: usize> {
    chunks: Vec<Box<[std::mem::MaybeUninit<T>; CHUNK_SIZE]>>,
    current_chunk_index: usize,
    next_slot_index: usize,
}
impl<T, const CHUNK_SIZE: usize> ChunkedArena<T, CHUNK_SIZE> {
    #[cold]
    fn make_first_chunk(&mut self) {
        if self.chunks.is_empty() {
            self.chunks.push(Box::new(
                [const { std::mem::MaybeUninit::uninit() }; CHUNK_SIZE],
            ));
        }
    }

    /// Const variant of [`Self::new`].
    /// This doesn't allocate the first chunk to be const fn.
    pub(crate) const fn new_const() -> Self {
        assert!(CHUNK_SIZE > 0, "CHUNK_SIZE must be >= 1");
        Self {
            chunks: Vec::new(),
            current_chunk_index: 0,
            next_slot_index: 0,
        }
    }

    pub(crate) fn new() -> Self {
        let mut this = Self::new_const();
        this.make_first_chunk();
        this
    }

    pub(crate) fn alloc(&mut self, value: T) -> std::ptr::NonNull<T> {
        // for the case of `new_const`
        if self.chunks.is_empty() {
            self.make_first_chunk();
        }

        if self.next_slot_index >= CHUNK_SIZE {
            self.chunks.push(Box::new(
                [const { std::mem::MaybeUninit::uninit() }; CHUNK_SIZE],
            ));
            self.current_chunk_index += 1;
            self.next_slot_index = 0;
        }
        let alloced_ptr = unsafe {
            self.chunks
                .get_unchecked_mut(self.current_chunk_index)
                .get_unchecked_mut(self.next_slot_index)
                .write(value)
        };
        self.next_slot_index += 1;
        unsafe { std::ptr::NonNull::new_unchecked(alloced_ptr) }
    }
}
impl<T, const CHUNK_SIZE: usize> Default for ChunkedArena<T, CHUNK_SIZE> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T, const CHUNK_SIZE: usize> Drop for ChunkedArena<T, CHUNK_SIZE> {
    fn drop(&mut self) {
        for chunk in &mut self.chunks[..self.current_chunk_index] {
            chunk.iter_mut().for_each(|slot| unsafe {
                std::ptr::drop_in_place(slot.as_mut_ptr());
            });
        }
        if self.next_slot_index > 0 {
            self.chunks[self.current_chunk_index]
                .iter_mut()
                .take(self.next_slot_index)
                .for_each(|slot| unsafe {
                    std::ptr::drop_in_place(slot.as_mut_ptr());
                });
        }
    }
}
