mod items;

pub use items::*;

pub trait Labeled {
    fn label(&self) -> &'static str;
}
