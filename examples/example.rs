use gom::*;

const VEC: &str = id!(Vec);
const ID: &str = id!(@VEC.Bar);

fn main() {
    Registry::register(ID, vec![1, 2, 3]).unwrap();

    Registry::<Vec<i32>>::apply(ID, |v| {
        v.push(4);
    });

    let v = Registry::<Vec<i32>>::remove(ID);
    println!("{:?}", v);
}
