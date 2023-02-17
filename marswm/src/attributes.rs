use libmars::common::Dimensions;

#[derive(PartialEq)]
pub struct Attributes {
    pub floating_dimensions: Option<Dimensions>,
    pub stack_position: Option<usize>,
}

impl Default for Attributes {
    fn default() -> Self {
        return Attributes {
            floating_dimensions: None,
            stack_position: None,
        };
    }
}
