use core::panic;
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
    marker::PhantomData,
    sync::RwLock,
};

use lazy_static::lazy_static;

macro_rules! thread_deadlock {
    () => {
        panic!("Thread deadlock!")
    };
}

lazy_static! {
    static ref _TABLE: RwLock<HashMap<TypeId, RwLock<HashMap<String, RwLock<Box<dyn Any + Send + Sync>>>>>> =
        RwLock::new(HashMap::new());
}

thread_local! {
    static _LOCAL_TABLE: RefCell<HashMap<TypeId, HashMap<String, Box<dyn Any>>>> =
        RefCell::new(HashMap::new());
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Context {
    With(String, TypeId),
    Apply(String, TypeId),
}

enum Lock {
    Global,
    Type,
    Key,
}

thread_local! {
    // 上下文访问栈
    static CONTEXT: RefCell<Vec<Context>> = RefCell::new(Vec::new());
}

struct ContextOperator;
impl ContextOperator {
    fn push(ctx: Context) {
        CONTEXT.with(|ctx_cell| {
            ctx_cell.borrow_mut().push(ctx);
        });
    }

    fn pop() {
        CONTEXT.with(|ctx_cell| ctx_cell.borrow_mut().pop());
    }

    fn cannot_lock_write_lock<T: 'static>(name: &str, lock: Lock) -> bool {
        match lock {
            Lock::Global => CONTEXT.with_borrow(|v| !v.is_empty()),
            Lock::Type => CONTEXT.with_borrow(|v| {
                v.iter().any(|x| match x {
                    Context::With(_, type_id) | Context::Apply(_, type_id) => {
                        type_id == &TypeId::of::<T>()
                    }
                })
            }),
            Lock::Key => CONTEXT.with_borrow(|v| {
                v.iter().any(|x| match x {
                    Context::With(key, type_id) | Context::Apply(key, type_id) => {
                        key == name && type_id == &TypeId::of::<T>()
                    }
                })
            }),
        }
    }
}

// 检查如果获取写锁是否会导致死锁
fn check_write_deadlock<T: 'static>(name: &str, lock: Lock) {
    if ContextOperator::cannot_lock_write_lock::<T>(name, lock) {
        thread_deadlock!();
    }
}

// 检查如果获取读锁是否会导致死锁
fn check_read_deadlock<T: 'static>(name: &str) {
    if CONTEXT.with_borrow(|v| {
        v.iter().any(|x| match x {
            Context::Apply(s, type_id) => s == name && type_id == &TypeId::of::<T>(),
            _ => false,
        })
    }) {
        thread_deadlock!();
    }
}

#[cfg(debug_assertions)]
macro_rules! check_deadlock {
    (mut $type:ty : $name:expr ; $em:expr) => {
        $crate::check_write_deadlock::<$type>($name, $em);
    };
    (ref $type:ty : $name:expr) => {
        $crate::check_read_deadlock::<$type>($name);
    };
}

#[cfg(not(debug_assertions))]
macro_rules! check_deadlock {
    (mut $type:ty : $name:expr ; $em:expr) => {};
    (ref $type:ty : $name:expr) => {};
}

/// 用于访问注册表的类型
///
/// # 注解
///
/// + 其索引方式是：`类型-键` 唯一，因而同一个键可以对应多个不同类型的值
/// + 如果闭包中使用了不恰当的嵌套，可能会导致线程死锁
pub struct Registry<T> {
    _marker: PhantomData<T>,
}

impl<T: 'static + Send + Sync + Any> Registry<T> {
    fn _register(name: &str, value: T) -> Option<()> {
        let type_id = TypeId::of::<T>();
        let has_type = {
            let map = _TABLE.read().ok()?;
            map.contains_key(&type_id)
        };
        if !has_type {
            check_deadlock!(mut T:name;Lock::Global);
            let mut map = _TABLE.write().ok()?;
            map.insert(type_id, RwLock::new(HashMap::new()));
        }
        let map = _TABLE.read().ok()?;
        check_deadlock!(mut T:name;Lock::Type);
        let mut type_map = map.get(&type_id)?.write().ok()?;
        type_map.insert(String::from(name), RwLock::new(Box::new(value)));
        Some(())
    }

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
    pub fn register(name: &str, value: T) -> Result<(), ()> {
        Self::_register(name, value).ok_or(())
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
            check_deadlock!(mut T:name;Lock::Type);
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
        check_deadlock!(mut T:name;Lock::Key);
        let mut value = type_map.get(name)?.write().ok()?;
        let var = value.downcast_mut::<T>()?;
        ContextOperator::push(Context::Apply(String::from(name), type_id));
        let ret = Some(func(var));
        ContextOperator::pop();
        ret
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
        check_deadlock!(ref T:name);
        let value = type_map.get(name)?.read().ok()?;
        let var = value.downcast_ref::<T>()?;
        ContextOperator::push(Context::With(String::from(name), type_id));
        let ret = Some(func(var));
        ContextOperator::pop();
        ret
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
            check_deadlock!(mut T:name;Lock::Type);
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

/// 针对于线程局部变量的注册表
pub struct LocalRegistry<T> {
    _marker: PhantomData<T>,
}

impl<T: 'static> LocalRegistry<T> {
    /// 向注册表中注册一个新值
    ///
    /// 如果相同的键已存在，那么旧值将会被新值替换
    ///
    /// # 示例
    /// ```rust
    /// use gom::LocalRegistry;
    ///
    /// LocalRegistry::<i32>::register("my_key", 42);
    /// ```
    pub fn register(name: &str, value: T) {
        let type_id = TypeId::of::<T>();
        let has_type = _LOCAL_TABLE.with_borrow(|table| table.contains_key(&type_id));
        if !has_type {
            _LOCAL_TABLE.with_borrow_mut(|table| {
                table.insert(type_id, HashMap::new());
            });
        }
        _LOCAL_TABLE.with_borrow_mut(|table| {
            let type_map = table.get_mut(&type_id).unwrap();
            type_map.insert(String::from(name), Box::new(value));
        })
    }

    /// 从注册表中移除指定键对应的值
    ///
    /// 如果键不存在，则返回 `None`
    ///
    /// # 示例
    /// ```rust
    /// use gom::LocalRegistry;
    ///
    /// LocalRegistry::<i32>::register("my_key", 42);
    /// assert_eq!(LocalRegistry::<i32>::remove("my_key"), Some(42));
    /// assert_eq!(LocalRegistry::<i32>::remove("my_key"), None);
    /// ```
    pub fn remove(name: &str) -> Option<T> {
        let type_id = TypeId::of::<T>();
        let value = _LOCAL_TABLE.with_borrow_mut(|table| {
            let type_map = table.get_mut(&type_id)?;
            type_map.remove(name)
        })?;
        let value = value.downcast::<T>().ok()?;
        Some(*value)
    }

    /// 判断指定键是否存在于注册表中
    ///
    /// # 示例
    /// ```rust
    /// use gom::LocalRegistry;
    ///
    /// LocalRegistry::<i32>::register("my_key", 42);
    /// assert_eq!(LocalRegistry::<i32>::exists("my_key"), true);
    /// assert_eq!(LocalRegistry::<i32>::exists("other_key"), false);
    /// ```
    pub fn exists(name: &str) -> bool {
        let type_id = TypeId::of::<T>();
        _LOCAL_TABLE.with_borrow(|table| {
            let type_map = table.get(&type_id).unwrap();
            type_map.contains_key(name)
        })
    }

    /// 向注册表中的指定键应用一个函数，该函数可以修改注册表中的值
    ///
    /// 如果键不存在，则返回 `None`；否则，返回闭包函数的返回值
    ///
    /// # 示例
    /// ```rust
    /// use gom::LocalRegistry;
    ///
    /// LocalRegistry::register("my_key", 42);
    /// assert_eq!(LocalRegistry::<i32>::apply("my_key", |v| { *v += 1; *v }), Some(43));
    /// assert_eq!(LocalRegistry::<i32>::apply("other_key", |v| *v += 1), None);
    /// ```
    pub fn apply<R, F: FnOnce(&mut T) -> R>(name: &str, func: F) -> Option<R> {
        let type_id = TypeId::of::<T>();
        _LOCAL_TABLE.with_borrow_mut(|table| {
            let type_map = table.get_mut(&type_id)?;
            let value = type_map.get_mut(name)?;
            let value = value.downcast_mut::<T>()?;
            Some(func(value))
        })
    }

    /// 向注册表中的指定键应用一个函数，该函数仅能读取注册表中的值
    ///
    /// 如果键不存在，则返回 `None`；否则，返回闭包函数的返回值
    ///
    /// # 示例
    /// ```rust
    /// use gom::LocalRegistry;
    ///
    /// LocalRegistry::<i32>::register("my_key", 42);
    /// assert_eq!(LocalRegistry::<i32>::with("my_key", |v| *v), Some(42));
    /// assert_eq!(LocalRegistry::<i32>::with("other_key", |v| *v), None);
    /// ```
    pub fn with<R, F: FnOnce(&T) -> R>(name: &str, func: F) -> Option<R> {
        let type_id = TypeId::of::<T>();
        _LOCAL_TABLE.with_borrow(|table| {
            let type_map = table.get(&type_id)?;
            let value = type_map.get(name)?;
            let value = value.downcast_ref::<T>()?;
            Some(func(value))
        })
    }

    /// 使用新值替换注册表中的指定键对应的值
    ///
    /// 如果键不存在，则返回 `None` 并且不会注册新值；否则，返回旧值
    ///
    /// # 示例
    /// ```rust
    /// use gom::LocalRegistry;
    ///
    /// LocalRegistry::<i32>::register("my_key", 42);
    /// assert_eq!(LocalRegistry::<i32>::replace("my_key", 64), Some(42));
    /// assert_eq!(LocalRegistry::<i32>::replace("other_key", 32), None);
    /// ```
    pub fn replace(name: &str, value: T) -> Option<T> {
        let type_id = TypeId::of::<T>();
        let value = _LOCAL_TABLE.with_borrow_mut(|table| {
            let type_map = table.get_mut(&type_id)?;
            type_map.insert(name.to_string(), Box::new(value))
        })?;
        let value = value.downcast::<T>().ok()?;
        Some(*value)
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
