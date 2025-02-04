use gom::{id, Registry};

#[derive(Debug)]
struct Object(i32);

fn main() {
    const NUMBER: &str = id!(Number);
    Registry::register(id!(@NUMBER.one), 12i64);
    Registry::register("Number2", 34i32);

    println!(
        "{}: {}",
        id!(@NUMBER.one),
        Registry::<i64>::apply(id!(@NUMBER.one), |x| *x).unwrap()
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
