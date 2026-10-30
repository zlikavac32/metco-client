use std::borrow::Cow;
use std::collections::BTreeMap;

/// Type alias for metric tags.
///
/// Tags are used to add metadata to metrics. They are represented as a
/// `BTreeMap`, so they are always serialized sorted by name.
pub type Tags<'a> = BTreeMap<Cow<'a, str>, Cow<'a, str>>;
