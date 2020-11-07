use schemars::schema::{InstanceType, RootSchema, Schema, SchemaObject, SingleOrVec};

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Error {
    NoMetaDataForType,
    NoNameForType,
    NoSubSchemaForType,
    NoObjectPartFound,
    TypeIsNoRealType,
    NoTypeSet,
    TypeNotInDefs,
}

fn remove_start_from_ref(s: &str) -> &str {
    let mut to_remove_from_start = "#/definitions/".chars();
    let mut wrong_start = false;
    s.char_indices()
        .find(|(_, chara)| {
            to_remove_from_start
                .next()
                .map(|v| v == *chara)
                .map(|v| {
                    wrong_start = !v;
                    wrong_start
                })
                .unwrap_or(true)
        })
        .map(|v| v.0)
        .and_then(|pos| if wrong_start { None } else { Some(&s[pos..]) })
        .unwrap_or(s)
}

pub fn gen_from_type<A: schemars::JsonSchema>() -> Result<String> {
    gen(schemars::schema_for!(A))
}

pub fn gen(a: RootSchema) -> Result<String> {
    let schema = a.schema;
    let name = get_name(&schema)?;
    let res = if should_map_to_enum(&schema) {
        gen_enum(&schema)
    } else {
        gen_object(&schema, name)
    }?;

    Ok(res)
}

//tries to get the name of a type.
fn get_name(a: &SchemaObject) -> Result<&str> {
    a.metadata
        .as_deref()
        .ok_or(Error::NoMetaDataForType)?
        .title
        .as_deref()
        .ok_or(Error::NoNameForType)
}
//looks if the json conains an "anyof"
fn should_map_to_enum(a: &SchemaObject) -> bool {
    a.object.is_none()
}
fn gen_enum(a: &SchemaObject) -> Result<String> {
    Ok("AWESOME".into())
}

fn gen_object(a: &SchemaObject, name: &str) -> Result<String> {
    let res = a.object.as_deref().ok_or(Error::NoObjectPartFound)?;
    let parts = res
        .properties
        .iter()
        .map(|(key, value)| get_type_from_schema(&value).map(|v| (key.to_owned(), v)))
        .collect::<Result<Vec<(String, String)>>>()?;
    Ok(combine_parts(parts, name))
}

fn combine_parts(a: Vec<(String, String)>, type_name: &str) -> String {
    let defs = a
        .into_iter()
        .map(|(key_name, of_type)| format!("\t\tval {} : {}", key_name, of_type))
        .collect::<Vec<_>>()
        .join("\n");
    format!("type {} =\n\tstruct\n{}\n\tend", type_name, defs)
}

fn get_type_from_schema(a: &Schema) -> Result<String> {
    match a {
        Schema::Bool(_) => Err(Error::TypeIsNoRealType),
        Schema::Object(x) => x
            .instance_type
            .as_ref()
            .map(|v| build_in_types_to_name(v))
            .or_else(|| {
                let x = x
                    .reference
                    .as_deref()
                    .map(remove_start_from_ref)
                    .map(String::from)
                    .map(Ok);
                x
            })
            .or_else(|| {
                x.subschemas
                    .as_deref()
                    .and_then(|v| v.any_of.as_ref().map(|v| convert_any_to_known_type(&v)))
            })
            .ok_or(Error::NoTypeSet)
            .unwrap(),
    }
}

fn convert_any_to_known_type(v: &[Schema]) -> Result<String> {
    if v.len() == 2 {
        let without_null: Vec<_> = v
            .iter()
            .filter(|v| match v {
                Schema::Bool(_) => true,
                Schema::Object(x) => x
                    .instance_type
                    .as_ref()
                    .map(|v| match v {
                        SingleOrVec::Single(v) => **v != InstanceType::Null,
                        SingleOrVec::Vec(_) => true,
                    })
                    .unwrap_or(true),
            })
            .collect();
        if without_null.len() == 1 {
            return get_type_from_schema(without_null[0]).map(|v| make_type_optional(&v));
        } else {
            return v
                .iter()
                .map(get_type_from_schema)
                .collect::<Result<Vec<_>>>()
                .map(|v| format!("result<{}>", v.join(",")));
        }
    }
    Err(Error::NoNameForType)
}

fn build_in_types_to_name(a: &SingleOrVec<InstanceType>) -> Result<String> {
    match a {
        SingleOrVec::Single(a) => singular_build_in_type_to_name(a),
        SingleOrVec::Vec(x) => build_in_types_from_multiple(x),
    }
}

fn make_type_optional(a: &str) -> String {
    format!("option<{}>", a)
}

fn build_in_types_from_multiple(a: &[InstanceType]) -> Result<String> {
    if a.len() == 2 {
        let without_null: Vec<_> = a.iter().filter(|v| v != &&InstanceType::Null).collect();
        if without_null.len() == 1 {
            return singular_build_in_type_to_name(without_null[0]).map(|v| make_type_optional(&v));
        }
    }
    a.iter().map(singular_build_in_type_to_name).collect()
}

fn singular_build_in_type_to_name(a: &InstanceType) -> Result<String> {
    Ok(match a {
        InstanceType::Null => "unit",
        InstanceType::Boolean => "bool",
        InstanceType::Object => "object",
        InstanceType::Array => "object[]",
        InstanceType::Number => "float",
        InstanceType::String => "string",
        InstanceType::Integer => "int",
    }
    .to_string())
}
