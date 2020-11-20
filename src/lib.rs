use std::collections::HashMap;

use schemars::{
    schema::{InstanceType, ObjectValidation, RootSchema, Schema, SchemaObject, SingleOrVec},
    Map,
};

type Result<T> = std::result::Result<T, Error>;

#[derive(Default)]
pub struct ExternalTypeCollector {
    parsed_types: HashMap<String, String>,
    types_to_parse: Map<String, Schema>,
}

impl ExternalTypeCollector {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn get_type(&mut self, reference: &str) -> Result<String> {
        let reference = remove_start_from_ref(reference);
        dbg!(&self.types_to_parse);

        let x = self
            .types_to_parse
            .get(reference)
            .ok_or(Error::ExternalTypeNotAvailable)?
            .clone();
        let reference = reference.to_owned();
        match x {
            Schema::Bool(_) => Ok(reference),
            Schema::Object(x) => {
                let genned_type = gen_from_schema(&x, &reference, self)?;
                self.parsed_types.insert(reference.clone(), genned_type);
                get_name(&x).map(ToOwned::to_owned).or(Ok(reference))
            }
        }
    }
    pub fn add_types_to_parse(&mut self, types: Map<String, Schema>) {
        self.types_to_parse.extend(types)
    }
    pub fn get_external_types(&self) -> impl Iterator<Item = (&String, &String)> {
        self.parsed_types.iter()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Error {
    NoMetaDataForType,
    NoNameForType,
    NoSubSchemaForType,
    NoObjectPartFound,
    TypeIsNoRealType,
    NoTypeSet,
    TypeNotInDefs,
    EnumHasNoTypes,
    ExternalTypeNotAvailable,
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

pub fn gen_from_type<A: schemars::JsonSchema>(x: &mut ExternalTypeCollector) -> Result<String> {
    gen(schemars::schema_for!(A), x)
}

pub fn gen(a: RootSchema, x: &mut ExternalTypeCollector) -> Result<String> {
    let schema = a.schema;
    let name = get_name(&schema)?;
    x.add_types_to_parse(a.definitions);
    gen_from_schema(&schema, name, x)
}

fn gen_from_schema(a: &SchemaObject, name: &str, x: &mut ExternalTypeCollector) -> Result<String> {
    if should_map_to_enum(&a) {
        gen_enum(&a, x)
    } else {
        gen_object_from_schema_object(&a, name, x)
    }
}

fn gen_object_from_schema_object(
    a: &SchemaObject,
    name: &str,
    x: &mut ExternalTypeCollector,
) -> Result<String> {
    let res = a.object.as_deref().ok_or(Error::NoObjectPartFound)?;
    gen_full_object(res, name, x)
}

///tries to get the name of a type.
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
fn gen_enum(a: &SchemaObject, x: &mut ExternalTypeCollector) -> Result<String> {
    a.subschemas
        .as_deref()
        .and_then(|v| v.any_of.as_ref())
        .ok_or(Error::EnumHasNoTypes)
        .and_then(|v| {
            v.iter()
                .map(|a| get_type_from_schema(a, x))
                .collect::<Result<Vec<_>>>()
                .map(|v| v.join("\n"))
        })
}

fn get_type_from_schema(a: &Schema, d: &mut ExternalTypeCollector) -> Result<String> {
    match a {
        Schema::Bool(_) => Err(Error::TypeIsNoRealType),
        Schema::Object(x) => x
            .instance_type
            .as_ref()
            .map(|v| build_in_types_to_name(v, &x.object, d))
            .or_else(|| {
                let x = x.reference.as_deref().map(|v| d.get_type(v));
                x
            })
            .or_else(|| {
                x.subschemas
                    .as_deref()
                    .and_then(|v| v.any_of.as_ref().map(|v| convert_any_to_known_type(&v, d)))
            })
            .ok_or(Error::NoTypeSet)
            .unwrap(),
    }
}

fn convert_any_to_known_type(v: &[Schema], x: &mut ExternalTypeCollector) -> Result<String> {
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
            return get_type_from_schema(without_null[0], x).map(|v| make_type_optional(&v));
        } else {
            return v
                .iter()
                .map(|v| get_type_from_schema(v, x))
                .collect::<Result<Vec<_>>>()
                .map(|v| format!("result<{}>", v.join(",")));
        }
    }
    Err(Error::NoNameForType)
}

fn build_in_types_to_name(
    a: &SingleOrVec<InstanceType>,
    v: &Option<Box<ObjectValidation>>,
    x: &mut ExternalTypeCollector,
) -> Result<String> {
    match a {
        SingleOrVec::Single(a) => singular_build_in_type_to_name(a, v, x),
        SingleOrVec::Vec(a) => build_in_types_from_multiple(a, v, x),
    }
}

fn make_type_optional(a: &str) -> String {
    format!("option<{}>", a)
}

fn build_in_types_from_multiple(
    a: &[InstanceType],
    v: &Option<Box<ObjectValidation>>,
    x: &mut ExternalTypeCollector,
) -> Result<String> {
    if a.len() == 2 {
        let without_null: Vec<_> = a.iter().filter(|v| v != &&InstanceType::Null).collect();
        if without_null.len() == 1 {
            return singular_build_in_type_to_name(without_null[0], v, x)
                .map(|v| make_type_optional(&v));
        }
    }
    a.iter()
        .map(|a| singular_build_in_type_to_name(a, v, x))
        .collect()
}

fn singular_build_in_type_to_name(
    a: &InstanceType,
    v: &Option<Box<ObjectValidation>>,
    x: &mut ExternalTypeCollector,
) -> Result<String> {
    Ok(match a {
        InstanceType::Null => "unit".to_string(),
        InstanceType::Boolean => "bool".to_string(),
        InstanceType::Object => v
            .as_ref()
            .map(|v| get_object_body(v, x))
            .unwrap_or_else(|| Ok("object".to_string()))?,
        InstanceType::Array => "object[]".to_string(),
        InstanceType::Number => "float".to_string(),
        InstanceType::String => "string".to_string(),
        InstanceType::Integer => "int".to_string(),
    })
}

fn gen_full_object(
    a: &ObjectValidation,
    type_name: &str,
    x: &mut ExternalTypeCollector,
) -> Result<String> {
    let body = get_object_body(a, x)?;
    Ok(format!("type {} =\n\tstruct\n{}\n\tend", type_name, body))
}

fn get_object_body(a: &ObjectValidation, x: &mut ExternalTypeCollector) -> Result<String> {
    Ok(get_object_parts(a, x)?
        .into_iter()
        .map(|(key_name, of_type)| format!("\t\tval {} : {}", key_name, of_type))
        .collect::<Vec<_>>()
        .join("\n"))
}
fn get_object_parts(
    a: &ObjectValidation,
    x: &mut ExternalTypeCollector,
) -> Result<Vec<(String, String)>> {
    a.properties
        .iter()
        .map(|(key, value)| get_type_from_schema(&value, x).map(|v| (key.to_owned(), v)))
        .collect::<Result<Vec<(String, String)>>>()
}
