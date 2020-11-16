use serde::Serialize;
#[derive(Default, Debug, Clone, Serialize)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub last_cursor: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename = "list")]
pub struct List<T: TypeTag + Serialize> {
    pub page_info: PageInfo,
    #[serde(with = "WithType")]
    pub items: Vec<T>,
}

pub trait TypeTag {
    const TYPE: &'static str;
}

#[derive(Debug, Clone, Serialize)]
#[serde(remote = "T")]
struct WithType<T: TypeTag> {
    #[serde(rename = "type")]
    type_tag: String,
    #[serde(flatten)]
    t: T,
}

// struct WithType<T: TypeTag>(T)

// impl<T: TypeTag> WithType<T> {
//     pub fn new(t: T) -> Self {
//         Self {
//             type_tag: T::TYPE.to_owned(),
//             t: t,
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct Foo {
        foo: u16,
    }

    impl TypeTag for Foo {
        const TYPE: &'static str = "foo_type";
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

        assert_eq!(serde_json::to_string(&list).unwrap(), "{\"type\":\"list\",\"page_info\":{\"has_next_page\":false,\"last_cursor\":\"last_foo\"},\"items\":[{\"type\":\"foo_type\",\"foo\":0}]}");
    }
}
