use crate::types::Tags;

/// Escapes a metric value and writes it to the string.
///
/// Replaces `\`, `|`, and `;` with their escaped versions.
pub fn write_escaped_value(buf: &mut String, value: &str) {
    for c in value.chars() {
        match c {
            '\\' => buf.push_str("\\\\"),
            '|' => buf.push_str("\\|"),
            ';' => buf.push_str("\\;"),
            _ => buf.push(c),
        }
    }
}

/// Escapes a tag name and writes it to the string.
///
/// Replaces `\` and `=` with their escaped versions.
pub fn write_escaped_tag_name(buf: &mut String, name: &str) {
    for c in name.chars() {
        match c {
            '\\' => buf.push_str("\\\\"),
            '=' => buf.push_str("\\="),
            _ => buf.push(c),
        }
    }
}

/// Serializes a metric name and its tags into the string.
///
/// This is a helper function used by various metric implementations to
/// ensure consistent serialization of names and tags.
pub fn serialize_name_and_tags<'a>(
    buf: &mut String,
    name: &str,
    default_tags: &Tags<'a>,
    tags: Option<&Tags<'a>>,
) {
    write_escaped_value(buf, name);

    if let Some(tags) = tags {
        for (name, value) in tags {
            buf.push(';');
            write_escaped_tag_name(buf, name);
            buf.push('=');
            write_escaped_value(buf, value);
        }
    }

    for (name, value) in default_tags {
        if tags.is_none_or(|t| !t.contains_key(name)) {
            buf.push(';');
            write_escaped_tag_name(buf, name);
            buf.push('=');
            write_escaped_value(buf, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_value() {
        let mut buff = String::new();
        write_escaped_value(&mut buff, "test");
        assert_eq!(buff, "test");

        let mut buff = String::new();
        write_escaped_value(&mut buff, "te;st");
        assert_eq!(buff, "te\\;st");

        let mut buff = String::new();
        write_escaped_value(&mut buff, "t|e;s\\t");
        assert_eq!(buff, "t\\|e\\;s\\\\t");
    }

    #[test]
    fn test_escape_tag_name() {
        let mut buff = String::new();
        write_escaped_tag_name(&mut buff, "test");
        assert_eq!(buff, "test");

        let mut buff = String::new();
        write_escaped_tag_name(&mut buff, "te\\st");
        assert_eq!(buff, "te\\\\st");

        let mut buff = String::new();
        write_escaped_tag_name(&mut buff, "te=st");
        assert_eq!(buff, "te\\=st");
    }

    #[test]
    fn test_serialize_name_and_tags() {
        let mut buff = String::new();
        serialize_name_and_tags(&mut buff, "test", &Tags::default(), None);
        assert_eq!("test", buff);

        let mut default_tags = Tags::new();
        default_tags.insert("k1".into(), "v1".into());
        let mut buff = String::new();
        serialize_name_and_tags(&mut buff, "test", &default_tags, None);
        assert_eq!("test;k1=v1", buff);

        let mut metric_tags = Tags::new();
        metric_tags.insert("k2".into(), "v2".into());
        let mut buff = String::new();
        serialize_name_and_tags(&mut buff, "test", &default_tags, Some(&metric_tags));
        assert!(buff == "test;k2=v2;k1=v1" || buff == "test;k1=v1;k2=v2");

        // Metric tag overrides default tag
        let mut override_tags = Tags::new();
        override_tags.insert("k1".into(), "v1_new".into());
        let mut buff = String::new();
        serialize_name_and_tags(&mut buff, "test", &default_tags, Some(&override_tags));
        assert_eq!("test;k1=v1_new", buff);
    }

    #[test]
    fn test_serialize_with_escaping() {
        let mut metric_tags = Tags::new();
        metric_tags.insert("k1".into(), "v|1".into());

        let mut buff = String::new();
        serialize_name_and_tags(&mut buff, "t|est", &Tags::default(), Some(&metric_tags));
        assert_eq!("t\\|est;k1=v\\|1", buff);
    }
}
