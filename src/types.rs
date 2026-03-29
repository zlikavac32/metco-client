use std::borrow::Cow;
use std::collections::HashMap;

/// Type alias for metric tags.
///
/// Tags are used to add metadata to metrics. They are represented as a `HashMap`.
pub type Tags<'a> = HashMap<Cow<'a, str>, Cow<'a, str>>;
