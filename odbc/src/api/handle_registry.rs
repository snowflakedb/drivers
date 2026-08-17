use snafu::OptionExt;
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

use parking_lot::{ArcRwLockReadGuard, ArcRwLockWriteGuard, Mutex, RawRwLock, RwLock};

use crate::api::OdbcResult;
use crate::api::error::InvalidHandleSnafu;
use crate::api::types::DescriptorKind;
use odbc_sys as sql;

/// Discriminant packed into the high bits of an opaque `SQLHANDLE`.
///
/// Uses 2 bits so 32-bit Windows retains ~30 bits of slot space. All four
/// values are assigned; there is no unused 2-bit pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum HandleKind {
    /// Implicit and explicit descriptors. Bit pattern `00` so Desc#N packs as
    /// a small integer; Env/Dbc/Stmt use non-zero high bits and never collide.
    Desc = 0,
    Env = 1,
    Dbc = 2,
    Stmt = 3,
}

impl HandleKind {
    const fn from_bits(bits: usize) -> Option<Self> {
        match bits {
            0 => Some(Self::Desc),
            1 => Some(Self::Env),
            2 => Some(Self::Dbc),
            3 => Some(Self::Stmt),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DescLookup {
    Implicit {
        stmt_id: HandleId,
        kind: DescriptorKind,
    },
    Explicit {
        conn_id: HandleId,
    },
}

/// ODBC-local handle identity: a typed slot index into one of the four
/// [`HandleManager`] registries.
///
/// Slot `0` is the null sentinel (`SQL_NULL_HANDLE`). Non-zero slots are
/// packed with [`HandleKind`] in the high bits of the opaque `SQLHANDLE`
/// returned to applications.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandleId {
    kind: HandleKind,
    /// 1-based registry slot; `0` means null / not yet assigned.
    slot: usize,
}

impl Default for HandleId {
    fn default() -> Self {
        // Canonical null — kind is irrelevant when slot == 0; pack always
        // yields a null pointer. Desc is the zero bit-pattern.
        Self {
            kind: HandleKind::Desc,
            slot: 0,
        }
    }
}

impl fmt::Debug for HandleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.slot == 0 {
            f.write_str("HandleId(null)")
        } else {
            write!(f, "{:?}#{}", self.kind, self.slot)
        }
    }
}

/// Top 2 bits hold [`HandleKind`]; remaining bits hold the 1-based slot.
const KIND_BITS: u32 = 2;
const KIND_SHIFT: u32 = usize::BITS - KIND_BITS;
const SLOT_MASK: usize = (1usize << KIND_SHIFT) - 1;

impl HandleId {
    pub fn new(kind: HandleKind, slot: usize) -> Self {
        Self { kind, slot }
    }

    pub fn kind(self) -> HandleKind {
        self.kind
    }

    pub fn slot(self) -> usize {
        self.slot
    }

    /// Reject null (slot 0) or a handle whose packed kind does not match
    /// `expected`. Used at ABI entry points before registry lookup.
    pub fn require_kind(self, expected: HandleKind) -> OdbcResult<Self> {
        if self.slot == 0 || self.kind != expected {
            return InvalidHandleSnafu.fail();
        }
        Ok(self)
    }
}

impl From<HandleId> for sql::Handle {
    fn from(handle_id: HandleId) -> Self {
        // Never OR kind bits onto the null sentinel — Statement::new leaves
        // desc handles as Default until register_desc_handles assigns them,
        // and apps compare attribute values against SQL_NULL_HDESC.
        if handle_id.slot == 0 {
            return std::ptr::null_mut();
        }
        debug_assert!(
            handle_id.slot <= SLOT_MASK,
            "handle slot exceeds bit budget for this pointer width"
        );
        let raw = ((handle_id.kind as usize) << KIND_SHIFT) | (handle_id.slot & SLOT_MASK);
        raw as sql::Handle
    }
}

impl From<sql::Handle> for HandleId {
    fn from(handle: sql::Handle) -> Self {
        if handle.is_null() {
            return Self::default();
        }
        let raw = handle as usize;
        let kind_bits = raw >> KIND_SHIFT;
        let slot = raw & SLOT_MASK;
        // All 2-bit patterns are valid HandleKinds; from_bits is infallible
        // for values that fit in KIND_BITS.
        let kind = HandleKind::from_bits(kind_bits).unwrap_or(HandleKind::Desc);
        if slot == 0 {
            return Self::default();
        }
        Self { kind, slot }
    }
}

struct AllocState {
    free_ids: Vec<usize>,
    next_id: usize,
}

pub struct HandleManager<T> {
    kind: HandleKind,
    slots: Mutex<Vec<Arc<RwLock<Option<T>>>>>,
    alloc: Arc<Mutex<AllocState>>,
}

impl<T> HandleManager<T> {
    pub fn new(kind: HandleKind) -> Self {
        Self {
            kind,
            slots: Mutex::new(Vec::new()),
            alloc: Arc::new(Mutex::new(AllocState {
                free_ids: Vec::new(),
                next_id: 1,
            })),
        }
    }

    pub fn add(&self, value: T) -> OdbcResult<HandleId> {
        let mut alloc = self.alloc.lock();
        let slot = alloc.free_ids.pop().unwrap_or_else(|| {
            let id = alloc.next_id;
            alloc.next_id += 1;
            id
        });
        drop(alloc);

        let mut slots = self.slots.lock();
        let idx = slot - 1; // slots are 1-based
        if idx >= slots.len() {
            slots.resize_with(idx + 1, || Arc::new(RwLock::new(None)));
        }
        *slots[idx].write() = Some(value);
        Ok(HandleId::new(self.kind, slot))
    }

    pub fn get(&self, id: HandleId) -> OdbcResult<HandleGuard<T>> {
        if id.slot == 0 || id.kind != self.kind {
            return InvalidHandleSnafu.fail();
        }
        let slot = {
            let slots = self.slots.lock();
            let idx = id.slot.wrapping_sub(1);
            slots
                .get(idx)
                .cloned()
                .with_context(|| InvalidHandleSnafu)?
        };
        let guard = slot.read_arc();
        if guard.is_none() {
            return InvalidHandleSnafu.fail();
        }
        Ok(HandleGuard { guard })
    }

    pub fn get_for_delete(&self, id: HandleId) -> OdbcResult<DeleteGuard<T>> {
        if id.slot == 0 || id.kind != self.kind {
            return InvalidHandleSnafu.fail();
        }
        let slot = {
            let slots = self.slots.lock();
            let idx = id.slot.wrapping_sub(1);
            slots
                .get(idx)
                .cloned()
                .with_context(|| InvalidHandleSnafu)?
        };
        let guard = slot.write_arc();
        if guard.is_none() {
            return InvalidHandleSnafu.fail();
        }
        Ok(DeleteGuard {
            guard,
            id,
            alloc: Arc::clone(&self.alloc),
        })
    }
}

pub struct HandleGuard<T> {
    guard: ArcRwLockReadGuard<RawRwLock, Option<T>>,
}

impl<T> Deref for HandleGuard<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.guard.as_ref().expect(
            "read lock guarantees Some: only get_for_delete (write lock) clears the slot, \
             and our read lock prevents that",
        )
    }
}

pub struct DeleteGuard<T> {
    guard: ArcRwLockWriteGuard<RawRwLock, Option<T>>,
    id: HandleId,
    alloc: Arc<Mutex<AllocState>>,
}

impl<T> DeleteGuard<T> {
    pub fn value(&self) -> &T {
        self.guard
            .as_ref()
            .expect("delete guard holds the value until delete() consumes it")
    }

    pub fn delete(mut self) {
        *self.guard = None;
        self.alloc.lock().free_ids.push(self.id.slot);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_sql_handle_is_pointer_sized() {
        let id = HandleId::new(HandleKind::Dbc, 1);
        let handle: sql::Handle = id.into();
        assert_eq!(
            std::mem::size_of_val(&handle),
            std::mem::size_of::<sql::Handle>()
        );
    }

    #[test]
    fn handle_kind_from_bits_covers_all_two_bit_patterns() {
        assert_eq!(HandleKind::from_bits(0), Some(HandleKind::Desc));
        assert_eq!(HandleKind::from_bits(1), Some(HandleKind::Env));
        assert_eq!(HandleKind::from_bits(2), Some(HandleKind::Dbc));
        assert_eq!(HandleKind::from_bits(3), Some(HandleKind::Stmt));
        assert_eq!(HandleKind::from_bits(4), None);
    }

    #[test]
    fn pack_round_trip_for_each_kind() {
        for kind in [
            HandleKind::Desc,
            HandleKind::Env,
            HandleKind::Dbc,
            HandleKind::Stmt,
        ] {
            for slot in [1usize, 42, SLOT_MASK.min(1_000_000)] {
                let id = HandleId::new(kind, slot);
                let handle: sql::Handle = id.into();
                assert!(!handle.is_null());
                assert_eq!(HandleId::from(handle), id);

                // Both From directions: opaque bits survive unpack → repack.
                let id2 = HandleId::from(handle);
                let handle2: sql::Handle = id2.into();
                assert_eq!(handle as usize, handle2 as usize);
                assert_eq!(id2.kind(), kind);
                assert_eq!(id2.slot(), slot);
            }
        }
    }

    #[test]
    fn distinct_bit_patterns_at_slot_one() {
        let handles: Vec<sql::Handle> = [
            HandleKind::Desc,
            HandleKind::Env,
            HandleKind::Dbc,
            HandleKind::Stmt,
        ]
        .into_iter()
        .map(|k| HandleId::new(k, 1).into())
        .collect();
        for i in 0..handles.len() {
            for j in (i + 1)..handles.len() {
                assert_ne!(
                    handles[i], handles[j],
                    "slot-1 handles for different kinds must not collide"
                );
            }
        }
    }

    #[test]
    fn null_sentinel_round_trips() {
        let null_id = HandleId::default();
        let handle: sql::Handle = null_id.into();
        assert!(handle.is_null());
        assert_eq!(HandleId::from(handle), HandleId::default());
        assert_eq!(HandleId::from(std::ptr::null_mut()), HandleId::default());
    }

    #[test]
    fn require_kind_rejects_null_and_mismatch() {
        assert!(HandleId::default().require_kind(HandleKind::Dbc).is_err());
        let dbc = HandleId::new(HandleKind::Dbc, 1);
        assert!(dbc.require_kind(HandleKind::Dbc).is_ok());
        assert!(dbc.require_kind(HandleKind::Stmt).is_err());
    }

    #[test]
    fn same_slot_different_kind_not_equal() {
        assert_ne!(
            HandleId::new(HandleKind::Env, 1),
            HandleId::new(HandleKind::Dbc, 1)
        );
    }

    #[test]
    fn manager_add_get() {
        let mgr: HandleManager<i32> = HandleManager::new(HandleKind::Dbc);
        let id = mgr.add(42).unwrap();
        assert_eq!(id.kind(), HandleKind::Dbc);
        assert_eq!(id.slot(), 1);
        assert_eq!(*mgr.get(id).unwrap(), 42);
    }

    #[test]
    fn manager_slots_are_contiguous() {
        let mgr: HandleManager<i32> = HandleManager::new(HandleKind::Stmt);
        let a = mgr.add(1).unwrap();
        let b = mgr.add(2).unwrap();
        let c = mgr.add(3).unwrap();
        assert_eq!([a.slot(), b.slot(), c.slot()], [1, 2, 3]);
        let ha: sql::Handle = a.into();
        let hb: sql::Handle = b.into();
        // Same kind: packed values differ only by 1 in the low bits.
        assert_eq!((hb as usize) - (ha as usize), 1);
    }

    #[test]
    fn manager_rejects_wrong_kind_without_touching_peer() {
        let env_mgr: HandleManager<i32> = HandleManager::new(HandleKind::Env);
        let dbc_mgr: HandleManager<i32> = HandleManager::new(HandleKind::Dbc);
        let env_id = env_mgr.add(10).unwrap();
        let dbc_id = dbc_mgr.add(20).unwrap();
        assert_eq!(env_id.slot(), dbc_id.slot());

        assert!(dbc_mgr.get(env_id).is_err());
        assert!(env_mgr.get(dbc_id).is_err());
        assert!(dbc_mgr.get_for_delete(env_id).is_err());
        // Peer slots still intact.
        assert_eq!(*env_mgr.get(env_id).unwrap(), 10);
        assert_eq!(*dbc_mgr.get(dbc_id).unwrap(), 20);
    }

    #[test]
    fn manager_get_for_delete_and_delete() {
        let mgr: HandleManager<i32> = HandleManager::new(HandleKind::Env);
        let id = mgr.add(42).unwrap();
        let guard = mgr.get_for_delete(id).unwrap();
        assert_eq!(*guard.value(), 42);
        guard.delete();
        assert!(mgr.get(id).is_err());
    }

    #[test]
    fn manager_delete_guard_drop_restores() {
        let mgr: HandleManager<i32> = HandleManager::new(HandleKind::Desc);
        let id = mgr.add(42).unwrap();
        {
            let _guard = mgr.get_for_delete(id).unwrap();
            // drop without calling delete()
        }
        assert_eq!(*mgr.get(id).unwrap(), 42);
    }

    #[test]
    fn manager_id_recycling_preserves_kind() {
        let mgr: HandleManager<i32> = HandleManager::new(HandleKind::Stmt);
        let id1 = mgr.add(1).unwrap();
        let id2 = mgr.add(2).unwrap();
        mgr.get_for_delete(id1).unwrap().delete();
        let id3 = mgr.add(3).unwrap();
        assert_eq!(id3, id1);
        assert_eq!(id3.kind(), HandleKind::Stmt);
        assert_eq!(*mgr.get(id3).unwrap(), 3);
        assert_eq!(*mgr.get(id2).unwrap(), 2);
        let packed: sql::Handle = id3.into();
        assert_eq!(HandleId::from(packed).kind(), HandleKind::Stmt);
    }

    #[test]
    fn manager_ids_start_at_one() {
        let mgr: HandleManager<i32> = HandleManager::new(HandleKind::Env);
        let id = mgr.add(1).unwrap();
        assert_eq!(id, HandleId::new(HandleKind::Env, 1));
    }

    #[test]
    fn empty_slot_correct_kind_is_invalid() {
        let mgr: HandleManager<i32> = HandleManager::new(HandleKind::Dbc);
        let ghost = HandleId::new(HandleKind::Dbc, 1);
        assert!(mgr.get(ghost).is_err());
        let live = mgr.add(7).unwrap();
        mgr.get_for_delete(live).unwrap().delete();
        assert!(mgr.get(live).is_err());
    }
}
