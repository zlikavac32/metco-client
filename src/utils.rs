use crate::types::Tags;

/// Escapes a metric value and writes it to the string.
///
/// Replaces `\`, `|`, and `;` with their escaped versions.
pub fn write_escaped_value(buf: &mut String, value: &str) {
    write_escaped(buf, value, &['\\', '|', ';']);
}

/// Escapes a tag name and writes it to the string.
///
/// Replaces `\` and `=` with their escaped versions.
pub fn write_escaped_tag_name(buf: &mut String, name: &str) {
    write_escaped(buf, name, &['\\', '=']);
}

fn write_escaped(buf: &mut String, input: &str, special: &[char]) {
    buf.reserve(input.len());

    for c in input.chars() {
        if special.contains(&c) {
            buf.push('\\');
        }
        buf.push(c);
    }
}

fn write_tag(buf: &mut String, name: &str, value: &str) {
    buf.push(';');
    write_escaped_tag_name(buf, name);
    buf.push('=');
    write_escaped_value(buf, value);
}

/// Serializes a metric name and its tags into the string.
///
/// Tags from `tags` take precedence over tags with the same name in
/// `default_tags`.
///
/// This is a helper function used by various metric implementations to
/// ensure consistent serialization of names and tags.
pub fn serialize_name_and_tags(
    buf: &mut String,
    name: &str,
    default_tags: &Tags<'_>,
    tags: Option<&Tags<'_>>,
) {
    write_escaped_value(buf, name);

    for (name, value) in tags.into_iter().flatten() {
        write_tag(buf, name, value);
    }

    for (name, value) in default_tags {
        if tags.is_none_or(|t| !t.contains_key(name)) {
            write_tag(buf, name, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn escaped(f: fn(&mut String, &str), input: &str) -> String {
        let mut buf = String::new();
        f(&mut buf, input);
        buf
    }

    #[test]
    fn test_escape_value() {
        assert_eq!(escaped(write_escaped_value, "test"), "test");
        assert_eq!(escaped(write_escaped_value, "te;st"), "te\\;st");
        assert_eq!(escaped(write_escaped_value, "t|e;s\\t"), "t\\|e\\;s\\\\t");
    }

    #[test]
    fn test_escape_tag_name() {
        assert_eq!(escaped(write_escaped_tag_name, "test"), "test");
        assert_eq!(escaped(write_escaped_tag_name, "te\\st"), "te\\\\st");
        assert_eq!(escaped(write_escaped_tag_name, "te=st"), "te\\=st");
    }

    #[test]
    fn test_serialize_name_and_tags() {
        let mut buf = String::new();
        serialize_name_and_tags(&mut buf, "test", &Tags::default(), None);
        assert_eq!("test", buf);

        let default_tags = Tags::from([("k1".into(), "v1".into())]);
        let mut buf = String::new();
        serialize_name_and_tags(&mut buf, "test", &default_tags, None);
        assert_eq!("test;k1=v1", buf);

        let metric_tags = Tags::from([("k2".into(), "v2".into())]);
        let mut buf = String::new();
        serialize_name_and_tags(&mut buf, "test", &default_tags, Some(&metric_tags));
        assert!(buf == "test;k2=v2;k1=v1" || buf == "test;k1=v1;k2=v2");

        // Metric tag overrides default tag
        let override_tags = Tags::from([("k1".into(), "v1_new".into())]);
        let mut buf = String::new();
        serialize_name_and_tags(&mut buf, "test", &default_tags, Some(&override_tags));
        assert_eq!("test;k1=v1_new", buf);
    }

    #[test]
    fn test_serialize_with_escaping() {
        let metric_tags = Tags::from([("k1".into(), "v|1".into())]);

        let mut buf = String::new();
        serialize_name_and_tags(&mut buf, "t|est", &Tags::default(), Some(&metric_tags));
        assert_eq!("t\\|est;k1=v\\|1", buf);
    }
}
