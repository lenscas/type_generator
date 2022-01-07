use schemars::JsonSchema;
use type_gen::{gen_from_type, ExternalTypeCollector};

#[derive(JsonSchema)]
#[allow(dead_code)]
struct RecursiveStruct {
    next: Option<Box<RecursiveStruct>>,
    recursive_enum: Recursive,
}

#[derive(JsonSchema)]
#[allow(dead_code)]
enum Recursive {
    End,
    Next(Box<Recursive>),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut external_types = ExternalTypeCollector::new();
    let x = gen_from_type::<RecursiveStruct>(&mut external_types)
        .map(|x| x.into_option().map(|v| v.to_owned()));

    external_types
        .get_new_external_types()
        .for_each(|(_, v)| println!("{}", v));
    match x {
        Ok(x) => {
            if let Some(x) = x {
                println!("{}", x)
            }
        }
        Err(x) => println!("ERROR!!!\n{:?}", x),
    }
    Ok(())
}
