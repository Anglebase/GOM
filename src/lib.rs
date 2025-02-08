use std::{
    any::{Any, TypeId},
    collections::HashMap,
    marker::PhantomData,
    sync::RwLock,
};

use lazy_static::lazy_static;

lazy_static! {
    static ref _TABLE: RwLock<HashMap<TypeId, RwLock<HashMap<String, RwLock<Box<dyn Any + Send + Sync>>>>>> =
        RwLock::new(HashMap::new());
}

pub struct Registry<T> {
    _marker: PhantomData<T>,
}

impl<T: 'static + Send + Sync + Any> Registry<T> {
    pub fn register(name: &str, value: T) {
        let type_id = TypeId::of::<T>();
        let has_type = {
            let map = _TABLE.read().unwrap();
            map.contains_key(&type_id)
        };
        if !has_type {
            let mut map = _TABLE.write().unwrap();
            map.insert(type_id, RwLock::new(HashMap::new()));
        }
        let map = _TABLE.read().unwrap();
        let mut type_map = map.get(&type_id).unwrap().write().unwrap();
        type_map.insert(String::from(name), RwLock::new(Box::new(value)));
    }

    pub fn remove(name: &str) -> Option<T> {
        let type_id = TypeId::of::<T>();
        let lock_value = {
            let map = _TABLE.read().ok()?;
            let type_map = map.get(&type_id)?;
            let mut type_map = type_map.write().ok()?;
            type_map.remove(name)?
        };
        let value = lock_value.into_inner().ok()?;
        let type_value = value.downcast::<T>().ok()?;
        Some(*type_value)
    }

    fn _exists(name: &str) -> Option<bool> {
        let type_id = TypeId::of::<T>();
        let map = _TABLE.read().ok()?;
        let lock_type_map = map.get(&type_id)?;
        let type_map = lock_type_map.read().ok()?;
        Some(type_map.contains_key(name))
    }

    pub fn exists(name: &str) -> bool {
        Self::_exists(name).unwrap_or(false)
    }

    pub fn apply<R, F: FnOnce(&mut T) -> R>(name: &str, func: F) -> Option<R> {
        let type_id = TypeId::of::<T>();
        let type_map = _TABLE.read().ok()?;
        let type_map = type_map.get(&type_id)?.read().ok()?;
        let mut value = type_map.get(name)?.write().ok()?;
        let var = value.downcast_mut::<T>()?;
        Some(func(var))
    }

    pub fn with<R, F: FnOnce(&T) -> R>(name: &str, func: F) -> Option<R> {
        let type_id = TypeId::of::<T>();
        let type_map = _TABLE.read().ok()?;
        let type_map = type_map.get(&type_id)?.read().ok()?;
        let value = type_map.get(name)?.read().ok()?;
        let var = value.downcast_ref::<T>()?;
        Some(func(var))
    }

    pub fn replace(name: &str, value: T) -> Option<T> {
        let type_id = TypeId::of::<T>();
        let type_map = _TABLE.read().ok()?;
        let type_map = type_map.get(&type_id)?;
        let value = {
            let mut type_map = type_map.write().ok()?;
            let ret = type_map.remove(name)?;
            type_map.insert(String::from(name), RwLock::new(Box::new(value)));
            ret
        };
        let value = value.into_inner().ok()?;
        let type_value = value.downcast::<T>().ok()?;
        Some(*type_value)
    }

    #[deprecated(since = "0.1.6", note = "use `replace` instead")]
    pub fn take(name: &str, value: T) -> Option<T> {
        Self::replace(name, value)
    }
}

/// Make a identifier string with the given path
///
/// ```rust
/// use gom::id;
///
/// const MY_ID: &str = id!(my.module.MyType);
/// const OTHER_ID: &str = id!(@MY_ID.other.OtherType);
///
/// assert_eq!(MY_ID, ".my.module.MyType");
/// assert_eq!(OTHER_ID, ".my.module.MyType.other.OtherType");
/// ```
#[macro_export]
macro_rules! id {
    ($($x:ident).+) => {
        concat!($('.', stringify!($x)),+)
    };
    (@ $root:ident . $($x:ident).+) => {
        constcat::concat!($root, concat!($('.', stringify!($x)),+))
    }
}
