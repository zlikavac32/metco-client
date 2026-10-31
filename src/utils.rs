use crate::types::Tags;

/// Escapes a metric value and writes it to the string.
///
/// Replaces `\`, `|`, and `;` with their escaped versions.
pub fn write_escaped_value(buf: &mut String, value: &str) {
    write_escaped(buf, value, &['\\', '|', ';']);
}

/// Escapes a tag name and writes it to the string.
///
/// Replaces `\` and `=` with their escaped versions. Unlike values, `|` and
/// `;` are valid unescaped in tag names, and MetCo rejects `\|` and `\;` there.
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
/// `default_tags`. Tags are written sorted by name, so the output is
/// deterministic.
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

    // Both maps iterate in name order, so merging them keeps the output sorted.
    let mut defaults = default_tags.iter().peekable();

    for (name, value) in tags.into_iter().flatten() {
        while let Some((default_name, default_value)) = defaults.next_if(|(d, _)| *d < name) {
            write_tag(buf, default_name, default_value);
        }

        // Skip the default tag overridden by this one.
        defaults.next_if(|(d, _)| *d == name);

        write_tag(buf, name, value);
    }

    for (name, value) in defaults {
        write_tag(buf, name, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::tags;

    fn escaped(f: fn(&mut String, &str), input: &str) -> String {
        let mut buf = String::new();
        f(&mut buf, input);
        buf
    }

    fn serialized(name: &str, default_tags: &Tags<'_>, metric_tags: Option<&Tags<'_>>) -> String {
        let mut buf = String::new();
        serialize_name_and_tags(&mut buf, name, default_tags, metric_tags);
        buf
    }

    #[test]
    fn test_escape_value() {
        assert_eq!(escaped(write_escaped_value, "test"), "test");
        assert_eq!(escaped(write_escaped_value, "te;st"), "te\\;st");
        assert_eq!(escaped(write_escaped_value, "t|e;s\\t"), "t\\|e\\;s\\\\t");
    }

    #[test]
    fn test_escape_value_edge_cases() {
        assert_eq!(escaped(write_escaped_value, ""), "");
        assert_eq!(escaped(write_escaped_value, "|"), "\\|");
        assert_eq!(escaped(write_escaped_value, ";;"), "\\;\\;");
        assert_eq!(escaped(write_escaped_value, "\\\\"), "\\\\\\\\");
        assert_eq!(escaped(write_escaped_value, "|x|"), "\\|x\\|");
        // `=` is only special in tag names.
        assert_eq!(escaped(write_escaped_value, "a=b"), "a=b");
        assert_eq!(escaped(write_escaped_value, "čćž→🦀"), "čćž→🦀");
        assert_eq!(escaped(write_escaped_value, "a b\tc\n"), "a b\tc\n");
    }

    #[test]
    fn test_escape_tag_name() {
        assert_eq!(escaped(write_escaped_tag_name, "test"), "test");
        assert_eq!(escaped(write_escaped_tag_name, "te\\st"), "te\\\\st");
        assert_eq!(escaped(write_escaped_tag_name, "te=st"), "te\\=st");
    }

    #[test]
    fn test_escape_tag_name_edge_cases() {
        assert_eq!(escaped(write_escaped_tag_name, ""), "");
        assert_eq!(escaped(write_escaped_tag_name, "=="), "\\=\\=");
        assert_eq!(escaped(write_escaped_tag_name, "\\="), "\\\\\\=");
        assert_eq!(escaped(write_escaped_tag_name, "čćž🦀"), "čćž🦀");
    }

    #[test]
    fn test_escape_tag_name_keeps_value_separators() {
        assert_eq!(escaped(write_escaped_tag_name, "a;b|c"), "a;b|c");
    }

    #[test]
    fn test_serialize_tag_name_with_value_separators() {
        let metric_tags = tags(&[("a;b|c", "v")]);

        assert_eq!(
            "test;a;b|c=v",
            serialized("test", &Tags::new(), Some(&metric_tags))
        );
    }

    #[test]
    fn test_escaping_appends_to_existing_buffer() {
        let mut buf = String::from("prefix:");
        write_escaped_value(&mut buf, "a|b");
        write_escaped_tag_name(&mut buf, "c=d");
        assert_eq!(buf, "prefix:a\\|bc\\=d");
    }

    #[test]
    fn test_serialize_name_and_tags() {
        assert_eq!("test", serialized("test", &Tags::new(), None));

        let default_tags = tags(&[("k1", "v1")]);
        assert_eq!("test;k1=v1", serialized("test", &default_tags, None));

        let metric_tags = tags(&[("k2", "v2")]);
        assert_eq!(
            "test;k1=v1;k2=v2",
            serialized("test", &default_tags, Some(&metric_tags))
        );

        // Metric tag overrides default tag
        let override_tags = tags(&[("k1", "v1_new")]);
        assert_eq!(
            "test;k1=v1_new",
            serialized("test", &default_tags, Some(&override_tags))
        );
    }

    #[test]
    fn test_serialize_with_empty_metric_tags() {
        assert_eq!("test", serialized("test", &Tags::new(), Some(&Tags::new())));
        assert_eq!(
            "test;k1=v1",
            serialized("test", &tags(&[("k1", "v1")]), Some(&Tags::new()))
        );
    }

    #[test]
    fn test_serialize_only_metric_tags() {
        assert_eq!(
            "test;a=1;b=2",
            serialized("test", &Tags::new(), Some(&tags(&[("b", "2"), ("a", "1")])))
        );
    }

    #[test]
    fn test_serialize_tags_sorted_by_name() {
        let default_tags = tags(&[("b", "1"), ("d", "2")]);
        let metric_tags = tags(&[("c", "3"), ("a", "4"), ("d", "5")]);

        assert_eq!(
            "test;a=4;b=1;c=3;d=5",
            serialized("test", &default_tags, Some(&metric_tags))
        );
    }

    #[test]
    fn test_serialize_default_tags_before_and_after_metric_tags() {
        let default_tags = tags(&[("a", "1"), ("m", "2"), ("z", "3")]);
        let metric_tags = tags(&[("k", "4"), ("n", "5")]);

        assert_eq!(
            "test;a=1;k=4;m=2;n=5;z=3",
            serialized("test", &default_tags, Some(&metric_tags))
        );
    }

    #[test]
    fn test_serialize_all_default_tags_overridden() {
        let default_tags = tags(&[("a", "1"), ("b", "2")]);
        let metric_tags = tags(&[("a", "3"), ("b", "4")]);

        assert_eq!(
            "test;a=3;b=4",
            serialized("test", &default_tags, Some(&metric_tags))
        );
    }

    #[test]
    fn test_serialize_tags_sorted_by_unescaped_name() {
        // `=` sorts before `a`, and the order is decided before escaping.
        let metric_tags = tags(&[("a", "1"), ("=", "2")]);

        assert_eq!(
            "test;\\==2;a=1",
            serialized("test", &Tags::new(), Some(&metric_tags))
        );
    }

    #[test]
    fn test_serialize_with_escaping() {
        let metric_tags = tags(&[("k1", "v|1")]);

        assert_eq!(
            "t\\|est;k1=v\\|1",
            serialized("t|est", &Tags::new(), Some(&metric_tags))
        );
    }

    #[test]
    fn test_serialize_escapes_default_tags() {
        let default_tags = tags(&[("k=1", "v;1")]);

        assert_eq!("test;k\\=1=v\\;1", serialized("test", &default_tags, None));
    }
}
