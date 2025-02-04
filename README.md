# Global Object Manager (GOM)

This is a simple global object manager that makes it easier for you to use global objects in Rust.

# Example

```rust
use gom::Registry;

#[derive(Debug)]
struct Object(i32);

fn main() {
    Registry::register("Number1", 12i64);
    Registry::register("Number2", 34i32);

    println!(
        "Number1: {}",
        Registry::<i64>::apply("Number1", |x| *x).unwrap()
    );

    Registry::register("Object1", Object(56));

    Registry::apply("Object1", |obj: &mut Object| {
        obj.0 = 78;
    });

    println!(
        "Object1: {:?}",
        Registry::<Object>::remove("Object1").unwrap()
    );
}
```