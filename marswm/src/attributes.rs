use libmars::common::Dimensions;

#[derive(PartialEq)]
#[derive(Default)]
pub struct Attributes {
    pub is_floating: bool,
    pub is_moving: bool,
    pub is_pinned: bool,

    pub floating_dimensions: Option<Dimensions>,
}


