use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Mutex,
};

use lazy_static::lazy_static;

lazy_static! {
    static ref REGISTRY: Mutex<HashMap<TypeId, Box<dyn Any + Send>>> = Mutex::new(HashMap::new());
}

/// Global Object Registry Visitor
///
/// This struct is used to register and access global objects of a specific type.
///
/// # Example
/// ```rust
/// use gom::Registry;
///
/// Registry::register("key", 123);
/// let value = Registry::<i32>::apply("key", |v| *v + 1);
/// assert_eq!(value, Some(124));
/// ```
pub struct Registry<T> {
    _use: Option<T>,
}

impl<T: Send + 'static> Registry<T> {
    /// Register a new value with the given key
    ///
    /// If the key already exists in the same type, the value will be overwritten.
    ///
    /// # Example
    /// ```rust
    /// use gom::Registry;
    ///
    /// Registry::register("key", 123);
    /// ```
    pub fn register(key: &str, value: T) {
        let mut registry = REGISTRY.lock().unwrap();
        // Check if the map already exists for this type
        if !registry.contains_key(&TypeId::of::<T>()) {
            let map: HashMap<String, T> = HashMap::new();
            registry.insert(TypeId::of::<T>(), Box::new(map));
        }
        // Get type map
        let map = registry
            .get_mut(&TypeId::of::<T>())
            .unwrap()
            .downcast_mut::<HashMap<String, T>>()
            .unwrap();
        // Insert the value into the map
        map.insert(key.to_string(), value);
    }

    /// Apply a function to the value with the given key
    ///
    /// **If this function is nested, it will cause thread deadlock.**
    ///
    /// If the key does not exist, the function will not be called and `None` will be returned.
    ///
    /// # Returns
    /// `Some(R)` if the key exists and the function was applied successfully, `None` otherwise.
    ///
    /// # Example
    /// ```rust
    /// use gom::Registry;
    ///
    /// Registry::register("key", 123);
    /// let value = Registry::<i32>::apply("key", |v| *v + 1);
    /// assert_eq!(value, Some(124));
    /// ```
    pub fn apply<R, F: FnOnce(&mut T) -> R>(key: &str, f: F) -> Option<R> {
        let mut registry = REGISTRY.lock().unwrap();
        // Get type map
        let map = registry
            .get_mut(&TypeId::of::<T>())?
            .downcast_mut::<HashMap<String, T>>()?;
        // Get value
        let value = map.get_mut(key)?;
        Some(f(value))
    }

    /// Get the value with the given key and reomve it from the registry
    /// 
    /// If the key does not exist, `None` will be returned.
    ///
    /// # Returns
    /// `Some(T)` if the key exists, `None` otherwise.
    ///
    /// # Example
    /// ```rust
    /// use gom::Registry;
    ///
    /// Registry::register("key", 123);
    /// let value = Registry::<i32>::remove("key");
    /// assert_eq!(value, Some(123));
    /// let value = Registry::<i32>::remove("key");
    /// assert_eq!(value, None);
    /// ```
    pub fn remove(key: &str) -> Option<T> {
        let mut registry = REGISTRY.lock().unwrap();
        // Get type map
        let map = registry
            .get_mut(&TypeId::of::<T>())?
            .downcast_mut::<HashMap<String, T>>()?;
        // Get value
        map.remove(key)
    }

    /// Apply a function to the value with the given key
    /// 
    /// **This function will not cause thread deadlocks.**
    /// 
    /// If the key does not exist, the function will not be called and `None` will be returned.
    /// 
    /// In the context of 'with', the value cannot be retrieved again.
    ///
    /// # Returns
    /// `Some(R)` if the key exists and the function was applied successfully, `None` otherwise.
    ///
    /// # Example
    /// ```rust
    /// use gom::Registry;
    ///
    /// Registry::register("key", 123);
    /// let value = Registry::<i32>::with("key", |v| *v + 1);
    /// assert_eq!(value, Some(124));
    /// ```
    pub fn with<R, F: FnOnce(&mut T) -> R>(key: &str, f: F) -> Option<R> {
        let mut value = Self::remove(key)?;
        let result = f(&mut value);
        Self::register(key, value);
        Some(result)
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
        {
            const ROOT: &str = $root;
            const PATH: &str = concat!($('.', stringify!($x)),+);
            constcat::concat!(ROOT, PATH)
        }
    }
}
