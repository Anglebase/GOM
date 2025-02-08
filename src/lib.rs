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

/// 用于访问注册表的类型
pub struct Registry<T> {
    _marker: PhantomData<T>,
}

impl<T: 'static + Send + Sync + Any> Registry<T> {
    /// 向注册表中注册一个新值
    /// 
    /// 如果相同的键已存在，那么旧值将会被新值替换
    /// 
    /// # 示例
    /// 
    /// ```rust
    /// use gom::Registry;
    ///
    /// Registry::<i32>::register("my_key", 42);
    /// Registry::register("my_key", 64);
    /// ```
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

    /// 从注册表中移除指定键对应的值
    /// 
    /// 如果键不存在，则返回 `None`
    /// 
    /// # 示例
    /// 
    /// ```rust
    /// use gom::Registry;
    ///
    /// Registry::<i32>::register("my_key", 42);
    /// assert_eq!(Registry::<i32>::remove("my_key"), Some(42));
    /// assert_eq!(Registry::<i32>::remove("my_key"), None);
    /// ```
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

    /// 判断指定键是否存在于注册表中
    /// 
    /// # 示例
    /// 
    /// ```rust
    /// use gom::Registry;
    ///
    /// Registry::<i32>::register("my_key", 42);
    /// assert_eq!(Registry::<i32>::exists("my_key"), true);
    /// assert_eq!(Registry::<i32>::exists("other_key"), false);
    /// ```
    pub fn exists(name: &str) -> bool {
        Self::_exists(name).unwrap_or(false)
    }

    /// 向注册表中的指定键应用一个函数，该函数可以修改注册表中的值
    /// 
    /// 如果键不存在，则返回 `None`；否则，返回闭包函数的返回值
    /// 
    /// # 示例
    /// ```rust
    /// use gom::Registry;
    /// 
    /// Registry::<i32>::register("my_key", 42);
    /// assert_eq!(Registry::<i32>::apply("my_key", |v| { *v += 1; *v }), Some(43));
    /// assert_eq!(Registry::<i32>::apply("other_key", |v| *v += 1), None);
    /// ```
    pub fn apply<R, F: FnOnce(&mut T) -> R>(name: &str, func: F) -> Option<R> {
        let type_id = TypeId::of::<T>();
        let type_map = _TABLE.read().ok()?;
        let type_map = type_map.get(&type_id)?.read().ok()?;
        let mut value = type_map.get(name)?.write().ok()?;
        let var = value.downcast_mut::<T>()?;
        Some(func(var))
    }

    /// 向注册表中的指定键应用一个函数，该函数仅能读取注册表中的值
    /// 
    /// 如果键不存在，则返回 `None`；否则，返回闭包函数的返回值
    /// 
    /// # 示例
    /// ```rust
    /// use gom::Registry;
    /// 
    /// Registry::<i32>::register("my_key", 42);
    /// assert_eq!(Registry::<i32>::with("my_key", |v| *v), Some(42));
    /// assert_eq!(Registry::<i32>::with("other_key", |v| *v), None);
    /// ```
    pub fn with<R, F: FnOnce(&T) -> R>(name: &str, func: F) -> Option<R> {
        let type_id = TypeId::of::<T>();
        let type_map = _TABLE.read().ok()?;
        let type_map = type_map.get(&type_id)?.read().ok()?;
        let value = type_map.get(name)?.read().ok()?;
        let var = value.downcast_ref::<T>()?;
        Some(func(var))
    }

    /// 使用新值替换注册表中的指定键对应的值
    /// 
    /// 如果键不存在，则返回 `None` 并且不会注册新值；否则，返回旧值
    /// 
    /// # 示例
    /// ```rust
    /// use gom::Registry;
    /// 
    /// Registry::<i32>::register("my_key", 42);
    /// assert_eq!(Registry::<i32>::replace("my_key", 64), Some(42));
    /// assert_eq!(Registry::<i32>::replace("other_key", 32), None);
    /// ```
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

    /// 与 `replace` 相同，但已弃用，请使用 `replace` 替代
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
