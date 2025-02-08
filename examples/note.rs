use gom::*;

const ROOT: &str = id!(ROOT);
const NOTE: &str = id!(@ROOT.note);

pub struct Note {
    text: String,
}

fn note(text: &str) -> String {
    if !Registry::<Note>::exists(NOTE) {
        let note = Note {
            text: Default::default(),
        };
        Registry::register(NOTE, note).unwrap();
    }
    Registry::<Note>::apply(NOTE, |t| {
        let ret = t.text.clone();
        t.text = text.to_string();
        ret
    })
    .unwrap()
}

fn main() {
    println!("{:?}", note("Hello, world!"));
    println!("{:?}", note("Goodbye, world!"));
    println!("{:?}", note("How are you?"));
}
