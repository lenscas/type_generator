use schemars::JsonSchema;
use type_gen::{gen_from_type, ExternalTypeCollector};

#[derive(JsonSchema)]
#[allow(dead_code)]
struct TestType {
    a_string: String,
    a_number: i64,
    optional_float: Option<f32>,
    optional_external: Option<ExternalType>,
    external_simple_enum: Option<SimpleEnum>,
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
    B(f32),
    C { test: f32 },
}
#[derive(JsonSchema)]
#[allow(dead_code)]
enum SimpleEnum {
    A,
    B,
    C,
}

fn main() {
    println!(
        "{}",
        serde_json::to_string_pretty(&schemars::schema_for!(TestType)).unwrap()
    );
    let mut external_types = ExternalTypeCollector::new();
    let x = gen_from_type::<TestType>(&mut external_types);

    match x {
        Ok(x) => println!("{}", x),
        Err(x) => println!("ERROR!!!\n{:?}", x),
    }

    external_types
        .get_external_types()
        .for_each(|(_, v)| println!("{}", v));
}
