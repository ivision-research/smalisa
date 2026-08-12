#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
pub struct Enum<'a> {
    pub owner: &'a str,
    pub name: &'a str,
    pub ty: &'a str,
}
