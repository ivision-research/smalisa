#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Enum<'a> {
    pub owner: &'a str,
    pub name: &'a str,
    pub ty: &'a str,
}
