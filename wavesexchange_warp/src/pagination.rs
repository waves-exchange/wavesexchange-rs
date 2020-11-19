use serde::Serialize;
#[derive(Debug, Clone, Serialize)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub last_cursor: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename = "list")]
pub struct List<T: Serialize> {
    pub page_info: PageInfo,
    pub items: Vec<T>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    #[serde(tag = "type", rename = "foo")]
    struct Foo {
        foo: u16,
    }

    #[test]
    fn data_serialization() {
        let page_info = PageInfo {
            has_next_page: false,
            last_cursor: "last_foo".to_owned(),
        };

        let items = vec![Foo { foo: 0 }];

        let list = List {
            page_info: page_info,
            items: items,
        };

        assert_eq!(serde_json::to_string(&list).unwrap(), "{\"type\":\"list\",\"page_info\":{\"has_next_page\":false,\"last_cursor\":\"last_foo\"},\"items\":[{\"type\":\"foo\",\"foo\":0}]}");
    }
}
