use serde_json::Value;
pub fn canonical(v: &Value) -> Value {
    match v {
        Value::Array(a) => {
            let mut a: Vec<_> = a.iter().map(canonical).collect();
            a.sort_by_key(Value::to_string);
            Value::Array(a)
        }
        Value::Object(m) => {
            Value::Object(m.iter().map(|(k, v)| (k.clone(), canonical(v))).collect())
        }
        _ => v.clone(),
    }
}
