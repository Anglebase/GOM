use gom::*;

const ID1: &str = "id1";
const ID2: &str = "id2";

fn main() {
    Registry::register(ID1, 1).unwrap();
    Registry::register(ID2, 2.0).unwrap();

    Registry::<i32>::with(ID1, |v| {
        println!("id1: {}", v);
        Registry::register("id3", 2.9).unwrap();
    });

    Registry::<f64>::with("id3", |f| {
        println!("id3: {}", f);
    });
}
