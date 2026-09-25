#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Plan {
    pub(crate) primary: [bool; 2],
    pub(crate) normal: [bool; 2],
    pub(crate) checked: bool,
}
