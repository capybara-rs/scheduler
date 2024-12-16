use std::collections::BTreeMap;
use std::fmt;

use serde_yml::{self, Mapping, Value};

#[derive(Debug, PartialEq)]
pub enum Error {
    EnvVarNotFound { env_name: String },
    InvalidEnvSyntax,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::EnvVarNotFound { env_name } => write!(f, "env {env_name} not found"),
            Error::InvalidEnvSyntax => write!(f, "invalid env!() syntax"),
        }
    }
}

type Result<T> = std::result::Result<T, Error>;

pub fn recursive_replace_env(value: Value) -> Result<Value> {
    let replacer = EnvReplacer {
        envs: std::env::vars().collect(),
    };

    replacer.replace_value(value)
}

struct EnvReplacer {
    envs: BTreeMap<String, String>,
}

impl EnvReplacer {
    fn replace_value(&self, value: Value) -> Result<Value> {
        match value {
            serde_yml::Value::Null => Ok(value),
            serde_yml::Value::Bool(_) => Ok(value),
            serde_yml::Value::Number(_) => Ok(value),
            serde_yml::Value::String(s) => self.replace_string(s),
            serde_yml::Value::Sequence(vec) => self.replace_sequence(vec),
            serde_yml::Value::Mapping(map) => self.replace_mapping(map),
            serde_yml::Value::Tagged(_) => Ok(value),
        }
    }

    fn replace_string(&self, s: String) -> Result<Value> {
        let res = Self::find_env(s.as_str())?;

        match res {
            Some(env_name) => {
                let old = format!("env!({})", env_name);

                let new = self
                    .envs
                    .get(&env_name)
                    .ok_or(Error::EnvVarNotFound { env_name })?;

                let value = s.replace(old.as_str(), new);

                self.replace_string(value)
            }
            None => return Ok(Value::String(s)),
        }
    }

    fn replace_sequence(&self, vec: Vec<Value>) -> Result<Value> {
        let mut new_vec = Vec::with_capacity(vec.len());

        for value in vec {
            let new_value = self.replace_value(value)?;

            new_vec.push(new_value);
        }

        Ok(Value::Sequence(new_vec))
    }

    fn replace_mapping(&self, map: Mapping) -> Result<Value> {
        let mut new_map = Mapping::with_capacity(map.len());

        for (key, value) in map.into_iter() {
            let new_value = self.replace_value(value)?;

            new_map.insert(key, new_value);
        }

        Ok(Value::Mapping(new_map))
    }

    fn find_env(s: &str) -> Result<Option<String>> {
        const ENV_SYMBOL: &str = "env!(";

        let mut s = s.to_string();

        let index = match s.find(ENV_SYMBOL) {
            Some(index) => index,
            None => return Ok(None),
        };

        let s = s.split_off(index + ENV_SYMBOL.len());

        let mut env_name = String::new();

        for ch in s.chars() {
            if ch == ')' {
                return Ok(Some(env_name));
            }

            env_name.push(ch);
        }

        Err(Error::InvalidEnvSyntax)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::EnvReplacer;

    #[test]
    fn test_env_replacer() {
        let envs = BTreeMap::from_iter([
            (String::from("TOKEN"), String::from("example_token")),
            (String::from("URL"), String::from("http://localhost:3030")),
        ]);

        let replacer = EnvReplacer { envs };

        env_replacer_success(&replacer, "test: env!(TOKEN)\n", "test: example_token\n");
        env_replacer_success(
            &replacer,
            "test: env!(URL)/load\n",
            "test: http://localhost:3030/load\n",
        );
        env_replacer_success(
            &replacer,
            r#"test: ["env!(TOKEN)", "env!(URL)/load"]"#,
            "test:\n- example_token\n- http://localhost:3030/load\n",
        );
        env_replacer_success(
            &replacer,
            r#"test: ["env!(URL)/env!(TOKEN)/", "env!(URL)/load"]"#,
            "test:\n- http://localhost:3030/example_token/\n- http://localhost:3030/load\n",
        );
        env_replacer_failure(
            &replacer,
            "test: env!(RANDOM_ENV)",
            super::Error::EnvVarNotFound {
                env_name: String::from("RANDOM_ENV"),
            },
        );
        env_replacer_failure(
            &replacer,
            "test: env!(RANDOM_ENV",
            super::Error::InvalidEnvSyntax,
        );
    }

    fn env_replacer_success(replacer: &EnvReplacer, input: &str, expected_output: &str) {
        let value = serde_yml::from_str(input).unwrap();

        let value = replacer
            .replace_value(value)
            .unwrap();

        let output = serde_yml::to_string(&value).unwrap();

        assert_eq!(expected_output, output.as_str());
    }

    fn env_replacer_failure(replacer: &EnvReplacer, input: &str, expected_err: super::Error) {
        let value = serde_yml::from_str(input).unwrap();

        let err = replacer
            .replace_value(value)
            .err()
            .unwrap();

        assert_eq!(err, expected_err)
    }
}
