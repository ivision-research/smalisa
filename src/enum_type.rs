#[derive(Debug, Default, Clone, PartialEq)]
pub struct Enum<'a> {
    pub owner: &'a str,
    pub name: &'a str,
    pub ty: &'a str,
}
