# Global Object Manager (GOM)

This is a simple global object manager that makes it easier for you to use global objects in Rust.

# Note

You need to add crate `constcat` to support macro `id!(...)`. 

<i style="color:orange">This is a flaw in Rust.</i>


# Example

```rust
use gom::*;

const VEC: &str = id!(Vec);
const ID: &str = id!(@VEC.Bar);

fn main() {
    Registry::register(ID, vec![1, 2, 3]);

    Registry::<Vec<i32>>::with(ID, |v| {
        v.push(4);
    });

    let v = Registry::<Vec<i32>>::remove(ID);
    println!("{:?}", v);
}
```