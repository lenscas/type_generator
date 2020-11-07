use schemars::JsonSchema;
use type_gen::gen_from_type;

#[derive(JsonSchema)]
struct TestType {
    a_string: String,
    a_number: i64,
    optional_float: Option<f32>,
    optional_external: Option<ExternalType>,
}
#[derive(JsonSchema)]
struct ExternalType {
    test: String,
}

fn main() {
    println!(
        "{}",
        serde_json::to_string_pretty(&schemars::schema_for!(TestType)).unwrap()
    );
    let x = gen_from_type::<TestType>();
    match x {
        Ok(x) => println!("{}", x),
        Err(x) => println!("ERROR!!!\n{:?}", x),
    }
}
