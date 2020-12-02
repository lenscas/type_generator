use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{fs::OpenOptions, io::Write, process::Command};
use type_gen::{gen_from_type, ExternalTypeCollector};

#[derive(JsonSchema, Deserialize, Serialize, PartialEq)]
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
    recursive_type: SimpleRecursiveEnum,
}
#[derive(JsonSchema, Deserialize, Serialize, PartialEq)]
#[allow(dead_code)]
struct ExternalType {
    test: String,
}

#[derive(JsonSchema, Deserialize, Serialize, PartialEq)]
#[allow(dead_code)]
enum TestEnum {
    A,
    D,
    B(f32, i64),
    C { test: f32, test2: String },
    E(SimpleEnum),
}
#[derive(JsonSchema, Deserialize, Serialize, PartialEq)]
#[allow(dead_code)]
enum SimpleEnum {
    A,
    B,
    C,
}

#[derive(JsonSchema, Deserialize, Serialize, PartialEq)]
#[allow(dead_code)]
enum SimpleRecursiveEnum {
    Rec(Box<SimpleRecursiveEnum>),
    Nope(f64),
}

#[test]
fn test_gen() {
    let mut external_types = ExternalTypeCollector::new();
    let generated_type = gen_from_type::<TestType>(&mut external_types)
        .unwrap()
        .into_option()
        .unwrap()
        .to_owned();
    let data = TestType {
        a_string: "this is a string".into(),
        a_number: 2,
        optional_float: Some(2.0),
        optional_external: Some(ExternalType {
            test: "nice!".into(),
        }),
        external_simple_enum: Some(TestEnum::C {
            test: 9.1,
            test2: "awesome".into(),
        }),
        a_simple_array: vec![1.0, 1.2, 2.2, 3.3],
        an_array_of_options: vec![Some("Nice".into()), Some("awesome".into())],
        an_external_array: vec![
            ExternalType {
                test: "great".into(),
            },
            ExternalType {
                test: "second".into(),
            },
        ],
        recursive_type: SimpleRecursiveEnum::Rec(Box::new(SimpleRecursiveEnum::Nope(20.1))),
    };
    let json = serde_json::to_string(&serde_json::to_string(&data).expect("could not serialize"))
        .expect("very ugly hack to escape everything did not work :(");
    let external_types_string = external_types
        .get_new_external_types()
        .map(|v| v.1)
        .collect::<Vec<_>>()
        .join("\n");

    let fsharp_program = format!(
        "
open System
open FSharp.Json
{}
{}
[<EntryPoint>]
let main argv =
    let type_as_json = {}
    
    type_as_json
    |> Json.deserialize<{}>
    |> Json.serialize
    |> printfn \"%s\"

    0
",
        external_types_string, generated_type, json, "TestType"
    );

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("./test_parsing/Program.fs")
        .expect("could not open file");
    file.write_all(fsharp_program.as_bytes())
        .expect("Could not write program");
    let x = Command::new("dotnet")
        .arg("run")
        .current_dir("./test_parsing")
        .output()
        .expect("could not run command");

    let output = match serde_json::from_slice::<TestType>(&x.stdout) {
        Ok(x) => x,
        Err(y) => {
            println!(
                "Could not deserialize from output. Got:\n {}\n\n",
                String::from_utf8(x.stdout).expect("could not turn bytes of stdout into string")
            );
            println!(
                "stderr: \n{}\n\n",
                String::from_utf8(x.stderr).expect("could not turn bytes of stderr into string")
            );
            println!("got error:{}", y);
            panic!()
        }
    };
    assert!(output == data);
}
