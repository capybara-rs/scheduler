use crate::config::source;

use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub enum Value {
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
    String(String),
    Bool(bool),
    Float(f64),
    Integer(i64),
    Null,

    Source(source::Source),
}

#[derive(Debug)]
pub enum ParseEntryError {
    MissingField(&'static str),
    InvalidType,
    InvalidValue,
    InvalidItems,
    InvalidProperties,
    InvalidSource,
    InvalidSourceValue(String),
    InvalidTypeValue(String),
}
use serde::de::Error;

impl ParseEntryError {
    pub fn to_de_error<E>(&self) -> E
    where
        E: serde::de::Error,
    {
        match self {
            ParseEntryError::MissingField(field) => Error::missing_field(field),
            ParseEntryError::InvalidType => Error::custom("invalid 'type' tag"),
            ParseEntryError::InvalidValue => Error::custom("invalid 'value' tag"),
            ParseEntryError::InvalidItems => {
                Error::custom("invaid 'items' tag, should be a sequence")
            }
            ParseEntryError::InvalidProperties => {
                Error::custom("invalid 'properties' tag, shourld be map")
            }
            ParseEntryError::InvalidSource => Error::custom("invalid 'source' tag"),
            ParseEntryError::InvalidSourceValue(source) => {
                Error::unknown_variant(source.as_str(), &["execute_time", "last_execute_time"])
            }
            ParseEntryError::InvalidTypeValue(entry_type) => Error::unknown_variant(
                entry_type.as_str(),
                &[
                    "array", "object", "integer", "float", "string", "boolean", "null", "source",
                ],
            ),
        }
    }
}

impl Value {
    const TYPE_TAG: &str = "type";
    const PROPERTIES_TAG: &str = "properties";
    const ITEMS_TAG: &str = "items";
    const VALUE_TAG: &str = "value";
    const SOURCE_TAG: &str = "source";

    pub fn from_entry(entry: &serde_yml::Mapping) -> Result<Self, ParseEntryError> {
        println!("{:?}", entry);
        let entry_type = Self::get_type(&entry)?;

        Self::parse_entry_by_type(entry_type, &entry)
    }

    pub fn from_basic_entry(entry: &serde_yml::Mapping) -> Result<Self, ParseEntryError> {
        let entry_type = Self::get_type(&entry)?;

        Self::parse_basic_entry_by_type(entry_type, &entry)
    }

    fn get_type<'a>(entry: &'a serde_yml::Mapping) -> Result<&'a str, ParseEntryError> {
        let type_tag = serde_yml::Value::String(String::from(Self::TYPE_TAG));

        let entry_type = entry
            .get(&type_tag)
            .ok_or(ParseEntryError::MissingField(Self::TYPE_TAG))?;

        let entry_type = entry_type
            .as_str()
            .ok_or(ParseEntryError::InvalidType)?;

        Ok(entry_type)
    }

    fn get_items<'a>(
        entry: &'a serde_yml::Mapping,
    ) -> Result<&'a Vec<serde_yml::Value>, ParseEntryError> {
        let items_tag = serde_yml::Value::String(String::from(Self::ITEMS_TAG));

        let items = entry
            .get(&items_tag)
            .ok_or(ParseEntryError::MissingField(Self::ITEMS_TAG))?;

        let sequence = items
            .as_sequence()
            .ok_or(ParseEntryError::InvalidItems)?;

        Ok(sequence)
    }

    fn get_properties<'a>(
        entry: &'a serde_yml::Mapping,
    ) -> Result<&'a serde_yml::Mapping, ParseEntryError> {
        let properties_tag = serde_yml::Value::String(String::from(Self::PROPERTIES_TAG));

        let properties_tag_value = entry
            .get(&properties_tag)
            .ok_or(ParseEntryError::MissingField(Self::PROPERTIES_TAG))?;

        let mapping = properties_tag_value
            .as_mapping()
            .ok_or(ParseEntryError::InvalidProperties)?;

        Ok(mapping)
    }

    fn get_value<'a>(
        entry: &'a serde_yml::Mapping,
    ) -> Result<&'a serde_yml::Value, ParseEntryError> {
        let value_tag = serde_yml::Value::String(String::from(Self::VALUE_TAG));

        let value = entry
            .get(&value_tag)
            .ok_or(ParseEntryError::MissingField(Self::VALUE_TAG))?;

        Ok(value)
    }

    fn get_bool(entry: &serde_yml::Mapping) -> Result<Self, ParseEntryError> {
        let value = Self::get_value(entry)?;

        let value = value
            .as_bool()
            .ok_or(ParseEntryError::InvalidValue)?;

        Ok(Value::Bool(value))
    }

    fn get_float(entry: &serde_yml::Mapping) -> Result<Self, ParseEntryError> {
        let value = Self::get_value(entry)?;

        match value {
            serde_yml::Value::Number(value) => {
                let float_value = value
                    .as_f64()
                    .ok_or(ParseEntryError::InvalidValue)?;

                Ok(Value::Float(float_value))
            }
            _ => Err(ParseEntryError::InvalidValue),
        }
    }

    fn get_integer(entry: &serde_yml::Mapping) -> Result<Self, ParseEntryError> {
        let value = Self::get_value(entry)?;

        match value {
            serde_yml::Value::Number(value) => {
                let integer_value = value
                    .as_i64()
                    .ok_or(ParseEntryError::InvalidValue)?;

                Ok(Value::Integer(integer_value))
            }
            _ => Err(ParseEntryError::InvalidValue),
        }
    }

    fn get_string(entry: &serde_yml::Mapping) -> Result<Self, ParseEntryError> {
        let value = Self::get_value(entry)?;

        match value {
            serde_yml::Value::String(value) => Ok(Value::String(value.clone())),
            _ => Err(ParseEntryError::InvalidValue),
        }
    }

    fn get_object(entry: &serde_yml::Mapping) -> Result<Self, ParseEntryError> {
        let properties = Self::get_properties(entry)?;
        let mut object: HashMap<String, Value> = HashMap::with_capacity(properties.len());

        for (key, value) in properties {
            let key = key
                .as_str()
                .ok_or(ParseEntryError::InvalidProperties)?;

            let entry = value
                .as_mapping()
                .ok_or(ParseEntryError::InvalidProperties)?;

            let json_value = Self::from_entry(entry)?;

            _ = object.insert(String::from(key), json_value);
        }

        Ok(Value::Object(object))
    }

    fn get_array(entry: &serde_yml::Mapping) -> Result<Self, ParseEntryError> {
        let items = Self::get_items(entry)?;
        let mut array = Vec::with_capacity(items.len());

        for value in items {
            let entry = value
                .as_mapping()
                .ok_or(ParseEntryError::InvalidItems)?;

            let json_value = Self::from_entry(entry)?;

            array.push(json_value);
        }

        Ok(Value::Array(array))
    }

    fn get_source(entry: &serde_yml::Mapping) -> Result<Self, ParseEntryError> {
        let source_tag = serde_yml::Value::String(String::from(Self::SOURCE_TAG));

        let source = entry
            .get(&source_tag)
            .ok_or(ParseEntryError::MissingField(Self::SOURCE_TAG))?;

        let source = source
            .as_str()
            .ok_or(ParseEntryError::InvalidSource)?;

        match source {
            "last_execute_time" => Ok(Value::Source(
                crate::config::source::Source::LastExecuteDate,
            )),
            "execute_time" => Ok(Value::Source(crate::config::source::Source::ExecuteDate)),
            _ => Err(ParseEntryError::InvalidSourceValue(String::from(source))),
        }
    }

    fn parse_entry_by_type(
        entry_type: &str,
        entry: &serde_yml::Mapping,
    ) -> Result<Self, ParseEntryError> {
        match entry_type {
            "object" => Self::get_object(entry),
            "array" => Self::get_array(entry),
            "source" => Self::get_source(entry),
            "integer" => Self::get_integer(entry),
            "float" => Self::get_float(entry),
            "string" => Self::get_string(entry),
            "boolean" => Self::get_bool(entry),
            "null" => Ok(Value::Null),
            _ => Err(ParseEntryError::InvalidTypeValue(String::from(entry_type))),
        }
    }

    fn parse_basic_entry_by_type(
        entry_type: &str,
        entry: &serde_yml::Mapping,
    ) -> Result<Self, ParseEntryError> {
        match entry_type {
            "source" => Self::get_source(entry),
            "integer" => Self::get_integer(entry),
            "float" => Self::get_float(entry),
            "string" => Self::get_string(entry),
            _ => Err(ParseEntryError::InvalidTypeValue(String::from(entry_type))),
        }
    }
}
