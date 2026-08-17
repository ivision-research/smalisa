use std::{
    borrow::{Borrow, Cow},
    fmt,
    ops::Deref,
};

use crate::{method::MethodLineBuilder, AccessFlag, Annotation, Field, Line, Method};

/// A wrapper for class names with some convenience methods
///
/// This type can be used as a [Cow] with [OwnedSmaliClassName] as the owned counterpart
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
#[repr(transparent)]
pub struct SmaliClassName(str);

impl<'a> Default for &'a SmaliClassName {
    fn default() -> Self {
        SmaliClassName::new("")
    }
}

/// The owned version of a [ClassName]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct OwnedSmaliClassName(String);

impl fmt::Display for OwnedSmaliClassName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl OwnedSmaliClassName {
    fn new(s: String) -> Self {
        Self(s)
    }
}
impl SmaliClassName {
    pub(crate) fn new(s: &str) -> &Self {
        // SAFETY: repr(transparent) guarantees identical layout to str
        unsafe { &*(s as *const str as *const Self) }
    }

    /// View the [SmaliClassName] as a str
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Return the class with the leading `L` and trailing `;` removed
    ///
    /// Lfoo/bar/Baz; -> foo/bar/Baz
    pub fn without_markers(&self) -> &str {
        if self.0.len() < 2 {
            ""
        } else {
            &self.0[1..self.0.len() - 1]
        }
    }

    /// Convert the [SmaliClassName] to an [OwnedJavaClassName]
    pub fn as_java(&self) -> OwnedJavaClassName {
        OwnedJavaClassName((self.without_markers()).replace('/', "."))
    }
}

impl ClassName for SmaliClassName {
    fn as_smali_class_name(&self) -> Cow<'_, SmaliClassName> {
        Cow::Borrowed(self)
    }

    fn as_java_class_name(&self) -> Cow<'_, JavaClassName> {
        Cow::Owned(self.as_java())
    }

    fn split_java_package(&self) -> (Cow<'_, str>, &'_ str) {
        let unmarked = self.without_markers();

        // Lfoo; has no package
        let Some((pkg, clazz)) = unmarked.rsplit_once('/') else {
            return (Cow::Borrowed(""), unmarked);
        };

        // Lfoo/Baz; can borrow everything while Lfoo/bar/Baz; can't
        let pkg = if pkg.contains('/') {
            Cow::Owned(pkg.replace('/', "."))
        } else {
            Cow::Borrowed(pkg)
        };

        (pkg, clazz)
    }
}

impl SmaliClassName {
    /// Convert a class name from either a Java or Smali representation
    ///
    /// This method relies on the caller knowing what they want, [ClassName]s are deliberably
    /// narrow: they only include `Lfoo/bar/Baz;`/`foo.bar.Baz` or `LBaz;`/`Baz`. If you try to pass
    /// in something like `[Lfoo/bar/Baz;` that is not a type handled by [ClassName]s and this
    /// method will just quietly wrap it as `L[foo/bar/Baz;;` which is definitely not want you want.
    pub fn from_raw(value: &str) -> Cow<'_, Self> {
        if value.starts_with('L') && value.ends_with(';') {
            Cow::Borrowed(SmaliClassName::new(value))
        } else {
            let owned = format!("L{};", value.replace('.', "/"));
            Cow::Owned(OwnedSmaliClassName::new(owned))
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
#[repr(transparent)]
pub struct JavaClassName(str);

impl<'a> Default for &'a JavaClassName {
    fn default() -> Self {
        JavaClassName::new("")
    }
}

impl JavaClassName {
    fn new(s: &str) -> &Self {
        // SAFETY: repr(transparent) guarantees identical layout to str
        unsafe { &*(s as *const str as *const Self) }
    }
}

impl JavaClassName {
    /// Convert a class name from either a Java or Smali representation
    ///
    /// The same caveats from [ClassName::from_raw] apply here
    pub fn from_raw(value: &str) -> Cow<'_, Self> {
        if value.starts_with('L') && value.ends_with(';') {
            Cow::Owned(SmaliClassName::new(value).as_java())
        } else {
            Cow::Borrowed(JavaClassName::new(value))
        }
    }

    /// View the [JavaClassName] as a str
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert the [JavaClassName] to an [OwnedClassName]
    pub fn as_smali(&self) -> OwnedSmaliClassName {
        OwnedSmaliClassName::new(format!("L{};", self.0.replace('.', "/")))
    }
}

/// Trait unifying [SmaliClassName] and [JavaClassName] as well as their owned counterparts
pub trait ClassName {
    fn as_smali_class_name(&self) -> Cow<'_, SmaliClassName>;

    fn as_java_class_name(&self) -> Cow<'_, JavaClassName>;

    /// Retrieve the Java form of the package and class
    fn split_java_package(&self) -> (Cow<'_, str>, &'_ str);
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct OwnedJavaClassName(String);

/// Represents a fully parsed class.
#[derive(Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
pub struct Class<'a> {
    pub access: AccessFlag,
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub name: &'a SmaliClassName,
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub parent: &'a SmaliClassName,
    pub interfaces: Vec<&'a SmaliClassName>,
    pub annotations: Vec<Annotation<'a>>,
    pub methods: Vec<Method<'a>>,
    pub fields: Vec<Field<'a>>,
}

pub struct ClassLineBuilder<'a> {
    pub class: Class<'a>,

    method: Option<MethodLineBuilder<'a>>,
}

impl<'a> Default for ClassLineBuilder<'a> {
    fn default() -> Self {
        Self {
            class: Class::default(),
            method: None,
        }
    }
}

impl<'a> ClassLineBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a [Line] into the builder
    ///
    /// Note that this function is not state aware, a new [Line::Class] or other class related lines
    /// will overwrite any previously set state. It is up to the caller to manage state and ensure
    /// only a single class's [Line]s are pushed into the builder
    pub fn push_line(&mut self, line: Line<'a>) {
        if matches!(line, Line::MethodEnd) {
            if let Some(method) = self.method.take() {
                self.class.methods.push(method.finish());
            }
        } else if let Some(method) = &mut self.method {
            method.push_line(line)
        } else {
            match line {
                Line::Class(acc, cd) => {
                    self.class.access = acc;
                    self.class.name = cd;
                }
                Line::Super(sup) => {
                    self.class.parent = sup;
                }
                Line::Interface(inf) => {
                    self.class.interfaces.push(inf);
                }
                Line::MethodHeader(mh) => {
                    self.method = Some(MethodLineBuilder::new(&mh));
                }
                Line::Field(field) => {
                    self.class.fields.push(field);
                }
                Line::Annotation(ann) => {
                    self.class.annotations.push(ann);
                }
                _ => {}
            }
        }
    }

    pub fn finish(self) -> Class<'a> {
        // Note that we don't take out of `self.method` here because that would mean it is an
        // incomplete method: methods should always be taken out when a Line::MethodEnd is
        // discovered during building
        self.class
    }
}

impl ClassName for JavaClassName {
    fn split_java_package(&self) -> (Cow<'_, str>, &'_ str) {
        match self.0.rsplit_once('.') {
            Some((pkg, clazz)) => (Cow::Borrowed(pkg), clazz),
            None => (Cow::Borrowed(""), &self.0),
        }
    }

    fn as_smali_class_name(&self) -> Cow<'_, SmaliClassName> {
        Cow::Owned(self.as_smali())
    }

    fn as_java_class_name(&self) -> Cow<'_, JavaClassName> {
        Cow::Borrowed(self)
    }
}

impl ClassName for OwnedJavaClassName {
    fn as_java_class_name(&self) -> Cow<'_, JavaClassName> {
        self.as_ref().as_java_class_name()
    }

    fn as_smali_class_name(&self) -> Cow<'_, SmaliClassName> {
        self.as_ref().as_smali_class_name()
    }

    fn split_java_package(&self) -> (Cow<'_, str>, &'_ str) {
        self.as_ref().split_java_package()
    }
}

impl ClassName for OwnedSmaliClassName {
    fn as_java_class_name(&self) -> Cow<'_, JavaClassName> {
        self.as_ref().as_java_class_name()
    }

    fn as_smali_class_name(&self) -> Cow<'_, SmaliClassName> {
        self.as_ref().as_smali_class_name()
    }

    fn split_java_package(&self) -> (Cow<'_, str>, &'_ str) {
        self.as_ref().split_java_package()
    }
}

impl AsRef<SmaliClassName> for SmaliClassName {
    fn as_ref(&self) -> &SmaliClassName {
        &self
    }
}

impl ToOwned for SmaliClassName {
    type Owned = OwnedSmaliClassName;

    fn to_owned(&self) -> OwnedSmaliClassName {
        OwnedSmaliClassName(self.0.to_string())
    }
}

impl Borrow<SmaliClassName> for OwnedSmaliClassName {
    fn borrow(&self) -> &SmaliClassName {
        SmaliClassName::new(&self.0)
    }
}

impl Deref for OwnedSmaliClassName {
    type Target = SmaliClassName;

    fn deref(&self) -> &SmaliClassName {
        self.borrow()
    }
}

impl AsRef<str> for SmaliClassName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<SmaliClassName> for OwnedSmaliClassName {
    fn as_ref(&self) -> &SmaliClassName {
        self
    }
}

impl PartialEq<SmaliClassName> for OwnedSmaliClassName {
    fn eq(&self, other: &SmaliClassName) -> bool {
        **self == *other
    }
}

impl<'a> From<&'a SmaliClassName> for Cow<'a, SmaliClassName> {
    fn from(value: &'a SmaliClassName) -> Self {
        Cow::Borrowed(value)
    }
}

impl From<OwnedSmaliClassName> for Cow<'_, SmaliClassName> {
    fn from(value: OwnedSmaliClassName) -> Self {
        Cow::Owned(value)
    }
}

impl fmt::Display for SmaliClassName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for SmaliClassName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de: 'a, 'a> serde::Deserialize<'de> for &'a SmaliClassName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(SmaliClassName::new(<&'a str>::deserialize(deserializer)?))
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for JavaClassName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de: 'a, 'a> serde::Deserialize<'de> for &'a JavaClassName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(JavaClassName::new(<&'a str>::deserialize(deserializer)?))
    }
}

impl<'a> From<&JavaClassName> for Cow<'a, SmaliClassName> {
    fn from(value: &JavaClassName) -> Self {
        Cow::Owned(value.as_smali())
    }
}

impl<'a> From<OwnedJavaClassName> for Cow<'a, SmaliClassName> {
    fn from(value: OwnedJavaClassName) -> Self {
        Cow::Owned(value.as_smali())
    }
}

impl<'a> From<&'a OwnedJavaClassName> for Cow<'a, SmaliClassName> {
    fn from(value: &'a OwnedJavaClassName) -> Self {
        Cow::Owned(value.as_smali())
    }
}

impl<'a> From<&SmaliClassName> for Cow<'a, JavaClassName> {
    fn from(value: &SmaliClassName) -> Self {
        Cow::Owned(value.as_java())
    }
}

impl<'a> From<OwnedSmaliClassName> for Cow<'a, JavaClassName> {
    fn from(value: OwnedSmaliClassName) -> Self {
        Cow::Owned(value.as_java())
    }
}

impl<'a> From<&'a OwnedSmaliClassName> for Cow<'a, JavaClassName> {
    fn from(value: &'a OwnedSmaliClassName) -> Self {
        Cow::Owned(value.as_java())
    }
}

impl AsRef<JavaClassName> for JavaClassName {
    fn as_ref(&self) -> &JavaClassName {
        &self
    }
}

impl ToOwned for JavaClassName {
    type Owned = OwnedJavaClassName;

    fn to_owned(&self) -> OwnedJavaClassName {
        OwnedJavaClassName(self.0.to_string())
    }
}

impl Borrow<JavaClassName> for OwnedJavaClassName {
    fn borrow(&self) -> &JavaClassName {
        JavaClassName::new(&self.0)
    }
}

impl Deref for OwnedJavaClassName {
    type Target = JavaClassName;

    fn deref(&self) -> &JavaClassName {
        self.borrow()
    }
}

impl AsRef<str> for JavaClassName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for JavaClassName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for OwnedJavaClassName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<JavaClassName> for OwnedJavaClassName {
    fn as_ref(&self) -> &JavaClassName {
        self
    }
}

impl PartialEq<JavaClassName> for OwnedJavaClassName {
    fn eq(&self, other: &JavaClassName) -> bool {
        **self == *other
    }
}

impl<'a> From<&'a JavaClassName> for Cow<'a, JavaClassName> {
    fn from(value: &'a JavaClassName) -> Self {
        Cow::Borrowed(value)
    }
}

impl From<OwnedJavaClassName> for Cow<'_, JavaClassName> {
    fn from(value: OwnedJavaClassName) -> Self {
        Cow::Owned(value)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn from_raw_borrows_when_the_form_already_matches() {
        let smali = SmaliClassName::from_raw("Lfoo/bar/Baz;");
        assert!(matches!(smali, Cow::Borrowed(_)), "{smali:?}");
        assert_eq!(smali.as_str(), "Lfoo/bar/Baz;");

        let java = JavaClassName::from_raw("foo.bar.Baz");
        assert!(matches!(java, Cow::Borrowed(_)), "{java:?}");
        assert_eq!(java.as_str(), "foo.bar.Baz");
    }

    #[test]
    fn from_raw_converts_the_other_form() {
        let smali = SmaliClassName::from_raw("foo.bar.Baz");
        assert!(matches!(smali, Cow::Owned(_)), "{smali:?}");
        assert_eq!(smali.as_str(), "Lfoo/bar/Baz;");

        let java = JavaClassName::from_raw("Lfoo/bar/Baz;");
        assert!(matches!(java, Cow::Owned(_)), "{java:?}");
        assert_eq!(java.as_str(), "foo.bar.Baz");
    }

    #[test]
    fn conversions_round_trip() {
        for (smali, java) in [
            ("Lfoo/bar/Baz;", "foo.bar.Baz"),
            ("LBaz;", "Baz"),
            ("Lfoo/Bar$Inner;", "foo.Bar$Inner"),
        ] {
            let as_smali = SmaliClassName::new(smali);
            assert_eq!(as_smali.as_java().as_str(), java, "{smali} to java");
            assert_eq!(
                as_smali.as_java().as_smali().as_str(),
                smali,
                "{smali} round trip"
            );
        }
    }

    #[test]
    fn without_markers_drops_the_wrapping() {
        assert_eq!(
            SmaliClassName::new("Lfoo/bar/Baz;").without_markers(),
            "foo/bar/Baz"
        );
        assert_eq!(SmaliClassName::new("LBaz;").without_markers(), "Baz");
        assert_eq!(SmaliClassName::new("L;").without_markers(), "");
    }

    #[test]
    fn a_default_name_is_empty_rather_than_a_panic() {
        let name = <&SmaliClassName>::default();
        assert_eq!(name.as_str(), "");
        assert_eq!(name.without_markers(), "");
        assert_eq!(name.as_java().as_str(), "");
        assert_eq!(name.split_java_package(), (Cow::Borrowed(""), ""));
    }

    #[test]
    fn splitting_a_smali_package_borrows_what_it_can() {
        // A single segment package needs no dots put in, so it borrows
        let (pkg, class) = SmaliClassName::new("Lfoo/Baz;").split_java_package();
        assert!(matches!(pkg, Cow::Borrowed(_)), "{pkg:?}");
        assert_eq!((pkg.as_ref(), class), ("foo", "Baz"));

        // More than one segment has to be rewritten
        let (pkg, class) = SmaliClassName::new("Lfoo/bar/Baz;").split_java_package();
        assert!(matches!(pkg, Cow::Owned(_)), "{pkg:?}");
        assert_eq!((pkg.as_ref(), class), ("foo.bar", "Baz"));
    }

    #[test]
    fn splitting_a_package_handles_the_edges() {
        let no_pkg = SmaliClassName::new("LBaz;").split_java_package();
        assert_eq!(no_pkg, (Cow::Borrowed(""), "Baz"));

        let nested = SmaliClassName::new("Lfoo/Bar$Inner;").split_java_package();
        assert_eq!(nested, (Cow::Borrowed("foo"), "Bar$Inner"));

        let java = JavaClassName::from_raw("foo.bar.Baz");
        assert_eq!(java.split_java_package(), (Cow::Borrowed("foo.bar"), "Baz"));

        let java_no_pkg = JavaClassName::from_raw("Baz");
        assert_eq!(java_no_pkg.split_java_package(), (Cow::Borrowed(""), "Baz"));
    }

    #[test]
    fn the_trait_agrees_across_the_four_types() {
        let smali = SmaliClassName::new("Lfoo/bar/Baz;");
        let java = smali.as_java();
        let owned_smali = smali.to_owned();

        // The borrowed types are unsized, so this has to be generic rather than dyn
        fn check<C>(name: &C)
        where
            C: ClassName + ?Sized,
        {
            assert_eq!(name.as_smali_class_name().as_str(), "Lfoo/bar/Baz;");
            assert_eq!(name.as_java_class_name().as_str(), "foo.bar.Baz");
            let (pkg, class) = name.split_java_package();
            assert_eq!((pkg.as_ref(), class), ("foo.bar", "Baz"));
        }

        check(smali);
        check(&*java);
        check(&owned_smali);
        check(&java);
    }

    #[test]
    fn owning_a_name_and_borrowing_it_back_is_lossless() {
        let borrowed = SmaliClassName::new("Lfoo/bar/Baz;");
        let owned = borrowed.to_owned();
        assert_eq!(owned.as_str(), borrowed.as_str());
        assert_eq!(owned, *borrowed);
        assert_eq!(owned.to_string(), "Lfoo/bar/Baz;");

        let cow: Cow<'_, SmaliClassName> = owned.clone().into();
        assert_eq!(cow.as_str(), "Lfoo/bar/Baz;");

        let java = borrowed.as_java();
        assert_eq!(java.as_str(), "foo.bar.Baz");
        assert_eq!(java, *JavaClassName::from_raw("foo.bar.Baz"));
    }

    #[test]
    fn an_owned_name_can_be_looked_up_by_its_borrowed_form() {
        // Borrow requires the owned and borrowed forms to hash alike, otherwise
        // these lookups miss
        let mut smali: HashMap<OwnedSmaliClassName, u32> = HashMap::new();
        smali.insert(SmaliClassName::new("Lfoo/Bar;").to_owned(), 1);
        assert_eq!(smali.get(SmaliClassName::new("Lfoo/Bar;")), Some(&1));

        let mut java: HashMap<OwnedJavaClassName, u32> = HashMap::new();
        java.insert(SmaliClassName::new("Lfoo/Bar;").as_java(), 2);
        assert_eq!(java.get(&*JavaClassName::from_raw("foo.Bar")), Some(&2));
    }
}
