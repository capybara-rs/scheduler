pub enum Task {
    Http(http::Task),
}

pub mod http {
    use reqwest::Url;
    use serde::de::{Error, Visitor};
    use serde::Deserialize;
    use std::collections::HashMap;
    use std::fmt;

    pub enum Method {
        GET,
        POST,
        DELETE,
        PUT,
    }

    struct MethodVisitor;

    impl<'de> Visitor<'de> for MethodVisitor {
        type Value = Method;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "expected one of [GET, POST, PUT, DELETE]")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            use Method::*;

            match value {
                "GET" => Ok(GET),
                "POST" => Ok(POST),
                "PUT" => Ok(PUT),
                "DELETE" => Ok(DELETE),
                value => {
                    let msg = format!("invalid method value: {}", value);

                    Err(Error::custom(msg))
                }
            }
        }
    }

    impl<'de> Deserialize<'de> for Method {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_str(MethodVisitor)
        }
    }

    #[derive(Deserialize)]
    pub struct Task {
        name: String,
        method: Method,
        url: Url,
        #[serde(default)]
        headers: HashMap<String, String>,
        #[serde(default)]
        success_status_codes: Vec<u16>,
        body: Option<serde_yml::Value>,
    }
}
