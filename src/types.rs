use std::borrow::{Borrow, Cow};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::ops::Deref;

/// Type alias for metric tags.
///
/// Tags are used to add metadata to metrics. They are represented as a
/// `BTreeMap`, so they are always serialized sorted by name.
pub type Tags<'a> = BTreeMap<NonEmptyStr<'a>, NonEmptyStr<'a>>;

/// A string that is guaranteed to be non-empty.
///
/// MetCo rejects metrics with an empty name, tag name or tag value, so those
/// are represented by this type. It borrows or owns its contents like a
/// [`Cow<str>`].
///
/// # Examples
///
/// ```rust
/// use metco_client::{NonEmptyStr, non_empty};
///
/// // String literals are checked at compile time.
/// let name = non_empty!("requests");
///
/// // Other strings are checked at runtime.
/// let name = NonEmptyStr::new(format!("requests_{}", 1)).unwrap();
/// assert!(NonEmptyStr::new("").is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NonEmptyStr<'a>(Cow<'a, str>);

impl<'a> NonEmptyStr<'a> {
    /// Creates a non-empty string, failing if `value` is empty.
    ///
    /// # Errors
    ///
    /// Returns [`EmptyStrError`] if `value` is empty.
    pub fn new<S: Into<Cow<'a, str>>>(value: S) -> Result<Self, EmptyStrError> {
        let value = value.into();

        if value.is_empty() {
            Err(EmptyStrError)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the contents as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns a non-empty string borrowing from this one.
    #[must_use]
    pub fn borrowed(&self) -> NonEmptyStr<'_> {
        NonEmptyStr(Cow::Borrowed(&self.0))
    }

    /// Converts into a non-empty string that owns its contents.
    #[must_use]
    pub fn into_owned(self) -> NonEmptyStr<'static> {
        NonEmptyStr(Cow::Owned(self.0.into_owned()))
    }

    /// Returns the underlying [`Cow<str>`].
    #[must_use]
    pub fn into_inner(self) -> Cow<'a, str> {
        self.0
    }
}

impl NonEmptyStr<'static> {
    /// Creates a non-empty string from a static string.
    ///
    /// Prefer the [`non_empty!`](crate::non_empty) macro, which runs this
    /// check at compile time.
    ///
    /// # Panics
    ///
    /// Panics if `value` is empty.
    #[must_use]
    pub const fn from_static(value: &'static str) -> Self {
        assert!(!value.is_empty(), "string must not be empty");

        Self(Cow::Borrowed(value))
    }
}

/// Creates a [`NonEmptyStr`] from a string literal, failing to compile if it
/// is empty.
///
/// # Examples
///
/// ```rust
/// use metco_client::non_empty;
///
/// let name = non_empty!("requests");
/// assert_eq!(name, "requests");
/// ```
///
/// ```compile_fail
/// let name = metco_client::non_empty!("");
/// ```
#[macro_export]
macro_rules! non_empty {
    ($value:literal) => {
        const { $crate::types::NonEmptyStr::from_static($value) }
    };
}

impl Deref for NonEmptyStr<'_> {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for NonEmptyStr<'_> {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for NonEmptyStr<'_> {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NonEmptyStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl PartialEq<str> for NonEmptyStr<'_> {
    fn eq(&self, other: &str) -> bool {
        *self.0 == *other
    }
}

impl PartialEq<&str> for NonEmptyStr<'_> {
    fn eq(&self, other: &&str) -> bool {
        *self.0 == **other
    }
}

impl<'a> TryFrom<&'a str> for NonEmptyStr<'a> {
    type Error = EmptyStrError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for NonEmptyStr<'_> {
    type Error = EmptyStrError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'a> TryFrom<Cow<'a, str>> for NonEmptyStr<'a> {
    type Error = EmptyStrError;

    fn try_from(value: Cow<'a, str>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'a> From<NonEmptyStr<'a>> for Cow<'a, str> {
    fn from(value: NonEmptyStr<'a>) -> Self {
        value.0
    }
}

impl From<NonEmptyStr<'_>> for String {
    fn from(value: NonEmptyStr<'_>) -> Self {
        value.0.into_owned()
    }
}

/// The error returned when creating a [`NonEmptyStr`] from an empty string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmptyStrError;

impl fmt::Display for EmptyStrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("string must not be empty")
    }
}

impl Error for EmptyStrError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_rejects_empty_strings() {
        assert_eq!(NonEmptyStr::new(""), Err(EmptyStrError));
        assert_eq!(NonEmptyStr::new(String::new()), Err(EmptyStrError));
        assert_eq!(NonEmptyStr::new(Cow::Borrowed("")), Err(EmptyStrError));
    }

    #[test]
    fn test_new_accepts_non_empty_strings() {
        assert_eq!(NonEmptyStr::new("a").unwrap(), "a");
        assert_eq!(NonEmptyStr::new(String::from("b")).unwrap(), "b");
        assert_eq!(NonEmptyStr::new(" ").unwrap(), " ");
        assert_eq!(NonEmptyStr::new("🦀").unwrap(), "🦀");
    }

    #[test]
    fn test_try_from() {
        assert_eq!(NonEmptyStr::try_from("").unwrap_err(), EmptyStrError);
        assert_eq!(
            NonEmptyStr::try_from(String::new()).unwrap_err(),
            EmptyStrError
        );
        assert_eq!(
            NonEmptyStr::try_from(Cow::Borrowed("")).unwrap_err(),
            EmptyStrError
        );

        assert_eq!(NonEmptyStr::try_from("a").unwrap(), "a");
        assert_eq!(NonEmptyStr::try_from(String::from("a")).unwrap(), "a");
        assert_eq!(NonEmptyStr::try_from(Cow::Borrowed("a")).unwrap(), "a");
    }

    #[test]
    fn test_from_static() {
        assert_eq!(NonEmptyStr::from_static("a"), "a");
        assert_eq!(non_empty!("a"), "a");
    }

    #[test]
    #[should_panic(expected = "string must not be empty")]
    fn test_from_static_panics_on_empty_string() {
        let empty = String::new().leak();
        let _ = NonEmptyStr::from_static(empty);
    }

    #[test]
    fn test_conversions() {
        let value = NonEmptyStr::new(String::from("abc")).unwrap();

        assert_eq!(value.as_str(), "abc");
        assert_eq!(&*value, "abc");
        assert_eq!(value.len(), 3);
        assert_eq!(value.to_string(), "abc");
        assert_eq!(value.borrowed(), value);
        assert!(matches!(
            value.borrowed().into_inner(),
            Cow::Borrowed("abc")
        ));
        assert_eq!(value.clone().into_owned(), value);
        assert_eq!(Cow::from(value.clone()), "abc");
        assert_eq!(String::from(value), "abc");
    }

    #[test]
    fn test_ordering_matches_str() {
        let mut values = [non_empty!("b"), non_empty!("="), non_empty!("a")];
        values.sort();

        assert_eq!(values, ["=", "a", "b"]);
    }

    #[test]
    fn test_tags_can_be_looked_up_by_str() {
        let tags = Tags::from([(non_empty!("k"), non_empty!("v"))]);

        assert_eq!(tags.get("k").unwrap(), "v");
        assert!(tags.contains_key("k"));
        assert!(!tags.contains_key("v"));
    }

    #[test]
    fn test_error_message() {
        assert_eq!(EmptyStrError.to_string(), "string must not be empty");
    }
}
