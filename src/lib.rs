use std::{collections::HashSet, fmt};

use schemars::{
    schema::{
        ArrayValidation, InstanceType, ObjectValidation, RootSchema, Schema, SchemaObject,
        SingleOrVec,
    },
    Map
};
use serde_json::Value;
use  indexmap::map::IndexMap;

type Result<T> = std::result::Result<T, Error>;

#[derive(Default)]
pub struct ExternalTypeCollector {
    parsed_types: IndexMap<String, String>,
    working_on: HashSet<String>,
    types_to_parse: Map<String, Schema>,
}

impl ExternalTypeCollector {
    pub fn new() -> Self {
        Default::default()
    }
    fn gen_type_and_insert(&mut self, reference: String, type_rep: &Schema) -> Result<String> {
        match type_rep {
            Schema::Bool(_) => Ok(reference),
            Schema::Object(x) => {
                let genned_type = gen_from_schema(x, &reference, self)?;
                self.parsed_types.insert(reference.clone(), genned_type);
                Ok(reference)
            }
        }
    }
    pub fn get_type(&mut self, reference: &str) -> Result<String> {
        let reference = remove_start_from_ref(reference);
        if self.working_on.contains(reference) {
            Ok(reference.to_owned())
        } else {
            let x = self
                .types_to_parse
                .get(reference)
                .ok_or(Error::ExternalTypeNotAvailable)?
                .clone();
            let reference = reference.to_owned();
            self.working_on.insert(reference.to_owned());
            let res = self.gen_type_and_insert(reference.to_owned(), &x);
            self.working_on.remove(&reference);
            res
        }
    }
    pub fn add_types_to_parse(&mut self, types: Map<String, Schema>) {
        self.types_to_parse.extend(types)
    }
    pub fn add_unnamed_type(&mut self, prefix: &str, type_rep: &ObjectValidation) -> Result<()> {
        let res = gen_full_object(type_rep, prefix, self)?;
        self.parsed_types.insert(prefix.to_owned(), res);
        Ok(())
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
    EnumHasNoTypes,
    ExternalTypeNotAvailable,
    SimpleEnumNotSimple,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::NoMetaDataForType => write!(f, "The type was expected to have metadata but this was not found"),
            Error::NoNameForType => write!(f, "The type did not have a useable typename, nor could one be generated"),
            Error::NoSubSchemaForType => write!(f, "The type was expected to have subschema's, but none found"),
            Error::NoObjectPartFound => write!(f, "The type was expected to have a object part, but none found"),
            Error::TypeIsNoRealType => write!(f, "The type doesn't have a good definition."),
            Error::NoTypeSet => write!(f, "The type was expected to have the instance_type field, but none found "),
            Error::EnumHasNoTypes => write!(f,"No suitable type found for one of the variants of the enum"),
            Error::ExternalTypeNotAvailable => write!(f,"An external type was referenced, but it was not found"),
            Error::SimpleEnumNotSimple => write!(f,"An enum was expected to not store any values, but it does"),
        }
    }
}
impl std::error::Error for Error {}

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
    let name = get_name(&schema, x)?;
    x.add_types_to_parse(a.definitions);
    gen_from_schema(&schema, &name, x)
}

fn gen_from_schema(a: &SchemaObject, name: &str, x: &mut ExternalTypeCollector) -> Result<String> {
    if should_map_to_enum(&a) {
        gen_enum(&a, x, Some(name), name)
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
fn get_name(a: &SchemaObject, y: &mut ExternalTypeCollector) -> Result<String> {
    a.metadata
        .as_deref()
        .and_then(|v| v.title.as_deref())
        .map(ToOwned::to_owned)
        .ok_or(Error::NoNameForType)
        .or_else(|x| {
            a.instance_type
                .as_ref()
                .map(|v| build_in_types_to_name(v, &a.object, &a.array, y, ""))
                .ok_or(x)
                .and_then(|v| v)
        })
}
//looks if the json conains an "anyof"
fn should_map_to_enum(a: &SchemaObject) -> bool {
    a.object.is_none()
}
fn gen_enum(
    a: &SchemaObject,
    x: &mut ExternalTypeCollector,
    name_overwrite: Option<&str>,
    type_prefix: &str,
) -> Result<String> {
    let header = name_overwrite
        .map(ToOwned::to_owned)
        .map(Ok)
        .or_else(|| {
            a.instance_type
                .as_ref()
                .map(|z| build_in_types_to_name(z, &a.object, &a.array, x, type_prefix))
        })
        .ok_or(Error::NoTypeSet)?
        .map(|res| gen_simple_enum_header(&res))?;
    a.subschemas
        .as_deref()
        .and_then(|v| v.any_of.as_ref())
        .map(|v| {
            v.iter()
                .map(|a| match a {
                    Schema::Bool(_) => {
                        panic!()
                    }
                    Schema::Object(z) => z
                        .object
                        .as_ref()
                        .map(|y| {
                            let (prop_name, schema) =
                                y.properties.iter().next().expect("expected one property");
                            let type_name = get_type_from_schema(
                                schema,
                                x,
                                &format!("{}{}", type_prefix, prop_name),
                            )?;
                            Ok(format!("    | {} of {}\n", prop_name, type_name))
                        })
                        .or_else(|| {
                            z.enum_values
                                .as_ref()
                                .map(|v| gen_simple_enum_body(v).map(|v| format!("{}\n", v)))
                        })
                        .ok_or(Error::NoNameForType)
                        .and_then(|v| v),
                })
                .collect::<Result<String>>()
        })
        .or_else(|| a.enum_values.as_ref().map(|v| gen_simple_enum_body(v)))
        .unwrap_or(Err(Error::EnumHasNoTypes))
        .map(|v| format!("{}\n{}", header, v))
}
fn gen_simple_enum_body(a: &[Value]) -> Result<String> {
    a.iter()
        .map(|v| serde_json::from_value::<String>(v.clone()))
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| Error::SimpleEnumNotSimple)
        .map(|v| format!("    | {}", v.join("\n    | ")))
}

fn get_type_from_schema(
    a: &Schema,
    d: &mut ExternalTypeCollector,
    type_prefix: &str,
) -> Result<String> {
    match a {
        Schema::Bool(_) => Err(Error::TypeIsNoRealType),
        Schema::Object(x) => x
            .instance_type
            .as_ref()
            .map(|v| build_in_types_to_name(v, &x.object, &x.array, d, type_prefix))
            .or_else(|| {
                let x = x.reference.as_deref().map(|v| d.get_type(v));
                x
            })
            .or_else(|| {
                x.subschemas.as_deref().and_then(|v| {
                    v.any_of
                        .as_ref()
                        .map(|v| convert_any_to_known_type(&v, d, type_prefix))
                })
            })
            .unwrap_or(Err(Error::NoTypeSet)),
    }
}

fn convert_any_to_known_type(
    v: &[Schema],
    x: &mut ExternalTypeCollector,
    type_prefix: &str,
) -> Result<String> {
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
            return get_type_from_schema(without_null[0], x, type_prefix)
                .map(|v| make_type_optional(&v));
        } else {
            return v
                .iter()
                .map(|v| get_type_from_schema(v, x, type_prefix))
                .collect::<Result<Vec<_>>>()
                .map(|v| format!("result<{}>", v.join(",")));
        }
    }
    Err(Error::NoNameForType)
}

fn build_in_types_to_name(
    a: &SingleOrVec<InstanceType>,
    v: &Option<Box<ObjectValidation>>,
    y: &Option<Box<ArrayValidation>>,
    x: &mut ExternalTypeCollector,
    type_prefix: &str,
) -> Result<String> {
    match a {
        SingleOrVec::Single(a) => singular_build_in_type_to_name(a, v, y, x, type_prefix),
        SingleOrVec::Vec(a) => build_in_types_from_multiple(a, v, y, x, type_prefix),
    }
}

fn make_type_optional(a: &str) -> String {
    format!("option<{}>", a)
}

fn build_in_types_from_multiple(
    a: &[InstanceType],
    v: &Option<Box<ObjectValidation>>,
    y: &Option<Box<ArrayValidation>>,
    x: &mut ExternalTypeCollector,
    type_prefix: &str,
) -> Result<String> {
    if a.len() == 2 {
        let without_null: Vec<_> = a.iter().filter(|v| v != &&InstanceType::Null).collect();
        if without_null.len() == 1 {
            return singular_build_in_type_to_name(without_null[0], v, y, x, type_prefix)
                .map(|v| make_type_optional(&v));
        }
    }
    a.iter()
        .map(|a| singular_build_in_type_to_name(a, v, y, x, type_prefix))
        .collect()
}

fn singular_build_in_type_to_name(
    a: &InstanceType,
    v: &Option<Box<ObjectValidation>>,
    y: &Option<Box<ArrayValidation>>,
    x: &mut ExternalTypeCollector,
    type_prefix: &str,
) -> Result<String> {
    Ok(match a {
        InstanceType::Null => "unit".to_string(),
        InstanceType::Boolean => "bool".to_string(),
        InstanceType::Object => v
            .as_ref()
            .map(|v| {
                x.add_unnamed_type(type_prefix, v)?;
                Ok(type_prefix.to_owned())
            })
            .unwrap_or_else(|| Ok("object".to_string()))?,
        InstanceType::Array => y
            .as_ref()
            .and_then(|v| v.items.as_ref())
            .map(|v| match v {
                SingleOrVec::Single(v) => {
                    get_type_from_schema(v.as_ref(), x, type_prefix).map(|v| format!("{}[]", v))
                }
                SingleOrVec::Vec(v) => v
                    .iter()
                    .map(|v| get_type_from_schema(v, x, type_prefix))
                    .collect::<Result<Vec<_>>>()
                    .map(|v| v.join(" * ")),
            })
            .unwrap_or_else(|| Ok(String::from("object[]")))?,
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
    let body = get_object_body(a, x, type_name)?;
    Ok(format!(
        "{}    {{\n{}\n    }}",
        gen_object_header(type_name),
        body
    ))
}

fn gen_simple_enum_header(type_name: &str) -> String {
    format!("type {} = \n", type_name)
}

fn gen_object_header(type_name: &str) -> String {
    gen_simple_enum_header(type_name)
}

fn get_object_body(
    a: &ObjectValidation,
    x: &mut ExternalTypeCollector,
    type_prefix: &str,
) -> Result<String> {
    //panic!("should we get here? {:?}", a);
    Ok(get_object_parts(a, x, type_prefix)?
        .into_iter()
        .map(|(key_name, of_type)| format!("        {} : {}", key_name, of_type))
        .collect::<Vec<_>>()
        .join("\n"))
}
fn get_object_parts(
    a: &ObjectValidation,
    x: &mut ExternalTypeCollector,
    type_prefix: &str,
) -> Result<Vec<(String, String)>> {
    a.properties
        .iter()
        .map(|(key, value)| {
            get_type_from_schema(&value, x, type_prefix).map(|v| (key.to_owned(), v))
        })
        .collect::<Result<Vec<(String, String)>>>()
}
