use libmars::common::Dimensions;

#[derive(PartialEq)]
pub struct Attributes {
    pub is_floating: bool,
    pub is_moving: bool,
    pub is_pinned: bool,

    pub floating_dimensions: Option<Dimensions>,
}

impl Default for Attributes {
    fn default() -> Self {
        Attributes {
            is_floating: false,
            is_moving: false,
            is_pinned: false,

            floating_dimensions: None,
        }
    }
}
