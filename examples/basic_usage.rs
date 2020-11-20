use schemars::JsonSchema;
use type_gen::{gen_from_type, ExternalTypeCollector};

#[derive(JsonSchema)]
#[allow(dead_code)]
struct TestType {
    a_string: String,
    a_number: i64,
    optional_float: Option<f32>,
    optional_external: Option<ExternalType>,
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

fn main() {
    println!(
        "{}",
        serde_json::to_string_pretty(&schemars::schema_for!(TestEnum)).unwrap()
    );
    let mut external_types = ExternalTypeCollector::new();
    let x = gen_from_type::<TestEnum>(&mut external_types);

    match x {
        Ok(x) => println!("{}", x),
        Err(x) => println!("ERROR!!!\n{:?}", x),
    }

    external_types
        .get_external_types()
        .for_each(|(_, v)| println!("{}", v));
}
