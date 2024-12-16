use crate::config::value;
use reqwest::Url;
use serde::de::{Error, Visitor};
use serde::Deserialize;
use serde_yml::Mapping;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum Method {
    Get,
    Post,
    Delete,
    Put,
    Patch,
}

struct MethodVisitor;

impl<'de> Visitor<'de> for MethodVisitor {
    type Value = Method;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "expected one of [GET, POST, PUT, DELETE, PATCH]")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        use Method::*;

        match value {
            "GET" => Ok(Get),
            "POST" => Ok(Post),
            "PUT" => Ok(Put),
            "DELETE" => Ok(Delete),
            "PATCH" => Ok(Patch),
            value => Err(Error::unknown_variant(
                value,
                &["GET", "POST", "PUT", "DELETE", "PATCH"],
            )),
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

#[derive(Deserialize, Debug, PartialEq)]
pub struct Task {
    name: String,
    method: Method,
    url: Url,
    #[serde(default)]
    headers: Headers,
    #[serde(default)]
    success_status_codes: Vec<u16>,
    body: Option<Body>,
}

#[derive(Debug, PartialEq)]
pub struct Headers(HashMap<String, value::Value>);

impl Default for Headers {
    fn default() -> Self {
        Self(HashMap::default())
    }
}

pub struct HeadersVisitor;

impl<'de> Visitor<'de> for HeadersVisitor {
    type Value = Headers;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "headers key: yaml entry")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut headers: HashMap<String, value::Value> = HashMap::new();

        loop {
            let entry: Option<(String, Mapping)> = map.next_entry()?;

            if let Some((key, entry)) = entry {
                let value = value::Value::from_basic_entry(&entry);

                match value {
                    Ok(value) => {
                        headers.insert(key, value);
                    }
                    Err(err) => return Err(err.to_de_error()),
                }
            } else {
                break;
            }
        }

        Ok(Headers(headers))
    }
}

impl<'de> Deserialize<'de> for Headers {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(HeadersVisitor)
    }
}

#[derive(Debug, PartialEq)]
pub enum Body {
    Json(value::Value),
}

struct BodyVisitor;

impl<'de> Visitor<'de> for BodyVisitor {
    type Value = Body;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "yaml entry")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let entry: (String, serde_yml::Mapping) = map
            .next_entry()?
            .ok_or(Error::custom("invalid body field"))?;

        let content_type = entry.0.as_str();

        return match content_type {
            "json" => {
                let value = value::Value::from_entry(&entry.1);

                match value {
                    Ok(json_value) => Ok(Body::Json(json_value)),
                    Err(err) => Err(err.to_de_error()),
                }
            }
            value => Err(Error::unknown_field(value, &["json"])),
        };
    }
}

impl<'de> Deserialize<'de> for Body {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(BodyVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::source::Source;
    use crate::config::value::Value;
    use serde_yml;

    #[test]
    fn test_deserialize_method() {
        success_deserialize_method("GET", Method::Get);
        success_deserialize_method("POST", Method::Post);
        success_deserialize_method("DELETE", Method::Delete);
        success_deserialize_method("PATCH", Method::Patch);
        success_deserialize_method("PUT", Method::Put);

        failure_deserialize_method("get");
        failure_deserialize_method("post");
        failure_deserialize_method("delete");
        failure_deserialize_method("patch");
        failure_deserialize_method("put");
    }

    fn success_deserialize_method(input: &str, expected: Method) {
        let method: Method = serde_yml::from_str(input).unwrap();

        assert_eq!(expected, method);
    }

    fn failure_deserialize_method(input: &str) {
        let is_err = serde_yml::from_str::<Method>(input)
            .err()
            .is_some();

        assert!(is_err)
    }

    #[test]
    fn test_deserialize_task() {
        let headers = HashMap::from_iter([
            (
                String::from("X-Api-Key"),
                Value::String(String::from("env!(YOUR_OWN_SERVICE_KEY)")),
            ),
            (
                String::from("X-Custom-Key"),
                Value::String(String::from("My Custom Key")),
            ),
            (
                String::from("X-Last-Execute-Time"),
                Value::Source(Source::LastExecuteDate),
            ),
            (
                String::from("X-Execute-Time"),
                Value::Source(Source::ExecuteDate),
            ),
        ]);

        let json_value = Value::Object(HashMap::from_iter([
            (String::from("field1"), Value::String(String::from("hello"))),
            (
                String::from("field2"),
                Value::Object(HashMap::from_iter([(
                    String::from("field1_1"),
                    Value::Integer(100),
                )])),
            ),
            (
                String::from("field3"),
                Value::Array(vec![
                    Value::Object(HashMap::from_iter([(
                        String::from("field1"),
                        Value::Bool(false),
                    )])),
                    Value::Bool(true),
                ]),
            ),
            (String::from("field4"), Value::Null),
            (
                String::from("last_execute_time"),
                Value::Source(Source::LastExecuteDate),
            ),
            (
                String::from("execute_time"),
                Value::Source(Source::ExecuteDate),
            ),
        ]));

        let body = Body::Json(json_value);
        success_deserialize_task(
            "
          type: http # required
          name: load_data # required
          method: GET # required
          url: http://localhost:3030/load # required
          headers: # optional, default is empty
            X-Api-Key:
              type: string
              value: env!(YOUR_OWN_SERVICE_KEY)
            X-Custom-Key:
              type: string
              value: \"My Custom Key\"
            X-Last-Execute-Time:
              type: source
              source: last_execute_time
            X-Execute-Time:
              type: source
              source: execute_time
          success_status_codes: # optional, default is 200
            - 200
          body: # optional
            json:
              type: object
              properties:
                field1:
                  type: string
                  value: hello
                field2:
                  type: object
                  properties:
                    field1_1:
                      type: integer
                      value: 100
                field3:
                  type: array
                  items:
                    - type: object
                      properties:
                        field1:
                          type: boolean
                          value: false
                    - type: boolean
                      value: TRUE
                field4:
                  type: \"null\"
                last_execute_time:
                  type: source
                  source: last_execute_time # this add string field with date in RFC3339
                execute_time:
                  type: source
                  source: execute_time # this add string field with date in RFC3339",
            Task {
                name: String::from("load_data"),
                method: Method::Get,
                url: Url::parse("http://localhost:3030/load").unwrap(),
                headers: Headers(headers),
                success_status_codes: vec![200],
                body: Some(body),
            },
        );
    }

    fn success_deserialize_task(input: &str, expected: Task) {
        let task: Task = serde_yml::from_str(input).unwrap();

        assert_eq!(expected, task);
    }
}
