use schemars::JsonSchema;
use type_gen::{gen_from_type, ExternalTypeCollector};

#[derive(JsonSchema)]
#[allow(dead_code)]
struct TestType {
    a_string: String,
    a_number: i64,
    optional_float: Option<f32>,
    optional_external: Option<ExternalType>,
    external_simple_enum: Option<TestEnum>,
    a_simple_array: Vec<f32>,
    an_array_of_options: Vec<Option<String>>,
    an_external_array: Vec<ExternalType>,
    an_optional_array: Option<Vec<SimpleEnum>>,
}
#[derive(JsonSchema)]
#[allow(dead_code)]
struct ExternalType {
    test: String,
}

#[derive(JsonSchema)]
#[allow(dead_code)]
enum TestEnum {
    A,
    D,
    B(f32, i64),
    C { test: f32, test2: String },
    E(SimpleEnum),
}
#[derive(JsonSchema)]
#[allow(dead_code)]
enum SimpleEnum {
    A,
    B,
    C,
}

#[derive(JsonSchema)]
#[allow(dead_code)]
enum SimpleRecursiveEnum {
    Rec(Box<SimpleRecursiveEnum>),
    Nope(f64),
}

fn main() {
    println!(
        "{}",
        serde_json::to_string_pretty(&schemars::schema_for!(SimpleRecursiveEnum)).unwrap()
    );
    let mut external_types = ExternalTypeCollector::new();
    let x = gen_from_type::<SimpleRecursiveEnum>(&mut external_types);
    external_types
        .get_external_types()
        .for_each(|(_, v)| println!("{}", v));
    match x {
        Ok(x) => println!("{}\n", x),
        Err(x) => println!("ERROR!!!\n{:?}", x),
    }
}
