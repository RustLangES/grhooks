#![allow(clippy::missing_errors_doc)]

use serde_json::Value;
use srtemplate::SrTemplate;

mod cmd;

pub use cmd::execute_command;

pub fn render_secret(secret: &str, event_type: &str) -> String {
    let ctx = SrTemplate::with_delimiter("${{", "}}");
    ctx.add_variable("event.type", event_type);
    let secret = ctx.render(secret).unwrap_or_else(|_| secret.to_string());
    tracing::debug!("Rendering secret: {secret}");
    secret
}

pub fn process_value<'a>(ctx: &SrTemplate<'a>, prefix: &'a str, value: &'a Value) {
    match value {
        Value::Null => {
            tracing::trace!("Processing value: {prefix} = {value}");
            ctx.add_variable(prefix, "null");
        }
        Value::Bool(b) => {
            tracing::trace!("Processing value: {prefix} = {value}");
            ctx.add_variable(prefix, b);
        }
        Value::Number(n) => {
            tracing::trace!("Processing value: {prefix} = {value}");
            ctx.add_variable(prefix, n);
        }
        Value::String(s) => {
            tracing::trace!("Processing value: {prefix} = {value}");
            ctx.add_variable(prefix, s);
        }
        Value::Array(arr) => {
            tracing::trace!("Processing value: {prefix} = Array {{}}");
            for (i, item) in arr.iter().enumerate() {
                let key = format!("{prefix}[{i}]");
                let key = unsafe {
                    core::mem::transmute::<&str, &'a str>(Box::leak(key.into_boxed_str()))
                };
                process_value(ctx, key, item);
            }
        }
        Value::Object(obj) => {
            tracing::trace!("Processing value: {prefix} = Object {{}}");
            for (k, v) in obj {
                let key = if prefix.is_empty() {
                    k.to_string()
                } else {
                    format!("{prefix}.{k}")
                };
                let key = unsafe {
                    core::mem::transmute::<&str, &'a str>(Box::leak(key.into_boxed_str()))
                };
                process_value(ctx, key, v);
            }
        }
    }
}
